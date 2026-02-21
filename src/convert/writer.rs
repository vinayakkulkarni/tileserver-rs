use anyhow::{Context, Result};
use geo::Rect;
use pmtiles::{Compression, PmTilesWriter, TileCoord, TileType};
use serde_json::json;
use std::collections::BTreeMap;
use std::fs::File;
use std::path::Path;

/// Collects MVT tile data and writes a PMTiles archive at the end.
///
/// Tiles are stored in a BTreeMap sorted by Hilbert tile ID to produce a
/// clustered, read-optimised archive.
pub struct PmTilesCollector {
    tiles: BTreeMap<u64, (TileCoord, Vec<u8>)>,
    min_zoom: u8,
    max_zoom: u8,
    bbox: Rect<f64>,
    layer_name: String,
}

impl PmTilesCollector {
    pub fn new(min_zoom: u8, max_zoom: u8, bbox: Rect<f64>, layer_name: String) -> Self {
        Self {
            tiles: BTreeMap::new(),
            min_zoom,
            max_zoom,
            bbox,
            layer_name,
        }
    }

    /// Insert a tile. Non-empty `data` only; empty tiles are skipped per PMTiles spec.
    pub fn add_tile(&mut self, z: u8, x: u32, y: u32, data: Vec<u8>) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }
        let coord = TileCoord::new(z, x, y).context("Invalid tile coordinate")?;
        let id: pmtiles::TileId = coord.into();
        self.tiles.insert(id.value(), (coord, data));
        Ok(())
    }

    /// Total number of non-empty tiles collected.
    pub fn tile_count(&self) -> usize {
        self.tiles.len()
    }

    /// Finalize and write the PMTiles archive to `output_path`.
    pub fn write(self, output_path: &Path) -> Result<()> {
        let west = self.bbox.min().x;
        let south = self.bbox.min().y;
        let east = self.bbox.max().x;
        let north = self.bbox.max().y;
        let center_lon = (west + east) / 2.0;
        let center_lat = (south + north) / 2.0;
        let center_zoom = (self.min_zoom + self.max_zoom) / 2;

        // Build TileJSON-compatible metadata for the archive
        let metadata = json!({
            "name": self.layer_name,
            "format": "pbf",
            "minzoom": self.min_zoom,
            "maxzoom": self.max_zoom,
            "bounds": [west, south, east, north],
            "center": [center_lon, center_lat, center_zoom],
            "vector_layers": [{
                "id": self.layer_name,
                "description": "",
                "minzoom": self.min_zoom,
                "maxzoom": self.max_zoom,
                "fields": {}
            }]
        });

        let file = File::create(output_path)
            .with_context(|| format!("Failed to create output file: {}", output_path.display()))?;

        let mut writer = PmTilesWriter::new(TileType::Mvt)
            .min_zoom(self.min_zoom)
            .max_zoom(self.max_zoom)
            .bounds(west, south, east, north)
            .center(center_lon, center_lat)
            .center_zoom(center_zoom)
            // MVT default is Gzip; tiles are pre-uncompressed, writer compresses them
            .tile_compression(Compression::Gzip)
            .internal_compression(Compression::Gzip)
            .metadata(&metadata.to_string())
            .create(file)
            .context("Failed to initialise PMTiles writer")?;

        // Tiles are already sorted by Hilbert tile ID (BTreeMap order)
        for (_id, (coord, data)) in self.tiles {
            writer
                .add_tile(coord, &data)
                .context("Failed to write tile to PMTiles archive")?;
        }

        writer
            .finalize()
            .context("Failed to finalise PMTiles archive")?;

        Ok(())
    }
}
