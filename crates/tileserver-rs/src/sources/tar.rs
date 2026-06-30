//! Tar-archive tile source: serves `{z}/{x}/{y}.{ext}` entries from a `.tar`
//! (optionally `.tar.gz`/`.tgz`/`.tar.br`/`.tar.zst`) bundle.
//!
//! On startup the archive is decompressed once into a single in-memory buffer
//! and an index of `(z, x, y) → byte range` is built by walking the tar
//! entries. Individual tiles are then served zero-copy via [`bytes::Bytes`]
//! slices into that buffer. The whole-archive-in-memory tradeoff is documented
//! for operators: for planet-scale tile sets, PMTiles remains the production
//! choice because its hierarchical directory avoids holding all tiles resident.

use async_trait::async_trait;
use bytes::Bytes;
use std::collections::HashMap;
use std::io::Read;
use std::ops::Range;
use std::path::Path;

use crate::config::SourceConfig;
use crate::error::{Result, TileServerError};
use crate::sources::tile_layout::flip_y;
use crate::sources::{TileCompression, TileData, TileFormat, TileMetadata, TileSource};

/// `(z, x, y) → byte range` tile index into a decompressed archive buffer.
type TileIndex = HashMap<(u8, u32, u32), Range<usize>>;

/// Archive compression detected from the file extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArchiveCompression {
    None,
    Gzip,
    Brotli,
    Zstd,
}

impl ArchiveCompression {
    /// Detect the archive compression from a file path's extension.
    fn from_path(path: &Path) -> Self {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
            Self::Gzip
        } else if name.ends_with(".tar.br") {
            Self::Brotli
        } else if name.ends_with(".tar.zst") {
            Self::Zstd
        } else {
            Self::None
        }
    }
}

/// Tar-archive tile source.
#[derive(Debug)]
pub struct TarSource {
    /// Decompressed archive bytes; tile slices reference into this buffer.
    buffer: Bytes,
    /// `(z, x, y) → byte range` index into `buffer`.
    index: TileIndex,
    /// TMS (south-up) addressing when `true`, XYZ otherwise.
    tms: bool,
    /// Resolved metadata (zoom/bounds derived from the index).
    metadata: TileMetadata,
}

impl TarSource {
    /// Build a tar source from operator config, decompressing and indexing the
    /// archive once.
    ///
    /// # Errors
    ///
    /// Returns [`TileServerError::FileError`] if the archive cannot be read or
    /// decompressed, or [`TileServerError::ConfigError`] if it contains no
    /// recognisable `{z}/{x}/{y}` tiles.
    pub async fn from_file(config: &SourceConfig) -> Result<Self> {
        let path = config.path.clone();
        let id = config.id.clone();
        let tms = config.tms;

        let (buffer, index) =
            tokio::task::spawn_blocking(move || decompress_and_index(Path::new(&path)))
                .await
                .map_err(|e| {
                    TileServerError::ConfigError(format!("tar index task failed: {e}"))
                })??;

        if index.is_empty() {
            return Err(TileServerError::ConfigError(format!(
                "tar source '{id}' contains no recognisable {{z}}/{{x}}/{{y}} tiles"
            )));
        }

        let format = resolve_format(config, &buffer, &index);
        let (minzoom, maxzoom) = zoom_range(&index);
        let bounds = bounds_from_index(&index, maxzoom, tms);
        let vector_layers = if format.is_vector() {
            extract_vector_layers(&buffer, &index, minzoom)
        } else {
            None
        };

        let metadata = TileMetadata {
            id: config.id.clone(),
            name: config.name.clone().unwrap_or_else(|| config.id.clone()),
            description: config.description.clone(),
            attribution: config.attribution.clone(),
            format,
            minzoom: config.minzoom.unwrap_or(minzoom),
            maxzoom: config.maxzoom.unwrap_or(maxzoom),
            bounds,
            center: None,
            vector_layers,
        };

        tracing::info!(
            "Loaded tar source '{}': {} tiles (zoom {}-{})",
            config.id,
            index.len(),
            metadata.minzoom,
            metadata.maxzoom
        );

        Ok(Self {
            buffer,
            index,
            tms,
            metadata,
        })
    }
}

#[async_trait]
impl TileSource for TarSource {
    async fn get_tile(&self, z: u8, x: u32, y: u32) -> Result<Option<TileData>> {
        let max_tile = 1u32 << z;
        if x >= max_tile || y >= max_tile {
            return Err(TileServerError::InvalidCoordinates { z, x, y });
        }
        if z < self.metadata.minzoom || z > self.metadata.maxzoom {
            return Ok(None);
        }

        let lookup_y = if self.tms { flip_y(z, y) } else { y };
        let Some(range) = self.index.get(&(z, x, lookup_y)) else {
            return Ok(None);
        };

        let data = self.buffer.slice(range.clone());
        let compression = detect_compression(&data);
        Ok(Some(TileData {
            data,
            format: self.metadata.format,
            compression,
        }))
    }

    fn metadata(&self) -> &TileMetadata {
        &self.metadata
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Decompress the archive into a single buffer and build the tile index.
fn decompress_and_index(path: &Path) -> Result<(Bytes, TileIndex)> {
    if !path.is_file() {
        return Err(TileServerError::FileError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("tar archive not found: {}", path.display()),
        )));
    }

    let raw = std::fs::read(path).map_err(TileServerError::FileError)?;
    let decompressed = decompress_archive(&raw, ArchiveCompression::from_path(path))?;
    let buffer = Bytes::from(decompressed);
    let index = build_index(&buffer)?;
    Ok((buffer, index))
}

/// Decompress the whole archive based on its detected compression.
fn decompress_archive(raw: &[u8], compression: ArchiveCompression) -> Result<Vec<u8>> {
    match compression {
        ArchiveCompression::None => Ok(raw.to_vec()),
        ArchiveCompression::Gzip => {
            let mut out = Vec::new();
            flate2::read::GzDecoder::new(raw)
                .read_to_end(&mut out)
                .map_err(TileServerError::FileError)?;
            Ok(out)
        }
        ArchiveCompression::Brotli => {
            let mut out = Vec::new();
            brotli::Decompressor::new(raw, 4096)
                .read_to_end(&mut out)
                .map_err(TileServerError::FileError)?;
            Ok(out)
        }
        ArchiveCompression::Zstd => {
            zstd::stream::decode_all(raw).map_err(TileServerError::FileError)
        }
    }
}

/// Walk the decompressed tar buffer and index each `{z}/{x}/{y}` entry by its
/// byte range inside the buffer.
fn build_index(buffer: &Bytes) -> Result<TileIndex> {
    let mut index = HashMap::new();
    let mut archive = tar::Archive::new(std::io::Cursor::new(buffer.as_ref()));
    let entries = archive
        .entries()
        .map_err(|e| TileServerError::ConfigError(format!("invalid tar archive: {e}")))?;

    for entry in entries {
        let entry =
            entry.map_err(|e| TileServerError::ConfigError(format!("invalid tar entry: {e}")))?;
        let header = entry.header();
        if header.entry_type().is_dir() {
            continue;
        }
        let Ok(path) = entry.path() else { continue };
        let Some(coords) = parse_tile_coords(path.as_ref()) else {
            continue;
        };
        let start = usize::try_from(entry.raw_file_position()).unwrap_or(0);
        let len = usize::try_from(entry.size()).unwrap_or(0);
        index.insert(coords, start..start + len);
    }

    Ok(index)
}

/// Parse `(z, x, y)` from the trailing `z/x/y.ext` path components.
fn parse_tile_coords(path: &Path) -> Option<(u8, u32, u32)> {
    let mut components: Vec<&str> = path
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => s.to_str(),
            _ => None,
        })
        .collect();
    let y_part = components.pop()?;
    let x_part = components.pop()?;
    let z_part = components.pop()?;

    let z: u8 = z_part.parse().ok()?;
    let x: u32 = x_part.parse().ok()?;
    let y_stem = y_part.split('.').next().unwrap_or(y_part);
    let y: u32 = y_stem.parse().ok()?;
    Some((z, x, y))
}

/// Resolve the served format: `serve_as` override wins, then a probe of the
/// first indexed tile's bytes, defaulting to PBF.
fn resolve_format(config: &SourceConfig, buffer: &Bytes, index: &TileIndex) -> TileFormat {
    if let Some(fmt) = config.serve_as {
        return fmt;
    }
    // Probe the first tile's magic bytes for raster formats; default to PBF.
    if let Some(range) = index.values().next() {
        let bytes = &buffer[range.clone()];
        if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
            return TileFormat::Png;
        }
        if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
            return TileFormat::Jpeg;
        }
        if bytes.starts_with(b"RIFF") {
            return TileFormat::Webp;
        }
    }
    TileFormat::Pbf
}

/// Derive `(minzoom, maxzoom)` from the index keys.
fn zoom_range(index: &TileIndex) -> (u8, u8) {
    let mut min_z = u8::MAX;
    let mut max_z = 0u8;
    for &(z, _, _) in index.keys() {
        min_z = min_z.min(z);
        max_z = max_z.max(z);
    }
    if min_z == u8::MAX {
        (0, 0)
    } else {
        (min_z, max_z)
    }
}

/// Derive WGS-84 bounds from the tile envelope at the maximum zoom.
fn bounds_from_index(index: &TileIndex, max_z: u8, tms: bool) -> Option<[f64; 4]> {
    let (mut min_x, mut max_x, mut min_y, mut max_y) = (u32::MAX, 0u32, u32::MAX, 0u32);
    let mut any = false;
    for &(z, x, y) in index.keys() {
        if z != max_z {
            continue;
        }
        any = true;
        let y = if tms { flip_y(z, y) } else { y };
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    if !any {
        return None;
    }
    Some([
        tile_x_to_lon(min_x, max_z),
        tile_y_to_lat(max_y + 1, max_z),
        tile_x_to_lon(max_x + 1, max_z),
        tile_y_to_lat(min_y, max_z),
    ])
}

/// Extract `vector_layers` from the first MVT tile at `minzoom`, mirroring the
/// PMTiles introspection path.
fn extract_vector_layers(
    buffer: &Bytes,
    index: &TileIndex,
    minzoom: u8,
) -> Option<serde_json::Value> {
    let range = index
        .iter()
        .find(|((z, _, _), _)| *z == minzoom)
        .map(|(_, r)| r.clone())?;
    let raw = &buffer[range];
    let decoded = maybe_gunzip(raw);
    layer_names_to_vector_layers(&decoded)
}

/// Gunzip tile bytes if gzip-framed, else return a copy.
fn maybe_gunzip(data: &[u8]) -> Vec<u8> {
    if data.len() >= 2 && data[0] == 0x1f && data[1] == 0x8b {
        let mut out = Vec::new();
        if flate2::read::GzDecoder::new(data)
            .read_to_end(&mut out)
            .is_ok()
        {
            return out;
        }
    }
    data.to_vec()
}

/// Build a minimal `vector_layers` array from MVT layer names.
fn layer_names_to_vector_layers(mvt: &[u8]) -> Option<serde_json::Value> {
    let names = parse_mvt_layer_names(mvt);
    if names.is_empty() {
        return None;
    }
    let layers: Vec<serde_json::Value> = names
        .into_iter()
        .map(|id| serde_json::json!({ "id": id, "fields": {} }))
        .collect();
    Some(serde_json::Value::Array(layers))
}

/// Extract MVT layer names by scanning top-level protobuf fields.
///
/// The MVT spec encodes each layer as field 3 (tag `0x1a`, wire type 2) of the
/// `Tile` message, and each layer's `name` as field 1 (tag `0x0a`) of the
/// `Layer` message. This walks only those two tags, which is enough to list
/// layer names without a full protobuf decoder.
fn parse_mvt_layer_names(mvt: &[u8]) -> Vec<String> {
    let mut names = Vec::new();
    let mut pos = 0usize;
    while pos < mvt.len() {
        let tag = mvt[pos];
        pos += 1;
        // Tile.layers == field 3, wire type 2 (length-delimited) == 0x1a.
        if tag != 0x1a {
            return names;
        }
        let Some((layer_len, next)) = read_varint(mvt, pos) else {
            return names;
        };
        pos = next;
        let layer_end = pos + layer_len as usize;
        if layer_end > mvt.len() {
            return names;
        }
        if let Some(name) = first_layer_name(&mvt[pos..layer_end]) {
            names.push(name);
        }
        pos = layer_end;
    }
    names
}

/// Read the `name` (field 1) from a single MVT Layer message body.
fn first_layer_name(layer: &[u8]) -> Option<String> {
    let mut pos = 0usize;
    while pos < layer.len() {
        let tag = layer[pos];
        pos += 1;
        // Layer.name == field 1, wire type 2 == 0x0a.
        if tag == 0x0a {
            let (len, next) = read_varint(layer, pos)?;
            pos = next;
            let end = pos + len as usize;
            if end > layer.len() {
                return None;
            }
            return std::str::from_utf8(&layer[pos..end])
                .ok()
                .map(str::to_owned);
        }
        // Skip any other field by its wire type.
        let (len, next) = read_varint(layer, pos)?;
        pos = next;
        match tag & 0x07 {
            2 => pos += len as usize,
            0 => {}
            _ => return None,
        }
    }
    None
}

/// Read a base-128 varint, returning `(value, next_position)`.
fn read_varint(buf: &[u8], mut pos: usize) -> Option<(u64, usize)> {
    let mut value = 0u64;
    let mut shift = 0u32;
    loop {
        let b = *buf.get(pos)?;
        pos += 1;
        value |= u64::from(b & 0x7f) << shift;
        if b & 0x80 == 0 {
            return Some((value, pos));
        }
        shift += 7;
        if shift > 63 {
            return None;
        }
    }
}

/// Detect gzip framing on raw tile bytes (matches the MBTiles source policy).
fn detect_compression(data: &[u8]) -> TileCompression {
    if data.len() >= 2 && data[0] == 0x1f && data[1] == 0x8b {
        TileCompression::Gzip
    } else {
        TileCompression::None
    }
}

/// Web Mercator tile column → longitude (degrees) of its western edge.
fn tile_x_to_lon(x: u32, z: u8) -> f64 {
    f64::from(x) / f64::from(1u32 << z) * 360.0 - 180.0
}

/// Web Mercator tile row → latitude (degrees) of its northern edge.
fn tile_y_to_lat(y: u32, z: u8) -> f64 {
    let n = std::f64::consts::PI * (1.0 - 2.0 * f64::from(y) / f64::from(1u32 << z));
    n.sinh().atan().to_degrees()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn base_config(path: &str) -> SourceConfig {
        SourceConfig {
            id: "tar-src".to_string(),
            source_type: crate::config::SourceType::Tar,
            path: path.to_string(),
            name: None,
            attribution: None,
            description: None,
            resampling: None,
            layer_name: None,
            geometry_column: None,
            minzoom: None,
            maxzoom: None,
            query: None,
            serve_as: None,
            #[cfg(feature = "raster")]
            colormap: None,
            options: None,
            collection: None,
            asset_role: "visual".to_string(),
            dynamic: false,
            max_items: 100,
            stac_bbox: None,
            pixel_selection: crate::config::PixelSelectionMethod::First,
            tile_path_template: None,
            tms: false,
            #[cfg(feature = "dem")]
            input_source: None,
            #[cfg(feature = "dem")]
            dem_encoding: crate::config::DemEncoding::Terrarium,
            #[cfg(feature = "dem")]
            dem_scale: None,
            #[cfg(feature = "dem")]
            dem_offset: None,
            #[cfg(feature = "dem")]
            dem_band: 1,
            #[cfg(feature = "dem")]
            dem_nodata_color: None,
        }
    }

    /// A single-layer MVT tile with layer name "roads".
    fn mvt_with_roads_layer() -> Vec<u8> {
        // Tile { layers: [ Layer { name: "roads" } ] }
        // 0x1a = Tile.layers (field 3, LEN); 0x07 = layer length;
        // 0x0a = Layer.name (field 1, LEN); 0x05 = name length; "roads".
        let mut t = vec![0x1a, 0x07, 0x0a, 0x05];
        t.extend_from_slice(b"roads");
        t
    }

    fn build_tar(tiles: &[(&str, &[u8])]) -> Vec<u8> {
        let mut builder = tar::Builder::new(Vec::new());
        for (name, bytes) in tiles {
            let mut header = tar::Header::new_gnu();
            header.set_size(bytes.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append_data(&mut header, name, *bytes).unwrap();
        }
        builder.into_inner().unwrap()
    }

    fn write_temp(bytes: &[u8], suffix: &str) -> tempfile::TempPath {
        let mut f = tempfile::Builder::new().suffix(suffix).tempfile().unwrap();
        f.write_all(bytes).unwrap();
        f.into_temp_path()
    }

    fn gzip(data: &[u8]) -> Vec<u8> {
        let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        enc.write_all(data).unwrap();
        enc.finish().unwrap()
    }

    fn brotli_compress(data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut writer = brotli::CompressorWriter::new(&mut out, 4096, 5, 22);
        writer.write_all(data).unwrap();
        drop(writer);
        out
    }

    #[tokio::test]
    async fn from_file_rejects_missing_archive() {
        let cfg = base_config("/__no_such__/tiles.tar");
        let err = TarSource::from_file(&cfg).await.unwrap_err();
        assert!(matches!(err, TileServerError::FileError(_)));
    }

    #[tokio::test]
    async fn raw_tar_round_trips_a_tile() {
        let tar = build_tar(&[("2/1/1.pbf", b"tile-211"), ("3/4/5.pbf", b"tile-345")]);
        let path = write_temp(&tar, ".tar");
        let cfg = base_config(&path.to_string_lossy());
        let src = TarSource::from_file(&cfg).await.unwrap();
        let tile = src.get_tile(3, 4, 5).await.unwrap().expect("indexed tile");
        assert_eq!(tile.data.as_ref(), b"tile-345");
        assert_eq!(tile.format, TileFormat::Pbf);
    }

    #[tokio::test]
    async fn index_has_exact_tile_count_and_zoom_range() {
        let tar = build_tar(&[
            ("2/1/1.pbf", b"a"),
            ("3/4/5.pbf", b"b"),
            ("3/4/6.pbf", b"c"),
        ]);
        let path = write_temp(&tar, ".tar");
        let cfg = base_config(&path.to_string_lossy());
        let src = TarSource::from_file(&cfg).await.unwrap();
        assert_eq!(src.index.len(), 3);
        assert_eq!(src.metadata().minzoom, 2);
        assert_eq!(src.metadata().maxzoom, 3);
    }

    #[tokio::test]
    async fn gzip_archive_round_trips() {
        let tar = build_tar(&[("1/0/0.pbf", b"gz-tile")]);
        let path = write_temp(&gzip(&tar), ".tar.gz");
        let cfg = base_config(&path.to_string_lossy());
        let src = TarSource::from_file(&cfg).await.unwrap();
        let tile = src.get_tile(1, 0, 0).await.unwrap().expect("gz tile");
        assert_eq!(tile.data.as_ref(), b"gz-tile");
    }

    #[tokio::test]
    async fn brotli_archive_round_trips() {
        let tar = build_tar(&[("1/0/0.pbf", b"br-tile")]);
        let path = write_temp(&brotli_compress(&tar), ".tar.br");
        let cfg = base_config(&path.to_string_lossy());
        let src = TarSource::from_file(&cfg).await.unwrap();
        let tile = src.get_tile(1, 0, 0).await.unwrap().expect("br tile");
        assert_eq!(tile.data.as_ref(), b"br-tile");
    }

    #[tokio::test]
    async fn zstd_archive_round_trips() {
        let tar = build_tar(&[("1/0/0.pbf", b"zst-tile")]);
        let compressed = zstd::stream::encode_all(&tar[..], 3).unwrap();
        let path = write_temp(&compressed, ".tar.zst");
        let cfg = base_config(&path.to_string_lossy());
        let src = TarSource::from_file(&cfg).await.unwrap();
        let tile = src.get_tile(1, 0, 0).await.unwrap().expect("zst tile");
        assert_eq!(tile.data.as_ref(), b"zst-tile");
    }

    #[tokio::test]
    async fn missing_tile_returns_none() {
        let tar = build_tar(&[("2/1/1.pbf", b"x")]);
        let path = write_temp(&tar, ".tar");
        let cfg = base_config(&path.to_string_lossy());
        let src = TarSource::from_file(&cfg).await.unwrap();
        assert!(src.get_tile(2, 3, 3).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn invalid_coords_error() {
        let tar = build_tar(&[("2/1/1.pbf", b"x")]);
        let path = write_temp(&tar, ".tar");
        let cfg = base_config(&path.to_string_lossy());
        let src = TarSource::from_file(&cfg).await.unwrap();
        let err = src.get_tile(2, 99, 0).await.unwrap_err();
        assert!(matches!(
            err,
            TileServerError::InvalidCoordinates { z: 2, x: 99, y: 0 }
        ));
    }

    #[tokio::test]
    async fn empty_archive_errors() {
        let tar = build_tar(&[("readme.txt", b"not a tile")]);
        let path = write_temp(&tar, ".tar");
        let cfg = base_config(&path.to_string_lossy());
        let err = TarSource::from_file(&cfg).await.unwrap_err();
        assert!(matches!(err, TileServerError::ConfigError(_)));
    }

    #[tokio::test]
    async fn vector_layers_introspected() {
        let tar = build_tar(&[("1/0/0.pbf", &mvt_with_roads_layer())]);
        let path = write_temp(&tar, ".tar");
        let cfg = base_config(&path.to_string_lossy());
        let src = TarSource::from_file(&cfg).await.unwrap();
        let layers = src.metadata().vector_layers.as_ref().expect("layers");
        let arr = layers.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["id"], "roads");
    }

    #[tokio::test]
    async fn tms_flips_y_on_lookup() {
        // Archive stores TMS row 2; XYZ request y=1 at z=2 should resolve to it.
        let tar = build_tar(&[("2/1/2.pbf", b"tms-tile")]);
        let path = write_temp(&tar, ".tar");
        let mut cfg = base_config(&path.to_string_lossy());
        cfg.tms = true;
        let src = TarSource::from_file(&cfg).await.unwrap();
        let tile = src.get_tile(2, 1, 1).await.unwrap().expect("flipped tile");
        assert_eq!(tile.data.as_ref(), b"tms-tile");
    }

    #[tokio::test]
    async fn png_format_detected_from_magic() {
        let png = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        let tar = build_tar(&[("1/0/0.png", &png)]);
        let path = write_temp(&tar, ".tar");
        let cfg = base_config(&path.to_string_lossy());
        let src = TarSource::from_file(&cfg).await.unwrap();
        assert_eq!(src.metadata().format, TileFormat::Png);
    }

    #[tokio::test]
    async fn as_any_downcasts() {
        let tar = build_tar(&[("1/0/0.pbf", b"x")]);
        let path = write_temp(&tar, ".tar");
        let cfg = base_config(&path.to_string_lossy());
        let src = TarSource::from_file(&cfg).await.unwrap();
        assert!(src.as_any().downcast_ref::<TarSource>().is_some());
    }

    #[test]
    fn parse_tile_coords_extracts_zxy() {
        assert_eq!(parse_tile_coords(Path::new("3/4/5.pbf")), Some((3, 4, 5)));
        assert_eq!(
            parse_tile_coords(Path::new("tiles/10/200/300.png")),
            Some((10, 200, 300))
        );
        assert_eq!(parse_tile_coords(Path::new("readme.txt")), None);
    }

    #[test]
    fn archive_compression_from_extension() {
        assert_eq!(
            ArchiveCompression::from_path(Path::new("t.tar.gz")),
            ArchiveCompression::Gzip
        );
        assert_eq!(
            ArchiveCompression::from_path(Path::new("t.tgz")),
            ArchiveCompression::Gzip
        );
        assert_eq!(
            ArchiveCompression::from_path(Path::new("t.tar.br")),
            ArchiveCompression::Brotli
        );
        assert_eq!(
            ArchiveCompression::from_path(Path::new("t.tar.zst")),
            ArchiveCompression::Zstd
        );
        assert_eq!(
            ArchiveCompression::from_path(Path::new("t.tar")),
            ArchiveCompression::None
        );
    }
}
