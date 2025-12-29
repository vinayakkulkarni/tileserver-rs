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
mod pool;
mod source;
mod table;

pub use cache::{TileCache, TileCacheKey};
pub use pool::{PoolSettings, PostgresPool};
pub use source::PostgresFunctionSource;
pub use table::PostgresTableSource;

use semver::Version;

/// Minimum PostgreSQL version required (11.0.0)
pub const MINIMUM_POSTGRES_VERSION: Version = Version::new(11, 0, 0);

/// Minimum PostGIS version required (3.0.0) for ST_TileEnvelope support
pub const MINIMUM_POSTGIS_VERSION: Version = Version::new(3, 0, 0);

/// PostGIS version that supports margin parameter in ST_TileEnvelope (3.1.0)
pub const ST_TILE_ENVELOPE_MARGIN_VERSION: Version = Version::new(3, 1, 0);
