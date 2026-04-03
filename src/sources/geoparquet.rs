//! GeoParquet vector tile source with spatial filtering and MVT encoding.

use std::collections::HashMap;
use std::path::PathBuf;

use arrow_array::Array;
use arrow_schema::DataType;
use async_trait::async_trait;
use bytes::Bytes;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

use crate::config::SourceConfig;
use crate::error::{Result, TileServerError};
use crate::sources::{TileCompression, TileData, TileFormat, TileMetadata, TileSource};

const DEFAULT_MIN_ZOOM: u8 = 0;
const DEFAULT_MAX_ZOOM: u8 = 14;
const MVT_EXTENT: u32 = 4096;

pub fn tile_to_bbox(z: u8, x: u32, y: u32) -> [f64; 4] {
    let n = 2.0_f64.powi(z as i32);
    let lon_min = x as f64 / n * 360.0 - 180.0;
    let lon_max = (x + 1) as f64 / n * 360.0 - 180.0;
    let lat_max = (std::f64::consts::PI * (1.0 - 2.0 * y as f64 / n))
        .sinh()
        .atan()
        .to_degrees();
    let lat_min = (std::f64::consts::PI * (1.0 - 2.0 * (y + 1) as f64 / n))
        .sinh()
        .atan()
        .to_degrees();
    [lon_min, lat_min, lon_max, lat_max]
}

pub fn tile_to_bbox_with_buffer(z: u8, x: u32, y: u32, buffer_pixels: u32) -> [f64; 4] {
    let [lon_min, lat_min, lon_max, lat_max] = tile_to_bbox(z, x, y);
    let lon_range = lon_max - lon_min;
    let lat_range = lat_max - lat_min;
    let buffer_ratio = buffer_pixels as f64 / MVT_EXTENT as f64;
    let lon_buffer = lon_range * buffer_ratio;
    let lat_buffer = lat_range * buffer_ratio;
    [
        lon_min - lon_buffer,
        lat_min - lat_buffer,
        lon_max + lon_buffer,
        lat_max + lat_buffer,
    ]
}

fn extract_geo_metadata(
    file_metadata: &parquet::file::metadata::ParquetMetaData,
) -> Option<GeoMeta> {
    let kv = file_metadata.file_metadata().key_value_metadata()?;
    for entry in kv {
        if entry.key == "geo"
            && let Some(ref value) = entry.value
            && let Ok(geo_json) = serde_json::from_str::<serde_json::Value>(value)
        {
            let primary_column = geo_json
                .get("primary_column")
                .and_then(|v| v.as_str())
                .unwrap_or("geometry")
                .to_string();

            let columns = geo_json.get("columns");
            let col_info = columns.and_then(|c| c.get(&primary_column));

            let has_bbox_covering = col_info
                .and_then(|c| c.get("covering"))
                .and_then(|c| c.get("bbox"))
                .is_some();

            let bounds = col_info
                .and_then(|c| c.get("bbox"))
                .and_then(|b| b.as_array())
                .and_then(|arr| {
                    if arr.len() >= 4 {
                        Some([
                            arr[0].as_f64()?,
                            arr[1].as_f64()?,
                            arr[2].as_f64()?,
                            arr[3].as_f64()?,
                        ])
                    } else {
                        None
                    }
                });

            return Some(GeoMeta {
                geometry_column: primary_column,
                has_bbox_covering,
                bounds,
            });
        }
    }
    None
}

#[derive(Debug)]
struct GeoMeta {
    geometry_column: String,
    has_bbox_covering: bool,
    bounds: Option<[f64; 4]>,
}

fn extract_property_fields(
    schema: &arrow_schema::Schema,
    geometry_column: &str,
) -> Vec<(String, String)> {
    schema
        .fields()
        .iter()
        .filter(|f| {
            let name = f.name().as_str();
            name != "bbox" && name != geometry_column && name != "theme" && name != "type"
        })
        .filter(|f| {
            matches!(
                f.data_type(),
                DataType::Utf8
                    | DataType::LargeUtf8
                    | DataType::Int8
                    | DataType::Int16
                    | DataType::Int32
                    | DataType::Int64
                    | DataType::UInt8
                    | DataType::UInt16
                    | DataType::UInt32
                    | DataType::UInt64
                    | DataType::Float32
                    | DataType::Float64
                    | DataType::Boolean
            )
        })
        .map(|f| {
            let field_type = match f.data_type() {
                DataType::Utf8 | DataType::LargeUtf8 => "String",
                DataType::Boolean => "Boolean",
                _ => "Number",
            };
            (f.name().clone(), field_type.to_string())
        })
        .collect()
}

pub struct GeoParquetSource {
    metadata: TileMetadata,
    file_paths: Vec<PathBuf>,
    geometry_column: String,
    has_bbox_column: bool,
    property_fields: Vec<(String, String)>,
}

fn discover_parquet_files(path: &PathBuf) -> Result<Vec<PathBuf>> {
    if path.is_file() {
        return Ok(vec![path.clone()]);
    }
    if path.is_dir() {
        let mut files: Vec<PathBuf> = Vec::new();
        collect_parquet_files(path, &mut files)?;
        files.sort();
        if files.is_empty() {
            return Err(TileServerError::GeoParquetError(format!(
                "no .parquet files found in {}",
                path.display()
            )));
        }
        return Ok(files);
    }
    Err(TileServerError::GeoParquetError(format!(
        "path not found: {}",
        path.display()
    )))
}

fn collect_parquet_files(dir: &PathBuf, files: &mut Vec<PathBuf>) -> Result<()> {
    let entries = std::fs::read_dir(dir).map_err(|e| {
        TileServerError::GeoParquetError(format!("failed to read directory {}: {e}", dir.display()))
    })?;
    for entry in entries {
        let entry = entry
            .map_err(|e| TileServerError::GeoParquetError(format!("directory entry error: {e}")))?;
        let entry_path = entry.path();
        if entry_path.is_dir() {
            collect_parquet_files(&entry_path, files)?;
        } else if let Some(ext) = entry_path.extension() {
            let ext = ext.to_string_lossy().to_ascii_lowercase();
            if ext == "parquet" || ext == "geoparquet" {
                files.push(entry_path);
            }
        }
    }
    Ok(())
}

impl GeoParquetSource {
    pub async fn from_config(config: &SourceConfig) -> Result<Self> {
        let path = PathBuf::from(&config.path);
        let file_paths = discover_parquet_files(&path)?;
        let first_file = file_paths[0].clone();
        let file_count = file_paths.len();

        let config_geom_col = config.geometry_column.clone();
        let config_geom_col2 = config_geom_col.clone();
        let config_id = config.id.clone();
        let config_name = config.name.clone();
        let config_description = config.description.clone();
        let config_attribution = config.attribution.clone();
        let config_layer_name = config.layer_name.clone();
        let config_minzoom = config.minzoom;
        let config_maxzoom = config.maxzoom;

        let (geo_meta, property_fields, schema_fields) =
            tokio::task::spawn_blocking(move || -> Result<_> {
                let file = std::fs::File::open(&first_file).map_err(|e| {
                    TileServerError::GeoParquetError(format!("failed to open file: {e}"))
                })?;
                let builder = ParquetRecordBatchReaderBuilder::try_new(file).map_err(|e| {
                    TileServerError::GeoParquetError(format!("invalid parquet file: {e}"))
                })?;
                let parquet_meta = builder.metadata();
                let geo_meta = extract_geo_metadata(parquet_meta);
                let schema = builder.schema().clone();

                let geom_col = config_geom_col
                    .as_deref()
                    .or(geo_meta.as_ref().map(|m| m.geometry_column.as_str()))
                    .unwrap_or("geometry");

                let props = extract_property_fields(&schema, geom_col);

                let fields_json: serde_json::Value = props
                    .iter()
                    .map(|(name, typ)| (name.clone(), serde_json::Value::String(typ.clone())))
                    .collect::<serde_json::Map<String, serde_json::Value>>()
                    .into();

                Ok((geo_meta, props, fields_json))
            })
            .await
            .map_err(|e| TileServerError::GeoParquetError(format!("task join error: {e}")))??;

        let geometry_column = config_geom_col2.unwrap_or_else(|| {
            geo_meta
                .as_ref()
                .map(|m| m.geometry_column.clone())
                .unwrap_or_else(|| "geometry".to_string())
        });

        let has_bbox_column = geo_meta
            .as_ref()
            .map(|m| m.has_bbox_covering)
            .unwrap_or(false);

        let bounds = geo_meta.as_ref().and_then(|m| m.bounds);
        let layer_name = config_layer_name.unwrap_or_else(|| config_id.clone());
        let minzoom = config_minzoom.unwrap_or(DEFAULT_MIN_ZOOM);
        let maxzoom = config_maxzoom.unwrap_or(DEFAULT_MAX_ZOOM);

        let vector_layers = serde_json::json!([{
            "id": layer_name,
            "fields": schema_fields,
            "minzoom": minzoom,
            "maxzoom": maxzoom,
        }]);

        let display_name = config_name.unwrap_or_else(|| {
            if path.is_dir() {
                path.file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "geoparquet".to_string())
            } else {
                path.file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "geoparquet".to_string())
            }
        });

        tracing::info!(
            "GeoParquet source '{}': {} file(s) from {}",
            config_id,
            file_count,
            path.display()
        );

        let metadata = TileMetadata {
            id: config_id,
            name: display_name,
            description: config_description,
            attribution: config_attribution,
            format: TileFormat::Pbf,
            minzoom,
            maxzoom,
            bounds,
            center: bounds.map(|b| [(b[0] + b[2]) / 2.0, (b[1] + b[3]) / 2.0, 4.0]),
            vector_layers: Some(vector_layers),
        };

        Ok(Self {
            metadata,
            file_paths,
            geometry_column,
            has_bbox_column,
            property_fields,
        })
    }
}

fn bbox_intersects(feat_bbox: [f64; 4], tile_bbox: &[f64; 4]) -> bool {
    feat_bbox[0] <= tile_bbox[2]
        && feat_bbox[1] <= tile_bbox[3]
        && feat_bbox[2] >= tile_bbox[0]
        && feat_bbox[3] >= tile_bbox[1]
}

#[inline]
fn mvt_command(id: u32, count: u32) -> u32 {
    (id & 0x7) | (count << 3)
}

#[inline]
fn zigzag_encode(n: i32) -> u32 {
    ((n << 1) ^ (n >> 31)) as u32
}

#[inline]
fn lon_lat_to_tile_xy(lon: f64, lat: f64, tile_bbox: &[f64; 4], extent: u32) -> (i32, i32) {
    let lon_range = tile_bbox[2] - tile_bbox[0];
    let x = ((lon - tile_bbox[0]) / lon_range * extent as f64) as i32;

    // Y: latitude must use Mercator (logarithmic) projection.
    // Linear interpolation of WGS84 degrees causes large position errors at low zoom levels.
    // The tile bbox latitudes from tile_to_bbox are inverse-Mercator of tile edges, so we
    // round-trip them back to Mercator Y for correct linear interpolation in projected space.
    #[inline]
    fn lat_to_merc(lat_deg: f64) -> f64 {
        let lat_rad = lat_deg.to_radians();
        (std::f64::consts::FRAC_PI_4 + lat_rad / 2.0).tan().ln()
    }

    let merc_north = lat_to_merc(tile_bbox[3]);
    let merc_south = lat_to_merc(tile_bbox[1]);
    let merc_lat = lat_to_merc(lat);

    // Tile Y increases downward; Mercator Y increases northward — invert.
    let y = ((merc_north - merc_lat) / (merc_north - merc_south) * extent as f64) as i32;
    (x, y)
}

fn read_f64_le(buf: &[u8], offset: usize) -> f64 {
    f64::from_le_bytes([
        buf[offset],
        buf[offset + 1],
        buf[offset + 2],
        buf[offset + 3],
        buf[offset + 4],
        buf[offset + 5],
        buf[offset + 6],
        buf[offset + 7],
    ])
}

#[inline]
fn read_u32_le(buf: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        buf[offset],
        buf[offset + 1],
        buf[offset + 2],
        buf[offset + 3],
    ])
}

fn wkb_point_to_mvt(wkb: &[u8], tile_bbox: &[f64; 4]) -> Option<(Vec<u32>, u32)> {
    if wkb.len() < 21 {
        return None;
    }
    let lon = read_f64_le(wkb, 5);
    let lat = read_f64_le(wkb, 13);
    let (x, y) = lon_lat_to_tile_xy(lon, lat, tile_bbox, MVT_EXTENT);
    let geom = vec![mvt_command(1, 1), zigzag_encode(x), zigzag_encode(y)];
    Some((geom, 1)) // GeomType::Point = 1
}

/// Encode a WKB LineString into MVT geometry commands.
/// Returns (geometry_commands, geom_type=2) or None if invalid.
fn wkb_linestring_to_mvt(wkb: &[u8], tile_bbox: &[f64; 4]) -> Option<(Vec<u32>, u32)> {
    // Header: 1 (byte order) + 4 (type) + 4 (num_points) = 9 bytes minimum
    if wkb.len() < 9 {
        return None;
    }
    let num_points = read_u32_le(wkb, 5) as usize;
    if num_points < 2 {
        return None;
    }
    // Each point is 16 bytes (two f64s)
    let required = 9 + num_points * 16;
    if wkb.len() < required {
        return None;
    }

    let mut geom: Vec<u32> = Vec::with_capacity(3 + num_points * 2);
    let mut cursor_x = 0i32;
    let mut cursor_y = 0i32;

    let lon = read_f64_le(wkb, 9);
    let lat = read_f64_le(wkb, 17);
    let (x0, y0) = lon_lat_to_tile_xy(lon, lat, tile_bbox, MVT_EXTENT);
    geom.push(mvt_command(1, 1));
    geom.push(zigzag_encode(x0 - cursor_x));
    geom.push(zigzag_encode(y0 - cursor_y));
    cursor_x = x0;
    cursor_y = y0;

    geom.push(mvt_command(2, (num_points - 1) as u32));
    for i in 1..num_points {
        let offset = 9 + i * 16;
        let lon = read_f64_le(wkb, offset);
        let lat = read_f64_le(wkb, offset + 8);
        let (px, py) = lon_lat_to_tile_xy(lon, lat, tile_bbox, MVT_EXTENT);
        geom.push(zigzag_encode(px - cursor_x));
        geom.push(zigzag_encode(py - cursor_y));
        cursor_x = px;
        cursor_y = py;
    }

    Some((geom, 2)) // GeomType::LineString = 2
}

/// Encode a single polygon ring (starting at `ring_start` offset in `wkb`) into MVT commands.
/// `cursor_x/cursor_y` are updated in-place to track delta encoding state.
/// Returns the number of bytes consumed for this ring, or None if invalid.
fn encode_wkb_ring(
    wkb: &[u8],
    ring_start: usize,
    tile_bbox: &[f64; 4],
    cursor_x: &mut i32,
    cursor_y: &mut i32,
    geom: &mut Vec<u32>,
) -> Option<usize> {
    if wkb.len() < ring_start + 4 {
        return None;
    }
    let num_points = read_u32_le(wkb, ring_start) as usize;
    // A valid ring has ≥4 WKB points (3 unique vertices + 1 closing duplicate)
    if num_points < 4 {
        return None;
    }
    let ring_data_len = 4 + num_points * 16;
    if wkb.len() < ring_start + ring_data_len {
        return None;
    }

    let coord_start = ring_start + 4;

    let lon0 = read_f64_le(wkb, coord_start);
    let lat0 = read_f64_le(wkb, coord_start + 8);
    let (x0, y0) = lon_lat_to_tile_xy(lon0, lat0, tile_bbox, MVT_EXTENT);
    let move_x = x0;
    let move_y = y0;

    geom.push(mvt_command(1, 1));
    geom.push(zigzag_encode(x0 - *cursor_x));
    geom.push(zigzag_encode(y0 - *cursor_y));
    *cursor_x = x0;
    *cursor_y = y0;

    // num_points - 2: skip the first point (already in MoveTo) and last (WKB closing duplicate)
    let line_count = num_points - 2;
    geom.push(mvt_command(2, line_count as u32));
    for i in 1..=line_count {
        let offset = coord_start + i * 16;
        let lon = read_f64_le(wkb, offset);
        let lat = read_f64_le(wkb, offset + 8);
        let (px, py) = lon_lat_to_tile_xy(lon, lat, tile_bbox, MVT_EXTENT);
        geom.push(zigzag_encode(px - *cursor_x));
        geom.push(zigzag_encode(py - *cursor_y));
        *cursor_x = px;
        *cursor_y = py;
    }

    geom.push(mvt_command(7, 1));
    // Per MVT spec, delta cursor resets to the ring's MoveTo position after ClosePath
    *cursor_x = move_x;
    *cursor_y = move_y;

    Some(ring_data_len)
}

/// Encode a WKB Polygon into MVT geometry commands.
/// Returns (geometry_commands, geom_type=3) or None if invalid.
fn wkb_polygon_to_mvt(wkb: &[u8], tile_bbox: &[f64; 4]) -> Option<(Vec<u32>, u32)> {
    // Header: 1 (byte order) + 4 (type) + 4 (num_rings) = 9 bytes minimum
    if wkb.len() < 9 {
        return None;
    }
    let num_rings = read_u32_le(wkb, 5) as usize;
    if num_rings == 0 {
        return None;
    }

    let mut geom: Vec<u32> = Vec::with_capacity(num_rings * 10);
    let mut cursor_x = 0i32;
    let mut cursor_y = 0i32;
    let mut offset = 9;

    for _ in 0..num_rings {
        let consumed = encode_wkb_ring(
            wkb,
            offset,
            tile_bbox,
            &mut cursor_x,
            &mut cursor_y,
            &mut geom,
        )?;
        offset += consumed;
    }

    Some((geom, 3)) // GeomType::Polygon = 3
}

fn wkb_multipoint_to_mvt(wkb: &[u8], tile_bbox: &[f64; 4]) -> Option<(Vec<u32>, u32)> {
    if wkb.len() < 9 {
        return None;
    }
    let n_geoms = read_u32_le(wkb, 5) as usize;
    if n_geoms == 0 {
        return None;
    }
    // Each sub-WKB Point is 21 bytes: 1 (byte order) + 4 (type) + 8 (X) + 8 (Y)
    if wkb.len() < 9 + n_geoms * 21 {
        return None;
    }

    // MVT encodes MultiPoint as a single MoveTo(n) command followed by n delta pairs
    let mut geom: Vec<u32> = Vec::with_capacity(1 + n_geoms * 2);
    geom.push(mvt_command(1, n_geoms as u32));

    let mut cursor_x = 0i32;
    let mut cursor_y = 0i32;

    for i in 0..n_geoms {
        let sub_offset = 9 + i * 21;
        let lon = read_f64_le(wkb, sub_offset + 5);
        let lat = read_f64_le(wkb, sub_offset + 13);
        let (px, py) = lon_lat_to_tile_xy(lon, lat, tile_bbox, MVT_EXTENT);
        geom.push(zigzag_encode(px - cursor_x));
        geom.push(zigzag_encode(py - cursor_y));
        cursor_x = px;
        cursor_y = py;
    }

    Some((geom, 1))
}

fn wkb_multilinestring_to_mvt(wkb: &[u8], tile_bbox: &[f64; 4]) -> Option<(Vec<u32>, u32)> {
    if wkb.len() < 9 {
        return None;
    }
    let n_geoms = read_u32_le(wkb, 5) as usize;
    if n_geoms == 0 {
        return None;
    }

    let mut geom: Vec<u32> = Vec::new();
    let mut cursor_x = 0i32;
    let mut cursor_y = 0i32;
    let mut offset = 9;

    for _ in 0..n_geoms {
        if wkb.len() < offset + 9 {
            return None;
        }
        let num_points = read_u32_le(wkb, offset + 5) as usize;
        if num_points < 2 {
            return None;
        }
        let sub_required = 9 + num_points * 16;
        if wkb.len() < offset + sub_required {
            return None;
        }

        let coord_start = offset + 9;

        let lon0 = read_f64_le(wkb, coord_start);
        let lat0 = read_f64_le(wkb, coord_start + 8);
        let (x0, y0) = lon_lat_to_tile_xy(lon0, lat0, tile_bbox, MVT_EXTENT);
        geom.push(mvt_command(1, 1));
        geom.push(zigzag_encode(x0 - cursor_x));
        geom.push(zigzag_encode(y0 - cursor_y));
        cursor_x = x0;
        cursor_y = y0;

        geom.push(mvt_command(2, (num_points - 1) as u32));
        for i in 1..num_points {
            let pt_offset = coord_start + i * 16;
            let lon = read_f64_le(wkb, pt_offset);
            let lat = read_f64_le(wkb, pt_offset + 8);
            let (px, py) = lon_lat_to_tile_xy(lon, lat, tile_bbox, MVT_EXTENT);
            geom.push(zigzag_encode(px - cursor_x));
            geom.push(zigzag_encode(py - cursor_y));
            cursor_x = px;
            cursor_y = py;
        }

        offset += sub_required;
    }

    Some((geom, 2))
}

fn wkb_multipolygon_to_mvt(wkb: &[u8], tile_bbox: &[f64; 4]) -> Option<(Vec<u32>, u32)> {
    if wkb.len() < 9 {
        return None;
    }
    let n_geoms = read_u32_le(wkb, 5) as usize;
    if n_geoms == 0 {
        return None;
    }

    let mut geom: Vec<u32> = Vec::new();
    let mut cursor_x = 0i32;
    let mut cursor_y = 0i32;
    let mut offset = 9;

    for _ in 0..n_geoms {
        if wkb.len() < offset + 9 {
            return None;
        }
        let num_rings = read_u32_le(wkb, offset + 5) as usize;
        if num_rings == 0 {
            return None;
        }
        let ring_offset_start = offset + 9;
        let mut ring_offset = ring_offset_start;

        for _ in 0..num_rings {
            let consumed = encode_wkb_ring(
                wkb,
                ring_offset,
                tile_bbox,
                &mut cursor_x,
                &mut cursor_y,
                &mut geom,
            )?;
            ring_offset += consumed;
        }

        offset = ring_offset;
    }

    Some((geom, 3))
}

fn wkb_to_mvt(wkb: &[u8], tile_bbox: &[f64; 4]) -> Option<(Vec<u32>, u32)> {
    if wkb.len() < 5 {
        return None;
    }
    // bytes 1-4 are the WKB geometry type (u32 LE); byte 0 is the byte-order marker
    let geom_type = read_u32_le(wkb, 1);
    match geom_type {
        1 => wkb_point_to_mvt(wkb, tile_bbox),
        2 => wkb_linestring_to_mvt(wkb, tile_bbox),
        3 => wkb_polygon_to_mvt(wkb, tile_bbox),
        4 => wkb_multipoint_to_mvt(wkb, tile_bbox),
        5 => wkb_multilinestring_to_mvt(wkb, tile_bbox),
        6 => wkb_multipolygon_to_mvt(wkb, tile_bbox),
        _ => None,
    }
}

fn encode_mvt_tile(features: &[MvtFeature], layer_name: &str) -> Vec<u8> {
    let mut keys: Vec<String> = Vec::with_capacity(32);
    let mut key_map: HashMap<String, u32> = HashMap::with_capacity(32);
    let mut values: Vec<Vec<u8>> = Vec::with_capacity(features.len() * 4);
    let mut encoded_features: Vec<Vec<u8>> = Vec::with_capacity(features.len());

    for feat in features {
        let mut tags: Vec<u32> = Vec::new();
        for (key, val) in &feat.properties {
            let key_idx = *key_map.entry(key.clone()).or_insert_with(|| {
                let idx = keys.len() as u32;
                keys.push(key.clone());
                idx
            });
            let val_idx = values.len() as u32;
            values.push(encode_mvt_value(val));
            tags.push(key_idx);
            tags.push(val_idx);
        }

        let mut feature_buf = Vec::new();
        // field 3: geometry type (varint)
        prost::encoding::uint32::encode(3, &feat.geom_type, &mut feature_buf);
        // field 4: geometry (packed uint32)
        prost::encoding::encode_key(
            4,
            prost::encoding::WireType::LengthDelimited,
            &mut feature_buf,
        );
        let geom_len: usize = feat
            .geometry
            .iter()
            .map(|v| prost::encoding::encoded_len_varint(*v as u64))
            .sum();
        prost::encoding::encode_varint(geom_len as u64, &mut feature_buf);
        for &val in &feat.geometry {
            prost::encoding::encode_varint(val as u64, &mut feature_buf);
        }
        // field 2: tags (packed uint32)
        if !tags.is_empty() {
            prost::encoding::encode_key(
                2,
                prost::encoding::WireType::LengthDelimited,
                &mut feature_buf,
            );
            let tags_len: usize = tags
                .iter()
                .map(|v| prost::encoding::encoded_len_varint(*v as u64))
                .sum();
            prost::encoding::encode_varint(tags_len as u64, &mut feature_buf);
            for &val in &tags {
                prost::encoding::encode_varint(val as u64, &mut feature_buf);
            }
        }
        encoded_features.push(feature_buf);
    }

    let mut layer_buf = Vec::new();
    // field 15: version
    prost::encoding::uint32::encode(15, &2u32, &mut layer_buf);
    // field 1: name
    prost::encoding::string::encode(1, &layer_name.to_string(), &mut layer_buf);
    // field 5: extent
    prost::encoding::uint32::encode(5, &MVT_EXTENT, &mut layer_buf);
    // field 3: keys
    for key in &keys {
        prost::encoding::string::encode(3, key, &mut layer_buf);
    }
    // field 4: values (each is a sub-message)
    for val_bytes in &values {
        prost::encoding::encode_key(
            4,
            prost::encoding::WireType::LengthDelimited,
            &mut layer_buf,
        );
        prost::encoding::encode_varint(val_bytes.len() as u64, &mut layer_buf);
        layer_buf.extend_from_slice(val_bytes);
    }
    // field 2: features (each is a sub-message)
    for feat_bytes in &encoded_features {
        prost::encoding::encode_key(
            2,
            prost::encoding::WireType::LengthDelimited,
            &mut layer_buf,
        );
        prost::encoding::encode_varint(feat_bytes.len() as u64, &mut layer_buf);
        layer_buf.extend_from_slice(feat_bytes);
    }

    let mut tile_buf = Vec::new();
    // field 3: layers
    prost::encoding::encode_key(3, prost::encoding::WireType::LengthDelimited, &mut tile_buf);
    prost::encoding::encode_varint(layer_buf.len() as u64, &mut tile_buf);
    tile_buf.extend_from_slice(&layer_buf);

    tile_buf
}

fn encode_mvt_value(val: &PropValue) -> Vec<u8> {
    let mut buf = Vec::new();
    match val {
        PropValue::String(s) => {
            prost::encoding::string::encode(1, s, &mut buf);
        }
        PropValue::Float(f) => {
            prost::encoding::float::encode(2, f, &mut buf);
        }
        PropValue::Double(d) => {
            prost::encoding::double::encode(3, d, &mut buf);
        }
        PropValue::Int(i) => {
            prost::encoding::int64::encode(4, i, &mut buf);
        }
        PropValue::Bool(b) => {
            prost::encoding::bool::encode(7, b, &mut buf);
        }
    }
    buf
}

#[derive(Debug, Clone)]
enum PropValue {
    String(String),
    Float(f32),
    Double(f64),
    Int(i64),
    Bool(bool),
}

struct MvtFeature {
    geom_type: u32,
    geometry: Vec<u32>,
    properties: Vec<(String, PropValue)>,
}

fn extract_row_properties(
    batch: &arrow_array::RecordBatch,
    row: usize,
    property_fields: &[(String, String)],
) -> Vec<(String, PropValue)> {
    let schema = batch.schema();
    let mut props = Vec::new();

    for (name, _) in property_fields {
        let col_idx = match schema.fields().iter().position(|f| f.name() == name) {
            Some(idx) => idx,
            None => continue,
        };
        let col = batch.column(col_idx);
        if col.is_null(row) {
            continue;
        }

        let val = extract_column_value(col.as_ref(), row);
        if let Some(v) = val {
            props.push((name.clone(), v));
        }
    }
    props
}

fn extract_column_value(col: &dyn arrow_array::Array, row: usize) -> Option<PropValue> {
    use arrow_array::*;

    if col.is_null(row) {
        return None;
    }

    if let Some(arr) = col.as_any().downcast_ref::<StringArray>() {
        return Some(PropValue::String(arr.value(row).to_string()));
    }
    if let Some(arr) = col.as_any().downcast_ref::<LargeStringArray>() {
        return Some(PropValue::String(arr.value(row).to_string()));
    }
    if let Some(arr) = col.as_any().downcast_ref::<Int32Array>() {
        return Some(PropValue::Int(arr.value(row) as i64));
    }
    if let Some(arr) = col.as_any().downcast_ref::<Int64Array>() {
        return Some(PropValue::Int(arr.value(row)));
    }
    if let Some(arr) = col.as_any().downcast_ref::<Float32Array>() {
        return Some(PropValue::Float(arr.value(row)));
    }
    if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
        return Some(PropValue::Double(arr.value(row)));
    }
    if let Some(arr) = col.as_any().downcast_ref::<BooleanArray>() {
        return Some(PropValue::Bool(arr.value(row)));
    }

    None
}

#[async_trait]
impl TileSource for GeoParquetSource {
    async fn get_tile(&self, z: u8, x: u32, y: u32) -> Result<Option<TileData>> {
        if z < self.metadata.minzoom || z > self.metadata.maxzoom {
            return Ok(None);
        }

        // Buffered bbox: includes features slightly outside the tile for seamless edge rendering.
        // Unbuffered bbox: used for coordinate projection so MVT coordinates align with the tile grid.
        // Using the buffered bbox for projection causes coordinates to be compressed into a wider
        // Mercator space — at z0 the 64-pixel buffer pushes latitude from ±85° to ±87.7°,
        // which in Mercator is ~24% wider, causing visible misalignment with the basemap.
        let filter_bbox = tile_to_bbox_with_buffer(z, x, y, 64);
        let tile_bbox = tile_to_bbox(z, x, y);
        let file_paths = self.file_paths.clone();
        let geometry_column = self.geometry_column.clone();
        let has_bbox = self.has_bbox_column;
        let property_fields = self.property_fields.clone();
        let layer_name = self
            .metadata
            .vector_layers
            .as_ref()
            .and_then(|vl| vl.as_array())
            .and_then(|arr| arr.first())
            .and_then(|l| l.get("id"))
            .and_then(|id| id.as_str())
            .unwrap_or(&self.metadata.id)
            .to_string();

        let result = tokio::task::spawn_blocking(move || -> Result<Option<Vec<u8>>> {
            let mut mvt_features: Vec<MvtFeature> = Vec::with_capacity(256);

            for file_path in &file_paths {
                let file = std::fs::File::open(file_path).map_err(|e| {
                    TileServerError::GeoParquetError(format!(
                        "failed to open {}: {e}",
                        file_path.display()
                    ))
                })?;

                let builder = ParquetRecordBatchReaderBuilder::try_new(file).map_err(|e| {
                    TileServerError::GeoParquetError(format!("failed to read parquet: {e}"))
                })?;

                let reader = builder.with_batch_size(8192).build().map_err(|e| {
                    TileServerError::GeoParquetError(format!("failed to build reader: {e}"))
                })?;

                for batch_result in reader {
                    let batch = batch_result.map_err(|e| {
                        TileServerError::GeoParquetError(format!("failed to read batch: {e}"))
                    })?;

                    let schema = batch.schema();
                    let geom_idx = match schema
                        .fields()
                        .iter()
                        .position(|f| f.name() == &geometry_column)
                    {
                        Some(idx) => idx,
                        None => continue,
                    };

                    for row in 0..batch.num_rows() {
                        if has_bbox
                            && let Some(bbox_idx) =
                                schema.fields().iter().position(|f| f.name() == "bbox")
                            && let Some(struct_arr) = batch
                                .column(bbox_idx)
                                .as_any()
                                .downcast_ref::<arrow_array::StructArray>()
                        {
                            let xmin = struct_arr.column_by_name("xmin").and_then(|c| {
                                c.as_any().downcast_ref::<arrow_array::Float64Array>()
                            });
                            let xmax = struct_arr.column_by_name("xmax").and_then(|c| {
                                c.as_any().downcast_ref::<arrow_array::Float64Array>()
                            });
                            let ymin = struct_arr.column_by_name("ymin").and_then(|c| {
                                c.as_any().downcast_ref::<arrow_array::Float64Array>()
                            });
                            let ymax = struct_arr.column_by_name("ymax").and_then(|c| {
                                c.as_any().downcast_ref::<arrow_array::Float64Array>()
                            });

                            if let (Some(xmin), Some(xmax), Some(ymin), Some(ymax)) =
                                (xmin, xmax, ymin, ymax)
                            {
                                let feat_bbox = [
                                    xmin.value(row),
                                    ymin.value(row),
                                    xmax.value(row),
                                    ymax.value(row),
                                ];
                                if !bbox_intersects(feat_bbox, &filter_bbox) {
                                    continue;
                                }
                            }
                        }

                        let geom_col = batch.column(geom_idx);
                        let wkb: Option<&[u8]> = geom_col
                            .as_any()
                            .downcast_ref::<arrow_array::BinaryArray>()
                            .and_then(|arr| {
                                if arr.is_null(row) {
                                    None
                                } else {
                                    Some(arr.value(row))
                                }
                            })
                            .or_else(|| {
                                geom_col
                                    .as_any()
                                    .downcast_ref::<arrow_array::LargeBinaryArray>()
                                    .and_then(|arr| {
                                        if arr.is_null(row) {
                                            None
                                        } else {
                                            Some(arr.value(row))
                                        }
                                    })
                            });

                        let wkb = match wkb {
                            Some(d) if !d.is_empty() => d,
                            _ => continue,
                        };

                        if let Some((geom, geom_type)) = wkb_to_mvt(wkb, &tile_bbox) {
                            let props = extract_row_properties(&batch, row, &property_fields);
                            mvt_features.push(MvtFeature {
                                geom_type,
                                geometry: geom,
                                properties: props,
                            });
                        }
                    }
                }
            }

            if mvt_features.is_empty() {
                return Ok(None);
            }

            Ok(Some(encode_mvt_tile(&mvt_features, &layer_name)))
        })
        .await
        .map_err(|e| TileServerError::GeoParquetError(format!("task join error: {e}")))?;

        match result? {
            Some(data) => Ok(Some(TileData {
                data: Bytes::from(data),
                format: TileFormat::Pbf,
                compression: TileCompression::None,
            })),
            None => Ok(None),
        }
    }

    fn metadata(&self) -> &TileMetadata {
        &self.metadata
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_to_bbox_zoom_0() {
        let bbox = tile_to_bbox(0, 0, 0);
        assert!((bbox[0] - (-180.0)).abs() < 0.001);
        assert!((bbox[2] - 180.0).abs() < 0.001);
        assert!(bbox[1] < -80.0);
        assert!(bbox[3] > 80.0);
    }

    #[test]
    fn test_tile_to_bbox_zoom_1() {
        let bbox = tile_to_bbox(1, 0, 0);
        assert!((bbox[0] - (-180.0)).abs() < 0.001);
        assert!((bbox[2] - 0.0).abs() < 0.001);
        assert!(bbox[3] > 60.0);
    }

    #[test]
    fn test_tile_to_bbox_zoom_14_nyc() {
        let bbox = tile_to_bbox(14, 4824, 6156);
        assert!(bbox[0] > -74.1 && bbox[0] < -73.9);
        assert!(bbox[2] > bbox[0]);
        assert!(bbox[3] > bbox[1]);
    }

    #[test]
    fn test_tile_to_bbox_ordering() {
        for z in 0..=18 {
            let bbox = tile_to_bbox(z, 0, 0);
            assert!(bbox[0] < bbox[2], "lon_min < lon_max at z={z}");
            assert!(bbox[1] < bbox[3], "lat_min < lat_max at z={z}");
        }
    }

    #[test]
    fn test_tile_to_bbox_with_buffer_expands() {
        let bbox = tile_to_bbox(10, 512, 512);
        let buffered = tile_to_bbox_with_buffer(10, 512, 512, 64);
        assert!(buffered[0] < bbox[0]);
        assert!(buffered[1] < bbox[1]);
        assert!(buffered[2] > bbox[2]);
        assert!(buffered[3] > bbox[3]);
    }

    #[test]
    fn test_tile_to_bbox_with_zero_buffer() {
        let bbox = tile_to_bbox(10, 512, 512);
        let buffered = tile_to_bbox_with_buffer(10, 512, 512, 0);
        assert!((buffered[0] - bbox[0]).abs() < 1e-10);
        assert!((buffered[1] - bbox[1]).abs() < 1e-10);
    }

    #[test]
    fn test_bbox_intersects_overlap() {
        assert!(bbox_intersects(
            [-74.0, 40.0, -73.0, 41.0],
            &[-74.5, 40.5, -73.5, 41.5]
        ));
    }

    #[test]
    fn test_bbox_intersects_no_overlap() {
        assert!(!bbox_intersects(
            [10.0, 10.0, 11.0, 11.0],
            &[-74.5, 40.5, -73.5, 41.5]
        ));
    }

    #[test]
    fn test_bbox_intersects_contained() {
        assert!(bbox_intersects(
            [-74.0, 40.5, -73.5, 41.0],
            &[-75.0, 40.0, -73.0, 42.0]
        ));
    }

    #[test]
    fn test_zigzag_encode() {
        assert_eq!(zigzag_encode(0), 0);
        assert_eq!(zigzag_encode(-1), 1);
        assert_eq!(zigzag_encode(1), 2);
        assert_eq!(zigzag_encode(-2), 3);
    }

    #[test]
    fn test_mvt_command() {
        assert_eq!(mvt_command(1, 1), 9); // MoveTo(1)
        assert_eq!(mvt_command(2, 3), 26); // LineTo(3)
        assert_eq!(mvt_command(7, 1), 15); // ClosePath(1)
    }

    #[test]
    fn test_lon_lat_to_tile_xy_center() {
        let (x, y) = lon_lat_to_tile_xy(0.5, 0.5, &[0.0, 0.0, 1.0, 1.0], 4096);
        assert_eq!(x, 2048);
        assert_eq!(y, 2048);
    }

    #[test]
    fn test_lon_lat_to_tile_xy_origin() {
        let (x, y) = lon_lat_to_tile_xy(0.0, 1.0, &[0.0, 0.0, 1.0, 1.0], 4096);
        assert_eq!(x, 0);
        assert_eq!(y, 0);
    }

    #[test]
    fn test_lon_lat_to_tile_xy_mercator_z1() {
        let bbox = tile_to_bbox(1, 0, 0);
        let lat_mid = f64::atan(f64::sinh(std::f64::consts::FRAC_PI_2)).to_degrees();
        let lon_mid = (bbox[0] + bbox[2]) / 2.0;
        let (x, y) = lon_lat_to_tile_xy(lon_mid, lat_mid, &bbox, 4096);
        assert_eq!(x, 2048);
        assert!((y - 2048).abs() <= 1, "expected y≈2048, got {y}");
    }

    #[test]
    fn test_wkb_point_to_mvt() {
        let mut wkb = vec![1u8]; // little-endian
        wkb.extend_from_slice(&1u32.to_le_bytes()); // Point
        wkb.extend_from_slice(&0.5f64.to_le_bytes()); // lon
        wkb.extend_from_slice(&0.5f64.to_le_bytes()); // lat

        let result = wkb_point_to_mvt(&wkb, &[0.0, 0.0, 1.0, 1.0]);
        assert!(result.is_some());
        let (geom, geom_type) = result.unwrap();
        assert_eq!(geom_type, 1); // Point
        assert_eq!(geom.len(), 3); // command + x + y
        assert_eq!(geom[0], mvt_command(1, 1));
    }

    #[test]
    fn test_wkb_point_to_mvt_empty() {
        assert!(wkb_point_to_mvt(&[], &[0.0, 0.0, 1.0, 1.0]).is_none());
    }

    #[test]
    fn test_wkb_point_to_mvt_too_short() {
        assert!(wkb_point_to_mvt(&[1, 0, 0, 0], &[0.0, 0.0, 1.0, 1.0]).is_none());
    }

    #[test]
    fn test_extract_property_fields_filters_correctly() {
        use arrow_schema::{Field, Schema};
        let schema = Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("name", DataType::Utf8, true),
            Field::new("geometry", DataType::Binary, false),
            Field::new("bbox", DataType::Utf8, true),
            Field::new("height", DataType::Float64, true),
        ]);
        let fields = extract_property_fields(&schema, "geometry");
        let names: Vec<&str> = fields.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"id"));
        assert!(names.contains(&"name"));
        assert!(names.contains(&"height"));
        assert!(!names.contains(&"geometry"));
        assert!(!names.contains(&"bbox"));
    }

    #[test]
    fn test_extract_property_fields_type_mapping() {
        use arrow_schema::{Field, Schema};
        let schema = Schema::new(vec![
            Field::new("name", DataType::Utf8, true),
            Field::new("count", DataType::Int32, false),
            Field::new("area", DataType::Float64, true),
            Field::new("active", DataType::Boolean, false),
        ]);
        let fields = extract_property_fields(&schema, "geometry");
        let map: HashMap<&str, &str> = fields
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        assert_eq!(map["name"], "String");
        assert_eq!(map["count"], "Number");
        assert_eq!(map["area"], "Number");
        assert_eq!(map["active"], "Boolean");
    }

    #[test]
    fn test_encode_mvt_value_string() {
        let bytes = encode_mvt_value(&PropValue::String("hello".to_string()));
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_encode_mvt_value_int() {
        let bytes = encode_mvt_value(&PropValue::Int(42));
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_encode_mvt_tile_basic() {
        let features = vec![MvtFeature {
            geom_type: 1,
            geometry: vec![mvt_command(1, 1), zigzag_encode(100), zigzag_encode(200)],
            properties: vec![("name".to_string(), PropValue::String("test".to_string()))],
        }];
        let tile_bytes = encode_mvt_tile(&features, "test_layer");
        assert!(!tile_bytes.is_empty());
    }

    #[test]
    fn test_config_source_type_geoparquet_serde() {
        use crate::config::SourceType;
        let json = serde_json::to_string(&SourceType::GeoParquet).unwrap();
        assert_eq!(json, "\"geoparquet\"");
        let parsed: SourceType = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, SourceType::GeoParquet);
    }

    #[test]
    fn test_parse_geoparquet_config() {
        let toml_str = r#"
            [server]
            host = "127.0.0.1"
            port = 3000

            [[sources]]
            id = "buildings"
            type = "geoparquet"
            path = "/data/buildings.parquet"
            name = "Overture Buildings"
            layer_name = "buildings"
            geometry_column = "geometry"
            minzoom = 0
            maxzoom = 14
        "#;

        let config: crate::config::Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.sources.len(), 1);
        assert_eq!(
            config.sources[0].source_type,
            crate::config::SourceType::GeoParquet
        );
        assert_eq!(config.sources[0].id, "buildings");
        assert_eq!(config.sources[0].layer_name, Some("buildings".to_string()));
        assert_eq!(
            config.sources[0].geometry_column,
            Some("geometry".to_string())
        );
        assert_eq!(config.sources[0].minzoom, Some(0));
        assert_eq!(config.sources[0].maxzoom, Some(14));
    }

    #[test]
    fn test_parse_geoparquet_config_minimal() {
        let toml_str = r#"
            [[sources]]
            id = "data"
            type = "geoparquet"
            path = "/data/data.parquet"
        "#;

        let config: crate::config::Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.sources.len(), 1);
        assert_eq!(
            config.sources[0].source_type,
            crate::config::SourceType::GeoParquet
        );
        assert!(config.sources[0].layer_name.is_none());
        assert!(config.sources[0].geometry_column.is_none());
        assert!(config.sources[0].minzoom.is_none());
        assert!(config.sources[0].maxzoom.is_none());
    }

    fn make_wkb_linestring(coords: &[(f64, f64)]) -> Vec<u8> {
        let mut wkb = vec![1u8];
        wkb.extend_from_slice(&2u32.to_le_bytes());
        wkb.extend_from_slice(&(coords.len() as u32).to_le_bytes());
        for (lon, lat) in coords {
            wkb.extend_from_slice(&lon.to_le_bytes());
            wkb.extend_from_slice(&lat.to_le_bytes());
        }
        wkb
    }

    fn make_wkb_polygon(rings: &[Vec<(f64, f64)>]) -> Vec<u8> {
        let mut wkb = vec![1u8];
        wkb.extend_from_slice(&3u32.to_le_bytes());
        wkb.extend_from_slice(&(rings.len() as u32).to_le_bytes());
        for ring in rings {
            wkb.extend_from_slice(&(ring.len() as u32).to_le_bytes());
            for (lon, lat) in ring {
                wkb.extend_from_slice(&lon.to_le_bytes());
                wkb.extend_from_slice(&lat.to_le_bytes());
            }
        }
        wkb
    }

    #[test]
    fn test_wkb_linestring_two_points() {
        let wkb = make_wkb_linestring(&[(0.0, 1.0), (1.0, 0.0)]);
        let result = wkb_linestring_to_mvt(&wkb, &[0.0, 0.0, 1.0, 1.0]);
        assert!(result.is_some());
        let (geom, geom_type) = result.unwrap();
        assert_eq!(geom_type, 2);
        assert_eq!(geom[0], mvt_command(1, 1));
        assert_eq!(geom[3], mvt_command(2, 1));
        assert_eq!(geom.len(), 6);
    }

    #[test]
    fn test_wkb_linestring_too_short() {
        assert!(wkb_linestring_to_mvt(&[1, 2, 0, 0, 0], &[0.0, 0.0, 1.0, 1.0]).is_none());
    }

    #[test]
    fn test_wkb_linestring_single_point_rejected() {
        let wkb = make_wkb_linestring(&[(0.5, 0.5)]);
        assert!(wkb_linestring_to_mvt(&wkb, &[0.0, 0.0, 1.0, 1.0]).is_none());
    }

    #[test]
    fn test_wkb_polygon_simple_square() {
        let ring = vec![(0.1, 0.1), (0.9, 0.1), (0.9, 0.9), (0.1, 0.9), (0.1, 0.1)];
        let wkb = make_wkb_polygon(&[ring]);
        let result = wkb_polygon_to_mvt(&wkb, &[0.0, 0.0, 1.0, 1.0]);
        assert!(result.is_some());
        let (geom, geom_type) = result.unwrap();
        assert_eq!(geom_type, 3);
        assert_eq!(geom[0], mvt_command(1, 1));
        assert!(geom.contains(&mvt_command(7, 1)));
    }

    #[test]
    fn test_wkb_polygon_too_few_ring_points() {
        let ring = vec![(0.0, 0.0), (1.0, 0.0), (0.5, 0.5)];
        let wkb = make_wkb_polygon(&[ring]);
        assert!(wkb_polygon_to_mvt(&wkb, &[0.0, 0.0, 1.0, 1.0]).is_none());
    }

    #[test]
    fn test_wkb_to_mvt_dispatches_point() {
        let mut wkb = vec![1u8];
        wkb.extend_from_slice(&1u32.to_le_bytes());
        wkb.extend_from_slice(&0.5f64.to_le_bytes());
        wkb.extend_from_slice(&0.5f64.to_le_bytes());
        let (_, geom_type) = wkb_to_mvt(&wkb, &[0.0, 0.0, 1.0, 1.0]).unwrap();
        assert_eq!(geom_type, 1);
    }

    #[test]
    fn test_wkb_to_mvt_dispatches_linestring() {
        let wkb = make_wkb_linestring(&[(0.0, 1.0), (1.0, 0.0)]);
        let (_, geom_type) = wkb_to_mvt(&wkb, &[0.0, 0.0, 1.0, 1.0]).unwrap();
        assert_eq!(geom_type, 2);
    }

    #[test]
    fn test_wkb_to_mvt_dispatches_polygon() {
        let ring = vec![(0.1, 0.1), (0.9, 0.1), (0.9, 0.9), (0.1, 0.9), (0.1, 0.1)];
        let wkb = make_wkb_polygon(&[ring]);
        let (_, geom_type) = wkb_to_mvt(&wkb, &[0.0, 0.0, 1.0, 1.0]).unwrap();
        assert_eq!(geom_type, 3);
    }

    #[test]
    fn test_wkb_to_mvt_unknown_type_returns_none() {
        let mut wkb = vec![1u8];
        wkb.extend_from_slice(&99u32.to_le_bytes());
        wkb.extend_from_slice(&[0u8; 20]);
        assert!(wkb_to_mvt(&wkb, &[0.0, 0.0, 1.0, 1.0]).is_none());
    }

    #[test]
    fn test_wkb_to_mvt_too_short_returns_none() {
        assert!(wkb_to_mvt(&[1, 0, 0], &[0.0, 0.0, 1.0, 1.0]).is_none());
    }

    #[test]
    fn test_wkb_multipolygon_dispatches() {
        let ring = vec![(0.1, 0.1), (0.9, 0.1), (0.9, 0.9), (0.1, 0.9), (0.1, 0.1)];
        let poly_wkb = make_wkb_polygon(std::slice::from_ref(&ring));
        let mut wkb = vec![1u8];
        wkb.extend_from_slice(&6u32.to_le_bytes());
        wkb.extend_from_slice(&1u32.to_le_bytes());
        wkb.extend_from_slice(&poly_wkb);
        let result = wkb_to_mvt(&wkb, &[0.0, 0.0, 1.0, 1.0]);
        assert!(result.is_some());
        let (_, geom_type) = result.unwrap();
        assert_eq!(geom_type, 3);
    }
}
