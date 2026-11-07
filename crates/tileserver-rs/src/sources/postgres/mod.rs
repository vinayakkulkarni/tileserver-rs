//! PostgreSQL sources for serving vector tiles from PostGIS.
//!
//! This module provides two types of PostgreSQL sources:
//!
//! - **Function sources**: Execute SQL functions that return MVT data
//! - **Table sources**: Generate optimized SQL queries with spatial filtering
//!
//! # Function Source
//!
//! ```sql
//! CREATE FUNCTION my_tiles(z integer, x integer, y integer)
//!     RETURNS bytea AS $$ ... $$ LANGUAGE plpgsql;
//! ```
//!
//! # Table Source
//!
//! Automatically discovers geometry columns and generates efficient tile queries
//! using `ST_TileEnvelope` for spatial filtering that utilizes spatial indexes.

mod cache;
#[cfg(feature = "raster")]
mod outdb;
mod pool;
mod source;
mod table;

pub use cache::{TileCache, TileCacheKey};
#[cfg(feature = "raster")]
pub use outdb::PostgresOutDbRasterSource;
pub use pool::{PoolSettings, PostgresPool};
pub use source::PostgresFunctionSource;
pub use table::{PostgresTableSource, TableInfo, is_point_geometry, pg_type_to_json_schema};

use semver::Version;

use crate::error::{Result, TileServerError};
use crate::sources::{TileCompression, TileData, TileFormat, TileMetadata};

/// `Ok(false)` outside the source's zoom range; an error for coordinates
/// outside the tile grid.
fn serves_tile(metadata: &TileMetadata, z: u8, x: u32, y: u32) -> Result<bool> {
    let max_tile = 1u32 << z;
    if x >= max_tile || y >= max_tile {
        return Err(TileServerError::InvalidCoordinates { z, x, y });
    }
    Ok((metadata.minzoom..=metadata.maxzoom).contains(&z))
}

/// An MVT tile from PostGIS bytes, gzip when they start with the gzip magic
/// number.
fn pbf_tile(data: Vec<u8>) -> TileData {
    let compression = if data.starts_with(&[0x1f, 0x8b]) {
        TileCompression::Gzip
    } else {
        TileCompression::None
    };
    TileData {
        data: bytes::Bytes::from(data),
        format: TileFormat::Pbf,
        compression,
    }
}

/// Serve from `cache` when it holds the tile; otherwise run `fetch` and
/// cache a hit.
async fn through_cache(
    cache: Option<&TileCache>,
    source_id: &str,
    (z, x, y): (u8, u32, u32),
    fetch: impl std::future::Future<Output = Result<Option<TileData>>>,
) -> Result<Option<TileData>> {
    let Some(cache) = cache else {
        return fetch.await;
    };
    let key = TileCacheKey {
        source_id: source_id.into(),
        z,
        x,
        y,
        encoding: TileCompression::None,
        filter: 0,
    };
    if let Some(tile) = cache.get(&key).await {
        return Ok(Some(tile));
    }
    let tile = fetch.await?;
    if let Some(tile) = &tile {
        cache.insert(key, tile.clone()).await;
    }
    Ok(tile)
}

/// Minimum PostgreSQL version required (11.0.0)
pub const MINIMUM_POSTGRES_VERSION: Version = Version::new(11, 0, 0);

/// Minimum PostGIS version required (3.0.0) for ST_TileEnvelope support
pub const MINIMUM_POSTGIS_VERSION: Version = Version::new(3, 0, 0);

/// PostGIS version that supports margin parameter in ST_TileEnvelope (3.1.0)
pub const ST_TILE_ENVELOPE_MARGIN_VERSION: Version = Version::new(3, 1, 0);

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata(minzoom: u8, maxzoom: u8) -> TileMetadata {
        TileMetadata {
            id: "t".into(),
            name: "t".into(),
            description: None,
            attribution: None,
            format: TileFormat::Pbf,
            minzoom,
            maxzoom,
            bounds: None,
            center: None,
            vector_layers: None,
        }
    }

    #[test]
    fn serves_tile_checks_grid_then_zoom_range() {
        let meta = metadata(2, 4);
        assert!(serves_tile(&meta, 3, 7, 7).unwrap());
        assert!(!serves_tile(&meta, 1, 0, 0).unwrap());
        assert!(!serves_tile(&meta, 5, 0, 0).unwrap());
        assert!(matches!(
            serves_tile(&meta, 3, 8, 0),
            Err(TileServerError::InvalidCoordinates { z: 3, x: 8, y: 0 })
        ));
    }

    #[test]
    fn pbf_tile_detects_gzip() {
        assert_eq!(
            pbf_tile(vec![0x1f, 0x8b, 0]).compression,
            TileCompression::Gzip
        );
        assert_eq!(pbf_tile(vec![0x1f]).compression, TileCompression::None);
        assert_eq!(pbf_tile(vec![1, 2]).format, TileFormat::Pbf);
    }

    async fn fetch(tile: Option<Vec<u8>>) -> Result<Option<TileData>> {
        Ok(tile.map(pbf_tile))
    }

    #[tokio::test]
    async fn through_cache_serves_a_hit_and_never_caches_a_miss() {
        let cache = TileCache::new(1, 60);
        let first = through_cache(Some(&cache), "t", (1, 0, 0), fetch(Some(vec![7])))
            .await
            .unwrap();
        assert_eq!(first.unwrap().data.as_ref(), [7]);
        // Cached: the new fetch result is not used.
        let second = through_cache(Some(&cache), "t", (1, 0, 0), fetch(Some(vec![8])))
            .await
            .unwrap();
        assert_eq!(second.unwrap().data.as_ref(), [7]);

        let miss = through_cache(Some(&cache), "t", (2, 0, 0), fetch(None))
            .await
            .unwrap();
        assert!(miss.is_none());
        // The miss was not cached: a later fetch result is used.
        let later = through_cache(Some(&cache), "t", (2, 0, 0), fetch(Some(vec![5])))
            .await
            .unwrap();
        assert_eq!(later.unwrap().data.as_ref(), [5]);

        let uncached = through_cache(None, "t", (1, 0, 0), fetch(Some(vec![9])))
            .await
            .unwrap();
        assert_eq!(
            uncached.unwrap().data.as_ref(),
            [9],
            "no cache: always fetch"
        );
    }
}
