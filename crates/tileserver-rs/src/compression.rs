//! `Accept-Encoding` negotiation and tile body (de)compression.
//!
//! Bridges the client's `Accept-Encoding` header to a tile's stored encoding,
//! deciding whether to passthrough (no work) or decode-then-re-encode. The tile
//! route handlers call [`negotiate`] to pick the response encoding and
//! [`recompress`] to produce the bytes; the moka cache keys on the chosen
//! encoding so re-encode cost is paid once per `(tile, encoding)` pair.

use std::io::{Read, Write};

use bytes::Bytes;

use crate::config::CompressionConfig;
use crate::error::{Result, TileServerError};
use crate::sources::{TileCompression, TileData};

/// Tiles below this size are not worth compressing: the encoder framing
/// overhead can exceed the savings, and the CPU cost never pays off.
pub const MIN_COMPRESS_BYTES: usize = 200;

/// Parse an `Accept-Encoding` header into `(token, q)` pairs. Tokens are
/// lowercased; `q=0` entries are retained so explicit refusals (`identity;q=0`)
/// can be distinguished from absence. Malformed q-values default to `1.0`.
fn parse_accept_encoding(header: &str) -> Vec<(String, f32)> {
    header
        .split(',')
        .filter_map(|part| {
            let mut segs = part.split(';');
            let token = segs.next()?.trim().to_ascii_lowercase();
            if token.is_empty() {
                return None;
            }
            let q = segs
                .find_map(|s| {
                    s.trim()
                        .strip_prefix("q=")
                        .and_then(|v| v.trim().parse::<f32>().ok())
                })
                .unwrap_or(1.0);
            Some((token, q))
        })
        .collect()
}

/// Effective q-value for `token`: exact match first, then wildcard `*`.
/// Returns `None` when refused (`q<=0`) or absent.
fn quality_of(accepted: &[(String, f32)], token: &str) -> Option<f32> {
    if let Some((_, q)) = accepted.iter().find(|(t, _)| t == token) {
        return (*q > 0.0).then_some(*q);
    }
    if let Some((_, q)) = accepted.iter().find(|(t, _)| t == "*") {
        return (*q > 0.0).then_some(*q);
    }
    None
}

/// Whether `identity` (no encoding) may be served. Per RFC 7231 identity is
/// acceptable by default unless explicitly refused via `identity;q=0` or `*;q=0`
/// with no more-specific identity entry.
fn identity_acceptable(accepted: &[(String, f32)]) -> bool {
    if let Some((_, q)) = accepted.iter().find(|(t, _)| t == "identity") {
        return *q > 0.0;
    }
    if let Some((_, q)) = accepted.iter().find(|(t, _)| t == "*") {
        return *q > 0.0;
    }
    true
}

/// Pick the highest-preference *compressed* encoding the client accepts.
/// Ties break on server preference order brotli > zstd > gzip (smallest first).
/// Falls back to [`TileCompression::None`] when no compressed encoding is
/// acceptable.
#[must_use]
pub fn best_target_encoding(header: &str) -> TileCompression {
    let accepted = parse_accept_encoding(header);
    let candidates = [
        (TileCompression::Brotli, "br"),
        (TileCompression::Zstd, "zstd"),
        (TileCompression::Gzip, "gzip"),
    ];
    let mut best: Option<(TileCompression, f32)> = None;
    for (comp, token) in candidates {
        if let Some(q) = quality_of(&accepted, token) {
            // Iterating in preference order + replacing only on a strictly
            // greater q means the first (most-preferred) encoding wins ties.
            match best {
                Some((_, bq)) if bq >= q => {}
                _ => best = Some((comp, q)),
            }
        }
    }
    best.map_or(TileCompression::None, |(comp, _)| comp)
}

/// Decide the response encoding given the client's `Accept-Encoding`, the tile's
/// stored encoding, its size, and config. The returned encoding equal to
/// `source` signals a passthrough (no decode/re-encode).
#[must_use]
pub fn negotiate(
    accept_encoding: Option<&str>,
    source: TileCompression,
    tile_len: usize,
    cfg: &CompressionConfig,
) -> TileCompression {
    if cfg.minimal_recompression {
        return source;
    }
    let Some(header) = accept_encoding else {
        return source;
    };
    let accepted = parse_accept_encoding(header);

    // Rule 1: source encoding already acceptable -> passthrough (latency win).
    match source {
        TileCompression::None if identity_acceptable(&accepted) => return source,
        TileCompression::None => {}
        other => {
            if let Some(token) = other.content_encoding()
                && quality_of(&accepted, token).is_some()
            {
                return source;
            }
        }
    }

    // Tiny tiles: skip compression when identity is acceptable.
    if tile_len < MIN_COMPRESS_BYTES && identity_acceptable(&accepted) {
        return TileCompression::None;
    }

    // Rule 2: best acceptable compressed encoding, else identity, else
    // passthrough (client accepts nothing we can produce).
    match best_target_encoding(header) {
        TileCompression::None if identity_acceptable(&accepted) => TileCompression::None,
        TileCompression::None => source,
        target => target,
    }
}

/// Decompress `data` from `from` to raw identity bytes.
pub fn decode(data: &[u8], from: TileCompression) -> Result<Vec<u8>> {
    match from {
        TileCompression::None => Ok(data.to_vec()),
        TileCompression::Gzip => {
            let mut out = Vec::new();
            flate2::read::GzDecoder::new(data)
                .read_to_end(&mut out)
                .map_err(|e| TileServerError::CompressionError(format!("gzip decode: {e}")))?;
            Ok(out)
        }
        TileCompression::Zstd => zstd::decode_all(data)
            .map_err(|e| TileServerError::CompressionError(format!("zstd decode: {e}"))),
        TileCompression::Brotli => {
            let mut out = Vec::new();
            brotli::Decompressor::new(data, 4096)
                .read_to_end(&mut out)
                .map_err(|e| TileServerError::CompressionError(format!("brotli decode: {e}")))?;
            Ok(out)
        }
    }
}

/// Compress raw identity `data` to `to` using the configured quality/level.
pub fn encode(data: &[u8], to: TileCompression, cfg: &CompressionConfig) -> Result<Vec<u8>> {
    match to {
        TileCompression::None => Ok(data.to_vec()),
        TileCompression::Gzip => {
            let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
            enc.write_all(data)
                .map_err(|e| TileServerError::CompressionError(format!("gzip encode: {e}")))?;
            enc.finish()
                .map_err(|e| TileServerError::CompressionError(format!("gzip encode: {e}")))
        }
        TileCompression::Zstd => zstd::encode_all(data, cfg.zstd_level)
            .map_err(|e| TileServerError::CompressionError(format!("zstd encode: {e}"))),
        TileCompression::Brotli => {
            let mut writer =
                brotli::CompressorWriter::new(Vec::new(), 4096, u32::from(cfg.br_quality), 22);
            writer
                .write_all(data)
                .map_err(|e| TileServerError::CompressionError(format!("brotli encode: {e}")))?;
            Ok(writer.into_inner())
        }
    }
}

/// Convert `data` from encoding `from` to encoding `to`. Identity-fast-path when
/// the encodings already match (no decode/encode round-trip).
pub fn recompress(
    data: &[u8],
    from: TileCompression,
    to: TileCompression,
    cfg: &CompressionConfig,
) -> Result<Vec<u8>> {
    if from == to {
        return Ok(data.to_vec());
    }
    let raw = decode(data, from)?;
    encode(&raw, to, cfg)
}

/// Re-encode `tile` to `target`, returning a new [`TileData`] whose bytes and
/// `compression` field carry `target`. The caller is responsible for having
/// chosen `target` via [`negotiate`]; this is the pure byte-transform step that
/// the cached and uncached re-encode paths share.
pub fn recode(
    tile: &TileData,
    target: TileCompression,
    cfg: &CompressionConfig,
) -> Result<TileData> {
    let bytes = recompress(&tile.data, tile.compression, target, cfg)?;
    Ok(TileData {
        data: Bytes::from(bytes),
        format: tile.format,
        compression: target,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn best_target_picks_brotli_from_full_set() {
        assert_eq!(
            best_target_encoding("br, zstd, gzip"),
            TileCompression::Brotli
        );
    }

    #[test]
    fn best_target_gzip_only() {
        assert_eq!(best_target_encoding("gzip"), TileCompression::Gzip);
    }

    #[test]
    fn best_target_identity_yields_none() {
        assert_eq!(best_target_encoding("identity"), TileCompression::None);
    }

    #[test]
    fn best_target_respects_q_values_over_order() {
        // gzip explicitly preferred over br -> gzip wins despite server order.
        assert_eq!(
            best_target_encoding("br;q=0.1, gzip;q=0.9"),
            TileCompression::Gzip
        );
    }

    #[test]
    fn best_target_zstd_when_br_refused() {
        assert_eq!(
            best_target_encoding("br;q=0, zstd, gzip"),
            TileCompression::Zstd
        );
    }

    #[test]
    fn negotiate_missing_header_passes_through_source() {
        let cfg = CompressionConfig::default();
        assert_eq!(
            negotiate(None, TileCompression::Gzip, 5000, &cfg),
            TileCompression::Gzip
        );
    }

    #[test]
    fn negotiate_minimal_recompression_always_passthrough() {
        let cfg = CompressionConfig {
            minimal_recompression: true,
            ..Default::default()
        };
        assert_eq!(
            negotiate(Some("br"), TileCompression::Gzip, 5000, &cfg),
            TileCompression::Gzip
        );
    }

    #[test]
    fn negotiate_passthrough_when_source_encoding_accepted() {
        let cfg = CompressionConfig::default();
        // source gzip + client accepts gzip -> passthrough even though br offered.
        assert_eq!(
            negotiate(Some("br, gzip"), TileCompression::Gzip, 5000, &cfg),
            TileCompression::Gzip
        );
    }

    #[test]
    fn negotiate_transcodes_when_source_not_accepted() {
        let cfg = CompressionConfig::default();
        // source gzip, client wants only br -> transcode to br.
        assert_eq!(
            negotiate(Some("br"), TileCompression::Gzip, 5000, &cfg),
            TileCompression::Brotli
        );
    }

    #[test]
    fn negotiate_identity_request_returns_none() {
        let cfg = CompressionConfig::default();
        assert_eq!(
            negotiate(Some("identity"), TileCompression::Gzip, 5000, &cfg),
            TileCompression::None
        );
    }

    #[test]
    fn negotiate_tiny_tile_skips_compression() {
        let cfg = CompressionConfig::default();
        // 50-byte gzip source, client wants br -> identity (too small to bother).
        assert_eq!(
            negotiate(Some("br"), TileCompression::Gzip, 50, &cfg),
            TileCompression::None
        );
    }

    #[test]
    fn negotiate_none_source_passthrough_when_identity_ok() {
        let cfg = CompressionConfig::default();
        assert_eq!(
            negotiate(Some("identity"), TileCompression::None, 5000, &cfg),
            TileCompression::None
        );
    }

    #[test]
    fn roundtrip_gzip() {
        let cfg = CompressionConfig::default();
        let raw = b"the quick brown fox jumps over the lazy dog".repeat(20);
        let enc = encode(&raw, TileCompression::Gzip, &cfg).unwrap();
        let dec = decode(&enc, TileCompression::Gzip).unwrap();
        assert_eq!(dec, raw);
    }

    #[test]
    fn roundtrip_zstd() {
        let cfg = CompressionConfig::default();
        let raw = b"the quick brown fox jumps over the lazy dog".repeat(20);
        let enc = encode(&raw, TileCompression::Zstd, &cfg).unwrap();
        let dec = decode(&enc, TileCompression::Zstd).unwrap();
        assert_eq!(dec, raw);
    }

    #[test]
    fn roundtrip_brotli() {
        let cfg = CompressionConfig::default();
        let raw = b"the quick brown fox jumps over the lazy dog".repeat(20);
        let enc = encode(&raw, TileCompression::Brotli, &cfg).unwrap();
        let dec = decode(&enc, TileCompression::Brotli).unwrap();
        assert_eq!(dec, raw);
    }

    #[test]
    fn recompress_gzip_to_brotli_preserves_bytes() {
        let cfg = CompressionConfig::default();
        let raw = b"vector tile payload bytes".repeat(50);
        let gz = encode(&raw, TileCompression::Gzip, &cfg).unwrap();
        let br = recompress(&gz, TileCompression::Gzip, TileCompression::Brotli, &cfg).unwrap();
        assert_eq!(decode(&br, TileCompression::Brotli).unwrap(), raw);
    }

    #[test]
    fn recompress_same_encoding_is_noop_passthrough() {
        let cfg = CompressionConfig::default();
        let data = b"already gzip".to_vec();
        let out = recompress(&data, TileCompression::Gzip, TileCompression::Gzip, &cfg).unwrap();
        assert_eq!(out, data);
    }

    #[test]
    fn encode_none_is_identity() {
        let cfg = CompressionConfig::default();
        let raw = b"raw".to_vec();
        assert_eq!(encode(&raw, TileCompression::None, &cfg).unwrap(), raw);
        assert_eq!(decode(&raw, TileCompression::None).unwrap(), raw);
    }

    /// Pseudo-random-ish payload that stays well above `MIN_COMPRESS_BYTES`
    /// even after gzip, so negotiation is not short-circuited by the tiny-tile
    /// rule. Deterministic (no rng dependency) via a simple LCG byte sequence.
    fn incompressible(len: usize) -> Vec<u8> {
        let mut state: u32 = 0x1234_5678;
        (0..len)
            .map(|_| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (state >> 24) as u8
            })
            .collect()
    }

    fn gzip_tile(raw: &[u8]) -> TileData {
        let cfg = CompressionConfig::default();
        TileData {
            data: Bytes::from(encode(raw, TileCompression::Gzip, &cfg).unwrap()),
            format: crate::sources::TileFormat::Pbf,
            compression: TileCompression::Gzip,
        }
    }

    #[test]
    fn recode_gzip_to_brotli_carries_target_encoding() {
        let cfg = CompressionConfig::default();
        let raw = b"vector payload bytes".repeat(40);
        let tile = gzip_tile(&raw);
        let out = recode(&tile, TileCompression::Brotli, &cfg).unwrap();
        assert_eq!(out.compression, TileCompression::Brotli);
        assert_eq!(out.format, crate::sources::TileFormat::Pbf);
        assert_eq!(decode(&out.data, TileCompression::Brotli).unwrap(), raw);
    }

    #[test]
    fn recode_gzip_to_identity_decodes_fully() {
        let cfg = CompressionConfig::default();
        let raw = b"vector payload bytes".repeat(40);
        let tile = gzip_tile(&raw);
        let out = recode(&tile, TileCompression::None, &cfg).unwrap();
        assert_eq!(out.compression, TileCompression::None);
        assert_eq!(out.data.as_ref(), raw.as_slice());
    }

    #[test]
    fn recode_to_same_encoding_is_byte_identical() {
        let cfg = CompressionConfig::default();
        let raw = b"vector payload bytes".repeat(40);
        let tile = gzip_tile(&raw);
        let out = recode(&tile, TileCompression::Gzip, &cfg).unwrap();
        assert_eq!(out.data, tile.data);
    }

    #[test]
    fn negotiate_then_recode_full_flow_to_brotli() {
        let cfg = CompressionConfig::default();
        let raw = incompressible(2048);
        let tile = gzip_tile(&raw);
        assert!(
            tile.data.len() >= MIN_COMPRESS_BYTES,
            "fixture must exceed tiny-tile threshold so negotiation is not short-circuited"
        );
        let target = negotiate(Some("br"), tile.compression, tile.data.len(), &cfg);
        assert_eq!(target, TileCompression::Brotli);
        let out = recode(&tile, target, &cfg).unwrap();
        assert_eq!(decode(&out.data, TileCompression::Brotli).unwrap(), raw);
    }

    #[test]
    fn negotiate_tiny_compressed_tile_short_circuits_to_identity() {
        let cfg = CompressionConfig::default();
        // Highly-repetitive data: gzip output is well under MIN_COMPRESS_BYTES.
        let raw = b"aaaa".repeat(200);
        let tile = gzip_tile(&raw);
        assert!(tile.data.len() < MIN_COMPRESS_BYTES);
        assert_eq!(
            negotiate(Some("br"), tile.compression, tile.data.len(), &cfg),
            TileCompression::None
        );
    }
}
