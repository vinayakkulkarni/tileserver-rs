//! `tileserver-rs inspect`: describe a PMTiles or MBTiles archive without
//! starting the server (issue #604).
//!
//! The report holds what the archive header and metadata say (format,
//! compression, zoom range, bounds, layers), the tile count, and on request
//! the per-zoom tile counts (MBTiles) and a decoded sample tile.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use bytes::Bytes;
use geozero::mvt::{Message, Tile, tile::GeomType};
use pmtiles::{AsyncBackend, AsyncPmTilesReader, HttpBackend, MmapBackend, TileCoord, TileType};
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde::Serialize;

use crate::compression;
use crate::error::{Result, TileServerError};
use crate::sources::TileCompression;

/// What to include beyond the header and metadata.
#[derive(Debug, Clone, Copy, Default)]
pub struct InspectOptions {
    /// Count tiles per zoom level (MBTiles only; PMTiles would need a full
    /// directory walk).
    pub per_zoom: bool,
    /// Decode one tile.
    pub tile: Option<TileAddress>,
}

/// A `z/x/y` tile address in XYZ order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TileAddress {
    pub z: u8,
    pub x: u32,
    pub y: u32,
}

/// Archive container format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Container {
    Pmtiles,
    Mbtiles,
}

/// Everything `inspect` reports about one archive.
#[derive(Debug, Clone, Serialize)]
pub struct InspectReport {
    pub target: String,
    pub container: Container,
    /// PMTiles spec version; `None` for MBTiles.
    pub spec_version: Option<u8>,
    /// `mvt`, `mlt`, `png`, `jpeg`, `webp`, `avif` or `unknown`.
    pub tile_type: String,
    /// Tile compression (`none`, `gzip`, `brotli`, `zstd`, `unknown`).
    pub tile_compression: String,
    pub size_bytes: Option<u64>,
    /// Number of addressed tiles.
    pub tile_count: Option<u64>,
    /// Number of distinct tile payloads (PMTiles deduplication).
    pub unique_tiles: Option<u64>,
    pub min_zoom: Option<u8>,
    pub max_zoom: Option<u8>,
    pub bounds: Option<[f64; 4]>,
    pub center: Option<[f64; 3]>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub attribution: Option<String>,
    pub vector_layers: Vec<LayerSummary>,
    /// Tiles per zoom level, when requested and available.
    pub zoom_counts: Option<BTreeMap<u8, u64>>,
    pub tile: Option<TileReport>,
}

/// One entry of the archive's `vector_layers` metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LayerSummary {
    pub id: String,
    pub fields: usize,
    pub min_zoom: Option<u8>,
    pub max_zoom: Option<u8>,
}

/// One decoded tile.
#[derive(Debug, Clone, Serialize)]
pub struct TileReport {
    pub address: TileAddress,
    /// `false` when the archive has no tile at this address.
    pub found: bool,
    /// Stored size (before decompression).
    pub stored_bytes: usize,
    /// Size after decompression.
    pub raw_bytes: usize,
    pub layers: Vec<TileLayerReport>,
}

/// One layer of a decoded vector tile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TileLayerReport {
    pub name: String,
    pub extent: u32,
    pub features: usize,
    /// Feature count per geometry type (`point`, `linestring`, `polygon`, `unknown`).
    pub geometry: BTreeMap<String, usize>,
    pub keys: Vec<String>,
}

/// Parse a `z/x/y` tile address and check it lies on the zoom's grid.
///
/// # Errors
///
/// Returns [`TileServerError::InvalidTileRequest`] for a malformed address
/// and [`TileServerError::InvalidCoordinates`] for one off the grid.
pub fn parse_tile_address(text: &str) -> Result<TileAddress> {
    let mut parts = text.trim().split('/');
    let (Some(z), Some(x), Some(y), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(TileServerError::InvalidTileRequest);
    };
    let z: u8 = z.parse().map_err(|_| TileServerError::InvalidTileRequest)?;
    let x: u32 = x.parse().map_err(|_| TileServerError::InvalidTileRequest)?;
    let y: u32 = y.parse().map_err(|_| TileServerError::InvalidTileRequest)?;
    if z > 30 {
        return Err(TileServerError::InvalidCoordinates { z, x, y });
    }
    let side = 1u64 << z;
    if u64::from(x) >= side || u64::from(y) >= side {
        return Err(TileServerError::InvalidCoordinates { z, x, y });
    }
    Ok(TileAddress { z, x, y })
}

/// Inspect a local `.pmtiles` / `.mbtiles` file or a remote `http(s)` PMTiles URL.
///
/// # Errors
///
/// Returns an error when the target does not exist, has an unsupported
/// extension, or cannot be read as the format its name says.
pub async fn inspect(target: &str, options: InspectOptions) -> Result<InspectReport> {
    if target.starts_with("http://") || target.starts_with("https://") {
        return inspect_remote_pmtiles(target, options).await;
    }

    let path = Path::new(target);
    if !path.exists() {
        return Err(TileServerError::NotFound(target.to_owned()));
    }
    let size = std::fs::metadata(path)?.len();
    match extension(path).as_deref() {
        Some("pmtiles") => {
            let backend = MmapBackend::try_from(path)
                .await
                .map_err(|e| TileServerError::MetadataError(format!("{target}: {e}")))?;
            let reader = AsyncPmTilesReader::try_from_source(backend)
                .await
                .map_err(|e| TileServerError::MetadataError(format!("{target}: {e}")))?;
            inspect_pmtiles(target, &reader, Some(size), options).await
        }
        Some("mbtiles") => {
            let target = target.to_owned();
            tokio::task::spawn_blocking(move || inspect_mbtiles(&target, size, options))
                .await
                .map_err(|e| TileServerError::Internal(e.into()))?
        }
        _ => Err(TileServerError::ConfigError(format!(
            "unsupported file type: {target} (expected .pmtiles or .mbtiles)"
        ))),
    }
}

fn extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
}

async fn inspect_remote_pmtiles(url: &str, options: InspectOptions) -> Result<InspectReport> {
    let client = pmtiles::reqwest::Client::builder()
        .user_agent(concat!("tileserver-rs/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| TileServerError::Internal(e.into()))?;
    // A HEAD request gives the archive size and turns a missing file into a
    // clear "not found" before the reader asks for the header range.
    let head = client
        .head(url)
        .send()
        .await
        .map_err(|e| TileServerError::NotFound(format!("{url}: {e}")))?;
    if head.status() == pmtiles::reqwest::StatusCode::NOT_FOUND {
        return Err(TileServerError::NotFound(url.to_owned()));
    }
    // `content_length()` reports the (empty) HEAD body, so read the header.
    let size = head
        .headers()
        .get(pmtiles::reqwest::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|&n| n > 0);
    let backend = HttpBackend::try_from(client, url)
        .map_err(|e| TileServerError::ConfigError(format!("invalid URL {url}: {e}")))?;
    let reader = AsyncPmTilesReader::try_from_source(backend)
        .await
        .map_err(|e| TileServerError::MetadataError(format!("{url}: {e}")))?;
    inspect_pmtiles(url, &reader, size, options).await
}

async fn inspect_pmtiles<B: AsyncBackend + Sync + Send>(
    target: &str,
    reader: &AsyncPmTilesReader<B>,
    size_bytes: Option<u64>,
    options: InspectOptions,
) -> Result<InspectReport> {
    let header = reader.get_header();
    let metadata: serde_json::Value = reader
        .get_metadata()
        .await
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or(serde_json::Value::Null);

    let tile_compression = compression_from_pmtiles(header.tile_compression);
    let tile = match options.tile {
        Some(address) => {
            let coord = TileCoord::new(address.z, address.x, address.y).map_err(|_| {
                TileServerError::InvalidCoordinates {
                    z: address.z,
                    x: address.x,
                    y: address.y,
                }
            })?;
            let data = reader
                .get_tile(coord)
                .await
                .map_err(|e| TileServerError::MetadataError(format!("tile read failed: {e}")))?;
            Some(tile_report(address, data, tile_compression)?)
        }
        None => None,
    };

    Ok(InspectReport {
        target: target.to_owned(),
        container: Container::Pmtiles,
        spec_version: Some(header.spec_version()),
        tile_type: tile_type_name(header.tile_type).to_owned(),
        tile_compression: compression_name(tile_compression).to_owned(),
        size_bytes,
        tile_count: header.n_addressed_tiles().map(u64::from),
        unique_tiles: header.n_tile_contents().map(u64::from),
        min_zoom: Some(header.min_zoom),
        max_zoom: Some(header.max_zoom),
        bounds: Some([
            header.min_longitude,
            header.min_latitude,
            header.max_longitude,
            header.max_latitude,
        ]),
        center: Some([
            header.center_longitude,
            header.center_latitude,
            f64::from(header.center_zoom),
        ]),
        name: json_str(&metadata, "name"),
        description: json_str(&metadata, "description"),
        attribution: json_str(&metadata, "attribution"),
        vector_layers: layer_summaries(metadata.get("vector_layers")),
        // A per-zoom count needs every leaf directory; header data cannot give it.
        zoom_counts: None,
        tile,
    })
}

fn inspect_mbtiles(target: &str, size: u64, options: InspectOptions) -> Result<InspectReport> {
    let conn = Connection::open_with_flags(target, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| TileServerError::MbTilesError(format!("cannot open {target}: {e}")))?;
    let db = |e: rusqlite::Error| TileServerError::MbTilesError(e.to_string());

    let mut meta: BTreeMap<String, String> = BTreeMap::new();
    {
        let mut stmt = conn
            .prepare("SELECT name, value FROM metadata")
            .map_err(db)?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(db)?;
        for row in rows {
            let (name, value) = row.map_err(db)?;
            meta.insert(name, value);
        }
    }

    let (count, min_z, max_z): (i64, Option<u8>, Option<u8>) = conn
        .query_row(
            "SELECT COUNT(*), MIN(zoom_level), MAX(zoom_level) FROM tiles",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(db)?;
    let count = u64::try_from(count).unwrap_or(0);

    let zoom_counts = if options.per_zoom {
        let mut stmt = conn
            .prepare(
                "SELECT zoom_level, COUNT(*) FROM tiles GROUP BY zoom_level ORDER BY zoom_level",
            )
            .map_err(db)?;
        let rows = stmt
            .query_map([], |row| Ok((row.get::<_, u8>(0)?, row.get::<_, i64>(1)?)))
            .map_err(db)?;
        let mut counts = BTreeMap::new();
        for row in rows {
            let (z, n) = row.map_err(db)?;
            counts.insert(z, u64::try_from(n).unwrap_or(0));
        }
        Some(counts)
    } else {
        None
    };

    let json_meta: serde_json::Value = meta
        .get("json")
        .and_then(|text| serde_json::from_str(text).ok())
        .unwrap_or(serde_json::Value::Null);
    let tile_type = mbtiles_tile_type(meta.get("format").map(String::as_str));

    // MBTiles stores tiles in TMS order: row = 2^z - 1 - y.
    let sample: Option<Vec<u8>> = match options.tile {
        Some(a) => {
            let row = (1u32 << a.z) - 1 - a.y;
            conn.query_row(
                "SELECT tile_data FROM tiles WHERE zoom_level = ?1 AND tile_column = ?2 AND tile_row = ?3",
                rusqlite::params![a.z, a.x, row],
                |r| r.get(0),
            )
            .optional()
            .map_err(db)?
        }
        None => None,
    };
    // Vector tiles in MBTiles are usually gzip; detect per tile, not per archive.
    let sample_compression = sample
        .as_deref()
        .map_or(TileCompression::None, sniff_compression);
    let first_compression = if tile_type == "mvt" || tile_type == "mlt" {
        conn.query_row("SELECT tile_data FROM tiles LIMIT 1", [], |r| {
            r.get::<_, Vec<u8>>(0)
        })
        .optional()
        .map_err(db)?
        .map_or(TileCompression::None, |data| sniff_compression(&data))
    } else {
        TileCompression::None
    };
    let tile = match options.tile {
        Some(address) => Some(tile_report(
            address,
            sample.map(Bytes::from),
            sample_compression,
        )?),
        None => None,
    };

    Ok(InspectReport {
        target: target.to_owned(),
        container: Container::Mbtiles,
        spec_version: None,
        tile_type: tile_type.to_owned(),
        tile_compression: compression_name(first_compression).to_owned(),
        size_bytes: Some(size),
        tile_count: Some(count),
        unique_tiles: None,
        min_zoom: meta.get("minzoom").and_then(|v| v.parse().ok()).or(min_z),
        max_zoom: meta.get("maxzoom").and_then(|v| v.parse().ok()).or(max_z),
        bounds: meta.get("bounds").and_then(|v| parse_floats::<4>(v)),
        center: meta.get("center").and_then(|v| parse_floats::<3>(v)),
        name: meta.get("name").cloned(),
        description: meta.get("description").cloned(),
        attribution: meta.get("attribution").cloned(),
        vector_layers: layer_summaries(json_meta.get("vector_layers")),
        zoom_counts,
        tile,
    })
}

/// Tile type name for an MBTiles `format` metadata value.
fn mbtiles_tile_type(format: Option<&str>) -> &'static str {
    match format.map(str::to_ascii_lowercase).as_deref() {
        Some("pbf" | "mvt") => "mvt",
        Some("mlt") => "mlt",
        Some("png") => "png",
        Some("jpg" | "jpeg") => "jpeg",
        Some("webp") => "webp",
        Some("avif") => "avif",
        _ => "unknown",
    }
}

/// Decode one stored tile. Vector tiles get per-layer statistics; other
/// formats report sizes only.
fn tile_report(
    address: TileAddress,
    data: Option<Bytes>,
    compression: TileCompression,
) -> Result<TileReport> {
    let Some(data) = data else {
        return Ok(TileReport {
            address,
            found: false,
            stored_bytes: 0,
            raw_bytes: 0,
            layers: Vec::new(),
        });
    };
    let raw = compression::decode(&data, compression)?;
    let layers = match Tile::decode(raw.as_slice()) {
        Ok(tile) if !tile.layers.is_empty() => tile.layers.iter().map(layer_report).collect(),
        // Not an MVT payload (raster, MLT, or an empty tile).
        _ => Vec::new(),
    };
    Ok(TileReport {
        address,
        found: true,
        stored_bytes: data.len(),
        raw_bytes: raw.len(),
        layers,
    })
}

fn layer_report(layer: &geozero::mvt::tile::Layer) -> TileLayerReport {
    let mut geometry: BTreeMap<String, usize> = BTreeMap::new();
    for feature in &layer.features {
        let kind = match feature.r#type() {
            GeomType::Point => "point",
            GeomType::Linestring => "linestring",
            GeomType::Polygon => "polygon",
            GeomType::Unknown => "unknown",
        };
        *geometry.entry(kind.to_owned()).or_default() += 1;
    }
    TileLayerReport {
        name: layer.name.clone(),
        extent: layer.extent.unwrap_or(4096),
        features: layer.features.len(),
        geometry,
        keys: layer.keys.clone(),
    }
}

fn layer_summaries(value: Option<&serde_json::Value>) -> Vec<LayerSummary> {
    let Some(serde_json::Value::Array(layers)) = value else {
        return Vec::new();
    };
    layers
        .iter()
        .filter_map(|layer| {
            let id = layer.get("id")?.as_str()?.to_owned();
            let zoom = |key: &str| {
                layer
                    .get(key)
                    .and_then(serde_json::Value::as_u64)
                    .and_then(|z| u8::try_from(z).ok())
            };
            Some(LayerSummary {
                id,
                fields: layer
                    .get("fields")
                    .and_then(serde_json::Value::as_object)
                    .map_or(0, serde_json::Map::len),
                min_zoom: zoom("minzoom"),
                max_zoom: zoom("maxzoom"),
            })
        })
        .collect()
}

fn json_str(value: &serde_json::Value, key: &str) -> Option<String> {
    value.get(key)?.as_str().map(str::to_owned)
}

fn parse_floats<const N: usize>(text: &str) -> Option<[f64; N]> {
    let values: Vec<f64> = text
        .split(',')
        .map(|v| v.trim().parse().ok())
        .collect::<Option<_>>()?;
    values.try_into().ok()
}

fn sniff_compression(data: &[u8]) -> TileCompression {
    if data.starts_with(&[0x1f, 0x8b]) {
        TileCompression::Gzip
    } else {
        TileCompression::None
    }
}

const fn tile_type_name(tile_type: TileType) -> &'static str {
    match tile_type {
        TileType::Mvt => "mvt",
        TileType::Mlt => "mlt",
        TileType::Png => "png",
        TileType::Jpeg => "jpeg",
        TileType::Webp => "webp",
        TileType::Avif => "avif",
        TileType::Unknown => "unknown",
    }
}

const fn compression_from_pmtiles(compression: pmtiles::Compression) -> TileCompression {
    match compression {
        pmtiles::Compression::Gzip => TileCompression::Gzip,
        pmtiles::Compression::Brotli => TileCompression::Brotli,
        pmtiles::Compression::Zstd => TileCompression::Zstd,
        pmtiles::Compression::None | pmtiles::Compression::Unknown => TileCompression::None,
    }
}

const fn compression_name(compression: TileCompression) -> &'static str {
    match compression {
        TileCompression::None => "none",
        TileCompression::Gzip => "gzip",
        TileCompression::Brotli => "brotli",
        TileCompression::Zstd => "zstd",
    }
}

/// Run `inspect` and render the report: plain text, or pretty JSON when
/// `json` is set. `tile` is a `z/x/y` address; `per_zoom` counts tiles per
/// zoom level (MBTiles).
///
/// # Errors
///
/// Returns an error for a malformed tile address, an unreadable target, or
/// a report that cannot be serialized.
pub async fn run(target: &str, tile: Option<&str>, per_zoom: bool, json: bool) -> Result<String> {
    let tile = tile.map(parse_tile_address).transpose()?;
    let report = inspect(target, InspectOptions { per_zoom, tile }).await?;
    if json {
        let mut out = serde_json::to_string_pretty(&report)
            .map_err(|e| TileServerError::Internal(e.into()))?;
        out.push('\n');
        Ok(out)
    } else {
        Ok(render_text(&report))
    }
}

/// Human-readable size: `1.2 GB`, `40.4 KB`, `512 B`.
#[must_use]
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

/// Render the report as aligned plain text.
#[must_use]
pub fn render_text(report: &InspectReport) -> String {
    let mut out = String::with_capacity(1024);
    let line = |out: &mut String, label: &str, value: &str| {
        let _ = writeln!(out, "{label:<13} {value}");
    };
    line(&mut out, "File:", &report.target);
    let container = match (report.container, report.spec_version) {
        (Container::Pmtiles, Some(v)) => format!("PMTiles v{v}"),
        (Container::Pmtiles, None) => "PMTiles".to_owned(),
        (Container::Mbtiles, _) => "MBTiles".to_owned(),
    };
    line(&mut out, "Format:", &container);
    line(&mut out, "Tile type:", &report.tile_type);
    line(&mut out, "Compression:", &report.tile_compression);
    if let Some(size) = report.size_bytes {
        line(&mut out, "Size:", &format_bytes(size));
    }
    if let Some(count) = report.tile_count {
        line(&mut out, "Tiles:", &count.to_string());
    }
    if let Some(unique) = report.unique_tiles {
        line(&mut out, "Unique tiles:", &unique.to_string());
    }
    if let (Some(min), Some(max)) = (report.min_zoom, report.max_zoom) {
        line(&mut out, "Zoom:", &format!("{min}-{max}"));
    }
    if let Some([w, s, e, n]) = report.bounds {
        line(&mut out, "Bounds:", &format!("[{w}, {s}, {e}, {n}]"));
    }
    if let Some([lon, lat, z]) = report.center {
        line(&mut out, "Center:", &format!("[{lon}, {lat}, {z}]"));
    }
    for (label, value) in [
        ("Name:", &report.name),
        ("Description:", &report.description),
        ("Attribution:", &report.attribution),
    ] {
        if let Some(value) = value {
            line(&mut out, label, value);
        }
    }

    if !report.vector_layers.is_empty() {
        let _ = writeln!(out, "\nLayers ({}):", report.vector_layers.len());
        let _ = writeln!(out, "  {:<24} {:>6}  ZOOM", "NAME", "FIELDS");
        for layer in &report.vector_layers {
            let zoom = match (layer.min_zoom, layer.max_zoom) {
                (Some(min), Some(max)) => format!("{min}-{max}"),
                _ => "-".to_owned(),
            };
            let _ = writeln!(out, "  {:<24} {:>6}  {zoom}", layer.id, layer.fields);
        }
    }

    if let Some(counts) = &report.zoom_counts {
        let _ = writeln!(out, "\nTiles per zoom:");
        for (z, n) in counts {
            let _ = writeln!(out, "  z{z:<3} {n}");
        }
    }

    if let Some(tile) = &report.tile {
        let a = tile.address;
        if !tile.found {
            let _ = writeln!(out, "\nTile {}/{}/{}: not in the archive", a.z, a.x, a.y);
        } else {
            let _ = writeln!(
                out,
                "\nTile {}/{}/{}: {} stored, {} raw",
                a.z,
                a.x,
                a.y,
                format_bytes(tile.stored_bytes as u64),
                format_bytes(tile.raw_bytes as u64)
            );
            if !tile.layers.is_empty() {
                let _ = writeln!(
                    out,
                    "  {:<24} {:>8}  {:<28} KEYS",
                    "LAYER", "FEATURES", "GEOMETRY"
                );
                for layer in &tile.layers {
                    let geometry = layer
                        .geometry
                        .iter()
                        .map(|(kind, n)| format!("{kind}:{n}"))
                        .collect::<Vec<_>>()
                        .join(" ");
                    let _ = writeln!(
                        out,
                        "  {:<24} {:>8}  {geometry:<28} {}",
                        layer.name,
                        layer.features,
                        layer.keys.len()
                    );
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use geozero::mvt::tile::{Feature, Layer};

    fn fixture(name: &str) -> String {
        format!("{}/../../data/tiles/{name}", env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn tile_address_parses_and_checks_grid() {
        assert_eq!(
            parse_tile_address("14/8192/5461").unwrap(),
            TileAddress {
                z: 14,
                x: 8192,
                y: 5461
            }
        );
        assert!(matches!(
            parse_tile_address("1/2/0"),
            Err(TileServerError::InvalidCoordinates { z: 1, x: 2, y: 0 })
        ));
        for bad in ["", "1/2", "1/2/3/4", "a/b/c", "-1/0/0", "31/0/0"] {
            assert!(parse_tile_address(bad).is_err(), "{bad} should not parse");
        }
    }

    #[test]
    fn type_and_compression_names() {
        for (format, name) in [
            (Some("pbf"), "mvt"),
            (Some("MVT"), "mvt"),
            (Some("mlt"), "mlt"),
            (Some("png"), "png"),
            (Some("jpg"), "jpeg"),
            (Some("jpeg"), "jpeg"),
            (Some("webp"), "webp"),
            (Some("avif"), "avif"),
            (Some("tiff"), "unknown"),
            (None, "unknown"),
        ] {
            assert_eq!(mbtiles_tile_type(format), name, "{format:?}");
        }
        for (tile_type, name) in [
            (TileType::Mvt, "mvt"),
            (TileType::Mlt, "mlt"),
            (TileType::Png, "png"),
            (TileType::Jpeg, "jpeg"),
            (TileType::Webp, "webp"),
            (TileType::Avif, "avif"),
            (TileType::Unknown, "unknown"),
        ] {
            assert_eq!(tile_type_name(tile_type), name);
        }
        for (pm, name) in [
            (pmtiles::Compression::None, "none"),
            (pmtiles::Compression::Unknown, "none"),
            (pmtiles::Compression::Gzip, "gzip"),
            (pmtiles::Compression::Brotli, "brotli"),
            (pmtiles::Compression::Zstd, "zstd"),
        ] {
            assert_eq!(compression_name(compression_from_pmtiles(pm)), name);
        }
        assert_eq!(sniff_compression(&[0x1f, 0x8b, 0]), TileCompression::Gzip);
        assert_eq!(sniff_compression(&[0x1a]), TileCompression::None);
    }

    #[test]
    fn non_vector_tiles_report_sizes_only() {
        let png = Bytes::from_static(&[0x89, b'P', b'N', b'G', 0, 0]);
        let report = tile_report(
            TileAddress { z: 0, x: 0, y: 0 },
            Some(png),
            TileCompression::None,
        )
        .unwrap();
        assert!(report.found);
        assert_eq!(report.stored_bytes, 6);
        assert!(report.layers.is_empty());
    }

    #[test]
    fn bytes_are_human_readable() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(41_370), "40.4 KB");
        assert_eq!(format_bytes(1_288_490_189), "1.2 GB");
    }

    #[test]
    fn layer_summaries_read_vector_layers() {
        let value = serde_json::json!([
            { "id": "water", "fields": { "class": "String", "name": "String" }, "minzoom": 0, "maxzoom": 14 },
            { "id": "roads" },
            { "fields": {} }
        ]);
        assert_eq!(
            layer_summaries(Some(&value)),
            vec![
                LayerSummary {
                    id: "water".into(),
                    fields: 2,
                    min_zoom: Some(0),
                    max_zoom: Some(14)
                },
                LayerSummary {
                    id: "roads".into(),
                    fields: 0,
                    min_zoom: None,
                    max_zoom: None
                },
            ]
        );
        assert!(layer_summaries(None).is_empty());
    }

    #[test]
    fn floats_parse_with_exact_length() {
        assert_eq!(parse_floats::<3>("8.5, 47.3, 12"), Some([8.5, 47.3, 12.0]));
        assert_eq!(parse_floats::<3>("8.5,47.3"), None);
        assert_eq!(parse_floats::<4>("a,b,c,d"), None);
    }

    #[test]
    fn tile_report_counts_features_per_geometry() {
        let feature = |kind: GeomType| Feature {
            r#type: Some(kind as i32),
            ..Default::default()
        };
        let tile = Tile {
            layers: vec![Layer {
                version: 2,
                name: "places".into(),
                features: vec![
                    feature(GeomType::Point),
                    feature(GeomType::Point),
                    feature(GeomType::Polygon),
                ],
                keys: vec!["name".into(), "kind".into()],
                extent: Some(4096),
                ..Default::default()
            }],
        };
        let raw = tile.encode_to_vec();
        let gz = compression::encode(&raw, TileCompression::Gzip, &Default::default()).unwrap();

        let report = tile_report(
            TileAddress { z: 0, x: 0, y: 0 },
            Some(Bytes::from(gz.clone())),
            TileCompression::Gzip,
        )
        .unwrap();
        assert!(report.found);
        assert_eq!(report.stored_bytes, gz.len());
        assert_eq!(report.raw_bytes, raw.len());
        assert_eq!(
            report.layers,
            vec![TileLayerReport {
                name: "places".into(),
                extent: 4096,
                features: 3,
                geometry: BTreeMap::from([("point".into(), 2), ("polygon".into(), 1)]),
                keys: vec!["name".into(), "kind".into()],
            }]
        );

        let missing = tile_report(
            TileAddress { z: 0, x: 0, y: 0 },
            None,
            TileCompression::None,
        )
        .unwrap();
        assert!(!missing.found);
    }

    #[tokio::test]
    async fn inspects_pmtiles_fixture() {
        let path = fixture("protomaps-sample.pmtiles");
        let report = inspect(
            &path,
            InspectOptions {
                per_zoom: false,
                tile: Some(TileAddress { z: 0, x: 0, y: 0 }),
            },
        )
        .await
        .unwrap();
        assert_eq!(report.container, Container::Pmtiles);
        assert_eq!(report.spec_version, Some(3));
        assert_eq!(report.tile_type, "mvt");
        assert!(report.tile_count.unwrap() > 0);
        assert_eq!(
            report.size_bytes,
            Some(std::fs::metadata(&path).unwrap().len())
        );
        assert!(!report.vector_layers.is_empty());
        let tile = report.tile.unwrap();
        assert!(tile.found);
        assert!(!tile.layers.is_empty());
        assert!(report.zoom_counts.is_none());
    }

    #[tokio::test]
    async fn inspects_mbtiles_fixture_with_zoom_counts() {
        let path = fixture("zurich_switzerland.mbtiles");
        let report = inspect(
            &path,
            InspectOptions {
                per_zoom: true,
                tile: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(report.container, Container::Mbtiles);
        assert_eq!(report.tile_type, "mvt");
        let counts = report.zoom_counts.unwrap();
        assert_eq!(counts.values().sum::<u64>(), report.tile_count.unwrap());
        assert_eq!(counts.keys().next().copied(), report.min_zoom);
    }

    #[tokio::test]
    async fn mbtiles_tile_uses_xyz_address() {
        let path = fixture("zurich_switzerland.mbtiles");
        let z = 10;
        // Find a stored tile and convert its TMS row to XYZ.
        let (x, tms_row): (u32, u32) = Connection::open(&path)
            .unwrap()
            .query_row(
                "SELECT tile_column, tile_row FROM tiles WHERE zoom_level = ?1 LIMIT 1",
                [z],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        let y = (1u32 << z) - 1 - tms_row;
        let report = inspect(
            &path,
            InspectOptions {
                per_zoom: false,
                tile: Some(TileAddress { z, x, y }),
            },
        )
        .await
        .unwrap();
        let tile = report.tile.unwrap();
        assert!(tile.found, "tile {z}/{x}/{y} should exist");
        assert!(!tile.layers.is_empty());
    }

    #[tokio::test]
    async fn text_report_lists_header_layers_and_tile() {
        let out = run(
            &fixture("protomaps-sample.pmtiles"),
            Some("0/0/0"),
            false,
            false,
        )
        .await
        .unwrap();
        for needle in [
            "Format:       PMTiles v3",
            "Tile type:    mvt",
            "Compression:  gzip",
            "Unique tiles:",
            "Zoom:         0-15",
            "Bounds:",
            "Center:",
            "Name:         Protomaps Basemap",
            "Layers (13):",
            "  NAME                     FIELDS  ZOOM",
            "Tile 0/0/0:",
            "  LAYER                    FEATURES  GEOMETRY",
            "polygon:",
        ] {
            assert!(out.contains(needle), "missing {needle:?} in:\n{out}");
        }
    }

    #[tokio::test]
    async fn text_report_shows_zoom_counts_and_missing_tile() {
        let out = run(
            &fixture("zurich_switzerland.mbtiles"),
            Some("14/0/0"),
            true,
            false,
        )
        .await
        .unwrap();
        assert!(out.contains("Format:       MBTiles"));
        assert!(out.contains("Tiles per zoom:"));
        assert!(out.contains("  z0   1"));
        assert!(out.contains("Tile 14/0/0: not in the archive"));
    }

    #[tokio::test]
    async fn json_report_round_trips() {
        let out = run(&fixture("zurich_switzerland.mbtiles"), None, false, true)
            .await
            .unwrap();
        let value: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(value["container"], "mbtiles");
        assert_eq!(value["tile_type"], "mvt");
        assert!(value["tile_count"].as_u64().unwrap() > 0);
        assert!(value["zoom_counts"].is_null());
        assert!(matches!(
            run(
                &fixture("zurich_switzerland.mbtiles"),
                Some("9/x/1"),
                false,
                true
            )
            .await,
            Err(TileServerError::InvalidTileRequest)
        ));
    }

    /// Serve the PMTiles fixture over HTTP (with Range support) and inspect
    /// it by URL, the same path a remote archive takes.
    #[tokio::test]
    async fn inspects_remote_pmtiles_over_http() {
        let dir = format!("{}/../../data/tiles", env!("CARGO_MANIFEST_DIR"));
        let app = axum::Router::new().fallback_service(tower_http::services::ServeDir::new(dir));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await });

        let url = format!("http://{addr}/protomaps-sample.pmtiles");
        let report = inspect(
            &url,
            InspectOptions {
                per_zoom: false,
                tile: Some(TileAddress { z: 0, x: 0, y: 0 }),
            },
        )
        .await
        .unwrap();
        assert_eq!(report.target, url);
        assert_eq!(report.container, Container::Pmtiles);
        assert_eq!(
            report.size_bytes,
            Some(
                std::fs::metadata(fixture("protomaps-sample.pmtiles"))
                    .unwrap()
                    .len()
            )
        );
        assert!(report.tile.unwrap().found);

        let missing = format!("http://{addr}/nope.pmtiles");
        assert!(matches!(
            inspect(&missing, InspectOptions::default()).await,
            Err(TileServerError::NotFound(_))
        ));
    }

    #[tokio::test]
    async fn rejects_unknown_extension_and_missing_file() {
        let path = fixture("../configs/example.toml");
        assert!(matches!(
            inspect(&path, InspectOptions::default()).await,
            Err(TileServerError::ConfigError(_) | TileServerError::NotFound(_))
        ));
        assert!(matches!(
            inspect("/no/such/file.pmtiles", InspectOptions::default()).await,
            Err(TileServerError::NotFound(_))
        ));
    }
}
