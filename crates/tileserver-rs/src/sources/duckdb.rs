//! DuckDB spatial query tile source with SQL template expansion.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use bytes::Bytes;
use duckdb::Connection;

use crate::config::SourceConfig;
use crate::error::{Result, TileServerError};
use crate::sources::{TileCompression, TileData, TileFormat, TileMetadata, TileSource};

const DEFAULT_MIN_ZOOM: u8 = 0;
const DEFAULT_MAX_ZOOM: u8 = 22;
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

#[must_use]
pub fn substitute_template(query: &str, z: u8, x: u32, y: u32, bbox: &[f64; 4]) -> String {
    query
        .replace("{z}", &z.to_string())
        .replace("{x}", &x.to_string())
        .replace("{y}", &y.to_string())
        .replace(
            "{bbox}",
            &format!("{}, {}, {}, {}", bbox[0], bbox[1], bbox[2], bbox[3]),
        )
        .replace("{bbox_xmin}", &bbox[0].to_string())
        .replace("{bbox_ymin}", &bbox[1].to_string())
        .replace("{bbox_xmax}", &bbox[2].to_string())
        .replace("{bbox_ymax}", &bbox[3].to_string())
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
    let lat_range = tile_bbox[3] - tile_bbox[1];
    let x = ((lon - tile_bbox[0]) / lon_range * extent as f64) as i32;
    let y = ((tile_bbox[3] - lat) / lat_range * extent as f64) as i32;
    (x, y)
}

#[derive(Debug, Clone)]
enum PropValue {
    String(String),
    Int(i64),
    Double(f64),
    Bool(bool),
}

struct MvtFeature {
    geom_type: u32,
    geometry: Vec<u32>,
    properties: Vec<(String, PropValue)>,
}

fn encode_mvt_value(val: &PropValue) -> Vec<u8> {
    let mut buf = Vec::with_capacity(16);
    match val {
        PropValue::String(s) => prost::encoding::string::encode(1, s, &mut buf),
        PropValue::Double(d) => prost::encoding::double::encode(3, d, &mut buf),
        PropValue::Int(i) => prost::encoding::int64::encode(4, i, &mut buf),
        PropValue::Bool(b) => prost::encoding::bool::encode(7, b, &mut buf),
    }
    buf
}

fn encode_mvt_tile(features: &[MvtFeature], layer_name: &str) -> Vec<u8> {
    let mut keys: Vec<String> = Vec::with_capacity(32);
    let mut key_map: HashMap<String, u32> = HashMap::with_capacity(32);
    let mut values: Vec<Vec<u8>> = Vec::with_capacity(features.len() * 4);
    let mut encoded_features: Vec<Vec<u8>> = Vec::with_capacity(features.len());

    for feat in features {
        let mut tags: Vec<u32> = Vec::with_capacity(feat.properties.len() * 2);
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

        let mut feature_buf = Vec::with_capacity(64);
        // field 3: geometry type
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

    let mut layer_buf = Vec::with_capacity(256);
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
    // field 4: values
    for val_bytes in &values {
        prost::encoding::encode_key(
            4,
            prost::encoding::WireType::LengthDelimited,
            &mut layer_buf,
        );
        prost::encoding::encode_varint(val_bytes.len() as u64, &mut layer_buf);
        layer_buf.extend_from_slice(val_bytes);
    }
    // field 2: features
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
    Some((
        vec![mvt_command(1, 1), zigzag_encode(x), zigzag_encode(y)],
        1,
    ))
}

fn wkb_linestring_to_mvt(wkb: &[u8], tile_bbox: &[f64; 4]) -> Option<(Vec<u32>, u32)> {
    if wkb.len() < 9 {
        return None;
    }
    let num_points = read_u32_le(wkb, 5) as usize;
    if num_points < 2 {
        return None;
    }
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

    Some((geom, 2))
}

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

    geom.push(mvt_command(1, 1));
    geom.push(zigzag_encode(x0 - *cursor_x));
    geom.push(zigzag_encode(y0 - *cursor_y));
    *cursor_x = x0;
    *cursor_y = y0;

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

    // MVT 2.1 §4.3.3.3: ClosePath does not move the cursor; the next

    // MoveTo is relative to the last LineTo vertex.

    geom.push(mvt_command(7, 1));

    Some(ring_data_len)
}

fn wkb_polygon_to_mvt(wkb: &[u8], tile_bbox: &[f64; 4]) -> Option<(Vec<u32>, u32)> {
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

    Some((geom, 3))
}

fn wkb_multipoint_to_mvt(wkb: &[u8], tile_bbox: &[f64; 4]) -> Option<(Vec<u32>, u32)> {
    if wkb.len() < 9 {
        return None;
    }
    let n_geoms = read_u32_le(wkb, 5) as usize;
    if n_geoms == 0 {
        return None;
    }
    if wkb.len() < 9 + n_geoms * 21 {
        return None;
    }

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

pub struct DuckDbSource {
    metadata: TileMetadata,
    conn: Arc<Mutex<Connection>>,
    query_template: String,
    layer_name: String,
}

impl DuckDbSource {
    pub async fn from_config(config: &SourceConfig) -> Result<Self> {
        let query = config.query.clone().ok_or_else(|| {
            TileServerError::DuckDbError(
                "DuckDB source requires a 'query' field with SQL template".to_string(),
            )
        })?;

        let db_path = if config.path.is_empty() {
            None
        } else {
            Some(config.path.clone())
        };

        let config_id = config.id.clone();
        let config_name = config.name.clone();
        let config_description = config.description.clone();
        let config_attribution = config.attribution.clone();
        let config_layer_name = config.layer_name.clone();
        let config_minzoom = config.minzoom;
        let config_maxzoom = config.maxzoom;

        let conn = tokio::task::spawn_blocking(move || -> Result<Connection> {
            let conn = match &db_path {
                Some(path) => Connection::open(path).map_err(|e| {
                    TileServerError::DuckDbError(format!("failed to open database: {e}"))
                })?,
                None => Connection::open_in_memory().map_err(|e| {
                    TileServerError::DuckDbError(format!("failed to create in-memory db: {e}"))
                })?,
            };

            if let Err(e) = conn.execute_batch("INSTALL spatial; LOAD spatial;") {
                tracing::warn!("DuckDB spatial extension not available: {e}");
            }

            Ok(conn)
        })
        .await
        .map_err(|e| TileServerError::DuckDbError(format!("task join error: {e}")))??;

        let layer_name = config_layer_name.unwrap_or_else(|| config_id.clone());
        let minzoom = config_minzoom.unwrap_or(DEFAULT_MIN_ZOOM);
        let maxzoom = config_maxzoom.unwrap_or(DEFAULT_MAX_ZOOM);

        let metadata = TileMetadata {
            id: config_id,
            name: config_name.unwrap_or_else(|| "duckdb".to_string()),
            description: config_description,
            attribution: config_attribution,
            format: TileFormat::Pbf,
            minzoom,
            maxzoom,
            bounds: Some([-180.0, -85.0, 180.0, 85.0]),
            center: Some([0.0, 0.0, 2.0]),
            vector_layers: Some(serde_json::json!([{
                "id": layer_name,
                "fields": {},
                "minzoom": minzoom,
                "maxzoom": maxzoom,
            }])),
        };

        Ok(Self {
            metadata,
            conn: Arc::new(Mutex::new(conn)),
            query_template: query,
            layer_name,
        })
    }
}

#[async_trait]
impl TileSource for DuckDbSource {
    async fn get_tile(&self, z: u8, x: u32, y: u32) -> Result<Option<TileData>> {
        if z < self.metadata.minzoom || z > self.metadata.maxzoom {
            return Ok(None);
        }

        let tile_bbox = tile_to_bbox(z, x, y);
        let sql = substitute_template(&self.query_template, z, x, y, &tile_bbox);
        let conn = self.conn.clone();
        let layer_name = self.layer_name.clone();

        let result = tokio::task::spawn_blocking(move || -> Result<Option<Vec<u8>>> {
            let conn = conn.lock().map_err(|e| {
                TileServerError::DuckDbError(format!("connection lock poisoned: {e}"))
            })?;

            let mut stmt = conn.prepare(&sql).map_err(|e| {
                TileServerError::DuckDbError(format!("query preparation failed: {e}"))
            })?;

            let mut features: Vec<MvtFeature> = Vec::new();

            let mut rows = stmt.query([]).map_err(|e| {
                TileServerError::DuckDbError(format!("query execution failed: {e}"))
            })?;

            let column_count = rows.as_ref().map(|s| s.column_count()).unwrap_or(0);
            let column_names: Vec<String> = rows
                .as_ref()
                .map(|s| {
                    (0..column_count)
                        .map(|i| {
                            s.column_name(i)
                                .map(|n| n.to_string())
                                .unwrap_or_else(|_| format!("col_{i}"))
                        })
                        .collect()
                })
                .unwrap_or_default();

            while let Some(row) = rows
                .next()
                .map_err(|e| TileServerError::DuckDbError(format!("row read failed: {e}")))?
            {
                let mut geometry_wkb: Option<Vec<u8>> = None;
                let mut properties: Vec<(String, PropValue)> = Vec::new();

                for (i, col_name) in column_names.iter().enumerate() {
                    let lower = col_name.to_lowercase();
                    if lower == "geom" || lower == "geometry" || lower == "wkb_geometry" {
                        if let Ok(blob) = row.get::<_, Vec<u8>>(i) {
                            geometry_wkb = Some(blob);
                        }
                        continue;
                    }

                    if let Ok(val) = row.get::<_, String>(i) {
                        properties.push((col_name.clone(), PropValue::String(val)));
                    } else if let Ok(val) = row.get::<_, i64>(i) {
                        properties.push((col_name.clone(), PropValue::Int(val)));
                    } else if let Ok(val) = row.get::<_, f64>(i) {
                        properties.push((col_name.clone(), PropValue::Double(val)));
                    } else if let Ok(val) = row.get::<_, bool>(i) {
                        properties.push((col_name.clone(), PropValue::Bool(val)));
                    }
                }

                let wkb = match geometry_wkb {
                    Some(ref w) if !w.is_empty() => w,
                    _ => continue,
                };

                if let Some((geom, geom_type)) = wkb_to_mvt(wkb, &tile_bbox) {
                    features.push(MvtFeature {
                        geom_type,
                        geometry: geom,
                        properties,
                    });
                }
            }

            if features.is_empty() {
                return Ok(None);
            }

            Ok(Some(encode_mvt_tile(&features, &layer_name)))
        })
        .await
        .map_err(|e| TileServerError::DuckDbError(format!("task join error: {e}")))?;

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
    fn test_tile_to_bbox_ordering() {
        for z in 0..=18 {
            let bbox = tile_to_bbox(z, 0, 0);
            assert!(bbox[0] < bbox[2]);
            assert!(bbox[1] < bbox[3]);
        }
    }

    #[test]
    fn test_substitute_template_basic() {
        let query = "SELECT * FROM t WHERE z = {z} AND x = {x} AND y = {y}";
        let result = substitute_template(query, 14, 8192, 5461, &[0.0, 0.0, 1.0, 1.0]);
        assert_eq!(
            result,
            "SELECT * FROM t WHERE z = 14 AND x = 8192 AND y = 5461"
        );
    }

    #[test]
    fn test_substitute_template_bbox() {
        let query = "WHERE ST_Intersects(geom, ST_MakeEnvelope({bbox}))";
        let result = substitute_template(query, 0, 0, 0, &[-180.0, -85.0, 180.0, 85.0]);
        assert!(result.contains("-180"));
        assert!(result.contains("180"));
        assert!(result.contains("-85"));
        assert!(result.contains("85"));
    }

    #[test]
    fn test_substitute_template_all_vars() {
        let query = "z={z} x={x} y={y} bbox={bbox}";
        let result = substitute_template(query, 5, 10, 20, &[1.0, 2.0, 3.0, 4.0]);
        assert!(result.contains("z=5"));
        assert!(result.contains("x=10"));
        assert!(result.contains("y=20"));
        assert!(result.contains("1, 2, 3, 4"));
    }

    #[test]
    fn test_substitute_template_bbox_components() {
        let query = "xmin={bbox_xmin} ymin={bbox_ymin} xmax={bbox_xmax} ymax={bbox_ymax}";
        let result = substitute_template(query, 0, 0, 0, &[-122.5, 37.7, -122.4, 37.8]);
        assert!(result.contains("xmin=-122.5"));
        assert!(result.contains("ymin=37.7"));
        assert!(result.contains("xmax=-122.4"));
        assert!(result.contains("ymax=37.8"));
    }

    #[test]
    fn test_substitute_template_no_vars() {
        let query = "SELECT * FROM table";
        let result = substitute_template(query, 0, 0, 0, &[0.0, 0.0, 0.0, 0.0]);
        assert_eq!(result, query);
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
        assert_eq!(mvt_command(1, 1), 9);
        assert_eq!(mvt_command(2, 3), 26);
        assert_eq!(mvt_command(7, 1), 15);
    }

    #[test]
    fn test_lon_lat_to_tile_xy_center() {
        let (x, y) = lon_lat_to_tile_xy(0.5, 0.5, &[0.0, 0.0, 1.0, 1.0], 4096);
        assert_eq!(x, 2048);
        assert_eq!(y, 2048);
    }

    #[test]
    fn test_wkb_point_to_mvt() {
        let mut wkb = vec![1u8];
        wkb.extend_from_slice(&1u32.to_le_bytes());
        wkb.extend_from_slice(&0.5f64.to_le_bytes());
        wkb.extend_from_slice(&0.5f64.to_le_bytes());

        let result = wkb_point_to_mvt(&wkb, &[0.0, 0.0, 1.0, 1.0]);
        assert!(result.is_some());
        let (geom, geom_type) = result.unwrap();
        assert_eq!(geom_type, 1);
        assert_eq!(geom.len(), 3);
    }

    #[test]
    fn test_wkb_point_to_mvt_empty() {
        assert!(wkb_point_to_mvt(&[], &[0.0, 0.0, 1.0, 1.0]).is_none());
    }

    #[test]
    fn test_encode_mvt_tile_basic() {
        let features = vec![MvtFeature {
            geom_type: 1,
            geometry: vec![mvt_command(1, 1), zigzag_encode(100), zigzag_encode(200)],
            properties: vec![("name".to_string(), PropValue::String("test".to_string()))],
        }];
        let bytes = encode_mvt_tile(&features, "test_layer");
        assert!(!bytes.is_empty());
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
    fn test_encode_mvt_value_bool() {
        let bytes = encode_mvt_value(&PropValue::Bool(true));
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_config_source_type_duckdb_serde() {
        use crate::config::SourceType;
        let json = serde_json::to_string(&SourceType::DuckDB).unwrap();
        assert_eq!(json, "\"duckdb\"");
        let parsed: SourceType = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, SourceType::DuckDB);
    }

    #[test]
    fn test_parse_duckdb_config() {
        let toml_str = r#"
            [[sources]]
            id = "places"
            type = "duckdb"
            path = ""
            query = "SELECT name, geometry FROM places WHERE ST_Intersects(geometry, ST_MakeEnvelope({bbox}))"
            layer_name = "places"
            minzoom = 0
            maxzoom = 14
        "#;

        let config: crate::config::Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.sources.len(), 1);
        assert_eq!(
            config.sources[0].source_type,
            crate::config::SourceType::DuckDB
        );
        assert_eq!(config.sources[0].id, "places");
        assert!(config.sources[0].query.is_some());
        assert_eq!(config.sources[0].layer_name, Some("places".to_string()));
    }

    #[test]
    fn test_parse_duckdb_config_minimal() {
        let toml_str = r#"
            [[sources]]
            id = "data"
            type = "duckdb"
            path = ""
            query = "SELECT * FROM t"
        "#;

        let config: crate::config::Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.sources.len(), 1);
        assert_eq!(
            config.sources[0].source_type,
            crate::config::SourceType::DuckDB
        );
        assert!(config.sources[0].query.is_some());
        assert!(config.sources[0].layer_name.is_none());
    }

    #[test]
    fn test_duckdb_connection_open_in_memory() {
        let conn = Connection::open_in_memory();
        assert!(conn.is_ok());
    }

    #[test]
    fn test_duckdb_basic_query() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE t (id INTEGER, name VARCHAR); INSERT INTO t VALUES (1, 'test');",
        )
        .unwrap();
        let mut stmt = conn.prepare("SELECT id, name FROM t").unwrap();
        let mut rows = stmt.query([]).unwrap();
        let row = rows.next().unwrap().unwrap();
        let id: i32 = row.get(0).unwrap();
        let name: String = row.get(1).unwrap();
        assert_eq!(id, 1);
        assert_eq!(name, "test");
    }
}

#[cfg(test)]
mod wkb_tests {
    //! Expected geometry streams are written out by hand from the MVT 2.1
    //! spec: `MoveTo`=1, `LineTo`=2, `ClosePath`=7, `command = id | count << 3`,
    //! `zigzag(n) = (n << 1) ^ (n >> 31)`, and the cursor is NOT moved by
    //! `ClosePath`. The tile bbox `[0, 0, 4096, 4096]` maps (lon, lat) to
    //! pixel (lon, 4096 - lat), so every coordinate below is exact.
    use super::*;

    const BBOX: [f64; 4] = [0.0, 0.0, 4096.0, 4096.0];

    fn px(x: f64, y: f64) -> [f64; 2] {
        [x, 4096.0 - y]
    }

    fn header(buf: &mut Vec<u8>, kind: u32) {
        buf.push(1);
        buf.extend_from_slice(&kind.to_le_bytes());
    }

    fn coords(buf: &mut Vec<u8>, points: &[[f64; 2]]) {
        for [x, y] in points {
            buf.extend_from_slice(&x.to_le_bytes());
            buf.extend_from_slice(&y.to_le_bytes());
        }
    }

    fn point(p: [f64; 2]) -> Vec<u8> {
        let mut b = Vec::new();
        header(&mut b, 1);
        coords(&mut b, &[p]);
        b
    }

    fn line(points: &[[f64; 2]]) -> Vec<u8> {
        let mut b = Vec::new();
        header(&mut b, 2);
        b.extend_from_slice(&u32::try_from(points.len()).expect("len").to_le_bytes());
        coords(&mut b, points);
        b
    }

    fn rings_body(buf: &mut Vec<u8>, rings: &[&[[f64; 2]]]) {
        buf.extend_from_slice(&u32::try_from(rings.len()).expect("len").to_le_bytes());
        for ring in rings {
            buf.extend_from_slice(&u32::try_from(ring.len()).expect("len").to_le_bytes());
            coords(buf, ring);
        }
    }

    fn polygon(rings: &[&[[f64; 2]]]) -> Vec<u8> {
        let mut b = Vec::new();
        header(&mut b, 3);
        rings_body(&mut b, rings);
        b
    }

    fn multi(kind: u32, parts: &[Vec<u8>]) -> Vec<u8> {
        let mut b = Vec::new();
        header(&mut b, kind);
        b.extend_from_slice(&u32::try_from(parts.len()).expect("len").to_le_bytes());
        for part in parts {
            b.extend_from_slice(part);
        }
        b
    }

    /// Square outer ring, pixels (10,10) (20,10) (20,20) (10,20) closed.
    fn outer() -> Vec<[f64; 2]> {
        vec![
            px(10.0, 10.0),
            px(20.0, 10.0),
            px(20.0, 20.0),
            px(10.0, 20.0),
            px(10.0, 10.0),
        ]
    }

    /// Hole ring, pixels (12,12) (12,18) (18,18) (18,12) closed.
    fn hole() -> Vec<[f64; 2]> {
        vec![
            px(12.0, 12.0),
            px(12.0, 18.0),
            px(18.0, 18.0),
            px(18.0, 12.0),
            px(12.0, 12.0),
        ]
    }

    #[test]
    fn point_encodes_moveto() {
        assert_eq!(
            wkb_to_mvt(&point(px(10.0, 10.0)), &BBOX),
            Some((vec![9, 20, 20], 1))
        );
    }

    #[test]
    fn linestring_encodes_deltas() {
        let wkb = line(&[px(10.0, 10.0), px(20.0, 20.0), px(15.0, 10.0)]);
        // MoveTo(10,10); LineTo x2: (+10,+10), (-5,-10)
        assert_eq!(
            wkb_to_mvt(&wkb, &BBOX),
            Some((vec![9, 20, 20, 18, 20, 20, 9, 19], 2))
        );
    }

    #[test]
    fn polygon_single_ring_drops_closing_point() {
        let ring = outer();
        // MoveTo(10,10); LineTo x3: (+10,0) (0,+10) (-10,0); ClosePath
        assert_eq!(
            wkb_to_mvt(&polygon(&[&ring]), &BBOX),
            Some((vec![9, 20, 20, 26, 20, 0, 0, 20, 19, 0, 15], 3))
        );
    }

    #[test]
    fn polygon_hole_moveto_is_relative_to_last_lineto() {
        let (o, h) = (outer(), hole());
        let (geom, kind) = wkb_to_mvt(&polygon(&[&o, &h]), &BBOX).expect("polygon");
        assert_eq!(kind, 3);
        // After the outer ring the cursor sits at (10,20), the last LineTo.
        // Hole MoveTo(12,12) is therefore (+2,-8) -> zigzag 4, 15.
        assert_eq!(&geom[11..14], &[9, 4, 15]);
        assert_eq!(geom.len(), 22);
    }

    #[test]
    fn multipoint_shares_one_moveto() {
        let wkb = multi(4, &[point(px(10.0, 10.0)), point(px(13.0, 6.0))]);
        // MoveTo x2: (10,10), (+3,-4)
        assert_eq!(wkb_to_mvt(&wkb, &BBOX), Some((vec![17, 20, 20, 6, 7], 1)));
    }

    #[test]
    fn multilinestring_cursor_carries_across_parts() {
        let a = line(&[px(10.0, 10.0), px(20.0, 10.0)]);
        let b = line(&[px(20.0, 30.0), px(10.0, 30.0)]);
        // MoveTo(10,10) LineTo(+10,0); MoveTo(0,+20) LineTo(-10,0)
        assert_eq!(
            wkb_to_mvt(&multi(5, &[a, b]), &BBOX),
            Some((vec![9, 20, 20, 10, 20, 0, 9, 0, 40, 10, 19, 0], 2))
        );
    }

    #[test]
    fn multipolygon_second_part_moveto_is_relative_to_last_lineto() {
        let o = outer();
        let shifted: Vec<[f64; 2]> = o.iter().map(|[x, y]| [x + 100.0, *y]).collect();
        let mut p1 = Vec::new();
        header(&mut p1, 3);
        rings_body(&mut p1, &[&o]);
        let mut p2 = Vec::new();
        header(&mut p2, 3);
        rings_body(&mut p2, &[&shifted]);
        let (geom, kind) = wkb_to_mvt(&multi(6, &[p1, p2]), &BBOX).expect("multipolygon");
        assert_eq!(kind, 3);
        // Cursor after part 1 = (10,20); part 2 starts at (110,10): (+100,-10).
        assert_eq!(&geom[11..14], &[9, 200, 19]);
    }

    #[test]
    fn malformed_input_returns_none() {
        assert_eq!(wkb_to_mvt(&[1, 1, 0], &BBOX), None);
        assert_eq!(wkb_to_mvt(&point(px(1.0, 1.0))[..20], &BBOX), None);
        let mut unknown = Vec::new();
        header(&mut unknown, 7);
        unknown.extend_from_slice(&[0; 8]);
        assert_eq!(wkb_to_mvt(&unknown, &BBOX), None);
        assert_eq!(wkb_to_mvt(&line(&[px(1.0, 1.0)]), &BBOX), None);
        let full = line(&[px(1.0, 1.0), px(2.0, 2.0)]);
        assert_eq!(wkb_to_mvt(&full[..full.len() - 1], &BBOX), None);
        let short_ring = [px(1.0, 1.0), px(2.0, 2.0), px(1.0, 1.0)];
        assert_eq!(wkb_to_mvt(&polygon(&[&short_ring]), &BBOX), None);
        assert_eq!(wkb_to_mvt(&polygon(&[]), &BBOX), None);
        assert_eq!(wkb_to_mvt(&multi(4, &[]), &BBOX), None);
        assert_eq!(wkb_to_mvt(&multi(5, &[]), &BBOX), None);
        assert_eq!(wkb_to_mvt(&multi(6, &[]), &BBOX), None);
        let bad_part = multi(5, &[line(&[px(1.0, 1.0)])]);
        assert_eq!(wkb_to_mvt(&bad_part, &BBOX), None);
    }
}
