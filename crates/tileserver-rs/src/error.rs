//! Error types and HTTP status code mapping for the tile server.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum TileServerError {
    #[error("source not found: {0}")]
    SourceNotFound(String),

    #[error("tile not found: z={z}, x={x}, y={y}")]
    TileNotFound { z: u8, x: u32, y: u32 },

    #[error("invalid tile coordinates: z={z}, x={x}, y={y}")]
    InvalidCoordinates { z: u8, x: u32, y: u32 },

    #[error("invalid tile request format")]
    InvalidTileRequest,

    #[error("style not found: {0}")]
    StyleNotFound(String),

    #[error("sprite not found: {0}")]
    SpriteNotFound(String),

    #[error("font not found: {0}")]
    FontNotFound(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("method not allowed: {0}")]
    MethodNotAllowed(String),

    #[error("failed to read file: {0}")]
    FileError(#[from] std::io::Error),

    #[error("failed to parse metadata: {0}")]
    MetadataError(String),

    #[error("configuration error: {0}")]
    ConfigError(String),

    #[error("render error: {0}")]
    RenderError(String),

    #[error("MBTiles error: {0}")]
    MbTilesError(String),

    #[error("MLT encode error: {0}")]
    MltEncodeError(String),

    #[error("MLT decode error: {0}")]
    MltDecodeError(String),

    #[error("transcoding not supported: {from} -> {to}")]
    TranscodeUnsupported { from: String, to: String },

    #[error("compression error: {0}")]
    CompressionError(String),

    #[cfg(feature = "raster")]
    #[error("raster error: {0}")]
    RasterError(String),

    #[cfg(feature = "postgres")]
    #[error("PostgreSQL error: {0}")]
    PostgresError(String),

    #[cfg(feature = "postgres")]
    #[error("PostgreSQL pool error: {0}")]
    PostgresPoolError(String),

    #[cfg(feature = "postgres")]
    #[error("PostgreSQL version error: {0}")]
    PostgresVersionError(String),

    #[cfg(feature = "geoparquet")]
    #[error("GeoParquet error: {0}")]
    GeoParquetError(String),

    #[cfg(feature = "duckdb")]
    #[error("DuckDB error: {0}")]
    DuckDbError(String),

    #[cfg(feature = "stac")]
    #[error("stac error: {0}")]
    StacError(String),

    #[cfg(feature = "sftp")]
    #[error("SFTP authentication error: {0}")]
    SftpAuthError(String),

    #[cfg(feature = "sftp")]
    #[error("SFTP connection error: {0}")]
    SftpConnectionError(String),

    #[cfg(feature = "sftp")]
    #[error("SFTP host key mismatch for {host}: expected {expected}, got {got}")]
    SftpHostKeyMismatch {
        host: String,
        expected: String,
        got: String,
    },

    #[error("upload error: {0}")]
    UploadError(String),

    #[error("file too large")]
    UploadTooLarge,

    #[error("internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for TileServerError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            TileServerError::SourceNotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            TileServerError::TileNotFound { .. } => (StatusCode::NOT_FOUND, self.to_string()),
            TileServerError::InvalidCoordinates { .. } => {
                (StatusCode::BAD_REQUEST, self.to_string())
            }
            TileServerError::InvalidTileRequest => (StatusCode::BAD_REQUEST, self.to_string()),
            TileServerError::StyleNotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            TileServerError::SpriteNotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            TileServerError::FontNotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            TileServerError::NotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            TileServerError::MethodNotAllowed(_) => {
                (StatusCode::METHOD_NOT_ALLOWED, self.to_string())
            }
            TileServerError::FileError(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "File read error".to_string(),
            ),
            TileServerError::MetadataError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
            TileServerError::ConfigError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
            TileServerError::RenderError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
            TileServerError::MbTilesError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
            TileServerError::MltEncodeError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
            TileServerError::MltDecodeError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
            TileServerError::TranscodeUnsupported { .. } => {
                (StatusCode::BAD_REQUEST, self.to_string())
            }
            TileServerError::CompressionError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
            #[cfg(feature = "raster")]
            TileServerError::RasterError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
            #[cfg(feature = "postgres")]
            TileServerError::PostgresError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
            #[cfg(feature = "postgres")]
            TileServerError::PostgresPoolError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
            #[cfg(feature = "postgres")]
            TileServerError::PostgresVersionError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
            #[cfg(feature = "geoparquet")]
            TileServerError::GeoParquetError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
            #[cfg(feature = "duckdb")]
            TileServerError::DuckDbError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
            #[cfg(feature = "stac")]
            TileServerError::StacError(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            TileServerError::UploadError(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            TileServerError::UploadTooLarge => (StatusCode::PAYLOAD_TOO_LARGE, self.to_string()),
            #[cfg(feature = "sftp")]
            TileServerError::SftpAuthError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
            #[cfg(feature = "sftp")]
            TileServerError::SftpConnectionError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
            #[cfg(feature = "sftp")]
            TileServerError::SftpHostKeyMismatch { .. } => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
            TileServerError::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
        };

        (status, message).into_response()
    }
}

pub type Result<T> = std::result::Result<T, TileServerError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_not_found_display() {
        let err = TileServerError::SourceNotFound("osm".to_string());
        assert_eq!(err.to_string(), "source not found: osm");
    }

    #[test]
    fn test_tile_not_found_display() {
        let err = TileServerError::TileNotFound {
            z: 14,
            x: 100,
            y: 200,
        };
        assert_eq!(err.to_string(), "tile not found: z=14, x=100, y=200");
    }

    #[test]
    fn test_invalid_coordinates_display() {
        let err = TileServerError::InvalidCoordinates { z: 1, x: 5, y: 5 };
        assert_eq!(err.to_string(), "invalid tile coordinates: z=1, x=5, y=5");
    }

    #[test]
    fn test_invalid_tile_request_display() {
        let err = TileServerError::InvalidTileRequest;
        assert_eq!(err.to_string(), "invalid tile request format");
    }

    #[test]
    fn test_style_not_found_display() {
        let err = TileServerError::StyleNotFound("bright".to_string());
        assert_eq!(err.to_string(), "style not found: bright");
    }

    #[test]
    fn test_config_error_display() {
        let err = TileServerError::ConfigError("bad toml".to_string());
        assert_eq!(err.to_string(), "configuration error: bad toml");
    }

    #[test]
    fn test_upload_too_large_display() {
        let err = TileServerError::UploadTooLarge;
        assert_eq!(err.to_string(), "file too large");
    }

    #[test]
    fn test_transcode_unsupported_display() {
        let err = TileServerError::TranscodeUnsupported {
            from: "mlt".to_string(),
            to: "geojson".to_string(),
        };
        assert_eq!(err.to_string(), "transcoding not supported: mlt -> geojson");
    }

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
        let err: TileServerError = io_err.into();
        assert!(matches!(err, TileServerError::FileError(_)));
    }

    #[test]
    fn test_from_anyhow_error() {
        let anyhow_err = anyhow::anyhow!("something went wrong");
        let err: TileServerError = anyhow_err.into();
        assert!(matches!(err, TileServerError::Internal(_)));
    }

    #[test]
    fn test_source_not_found_status_code() {
        let err = TileServerError::SourceNotFound("x".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_invalid_coordinates_status_code() {
        let err = TileServerError::InvalidCoordinates { z: 0, x: 0, y: 0 };
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_upload_too_large_status_code() {
        let err = TileServerError::UploadTooLarge;
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[test]
    fn test_file_error_status_code() {
        let err = TileServerError::FileError(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "denied",
        ));
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_transcode_unsupported_status_code() {
        let err = TileServerError::TranscodeUnsupported {
            from: "a".to_string(),
            to: "b".to_string(),
        };
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_compression_error_display_and_status() {
        let err = TileServerError::CompressionError("brotli encode: oom".to_string());
        assert_eq!(err.to_string(), "compression error: brotli encode: oom");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_upload_error_status_code() {
        let err = TileServerError::UploadError("bad file".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_render_error_status_code() {
        let err = TileServerError::RenderError("renderer crashed".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_sprite_not_found_display_and_status() {
        let err = TileServerError::SpriteNotFound("icon".to_string());
        assert_eq!(err.to_string(), "sprite not found: icon");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_font_not_found_display_and_status() {
        let err = TileServerError::FontNotFound("Roboto".to_string());
        assert_eq!(err.to_string(), "font not found: Roboto");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_not_found_display_and_status() {
        let err = TileServerError::NotFound("/foo".to_string());
        assert_eq!(err.to_string(), "not found: /foo");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_method_not_allowed_display_and_status() {
        let err = TileServerError::MethodNotAllowed("POST".to_string());
        assert_eq!(err.to_string(), "method not allowed: POST");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[test]
    fn test_metadata_error_display_and_status() {
        let err = TileServerError::MetadataError("bad json".to_string());
        assert_eq!(err.to_string(), "failed to parse metadata: bad json");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_config_error_status_code() {
        let err = TileServerError::ConfigError("bad config".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_mbtiles_error_display_and_status() {
        let err = TileServerError::MbTilesError("sqlite locked".to_string());
        assert_eq!(err.to_string(), "MBTiles error: sqlite locked");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_mlt_encode_error_display_and_status() {
        let err = TileServerError::MltEncodeError("layer overflow".to_string());
        assert_eq!(err.to_string(), "MLT encode error: layer overflow");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_mlt_decode_error_display_and_status() {
        let err = TileServerError::MltDecodeError("bad header".to_string());
        assert_eq!(err.to_string(), "MLT decode error: bad header");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_internal_error_status_code() {
        let err = TileServerError::Internal(anyhow::anyhow!("kaboom"));
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_tile_not_found_status_code() {
        let err = TileServerError::TileNotFound { z: 1, x: 2, y: 3 };
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_invalid_tile_request_status_code() {
        let err = TileServerError::InvalidTileRequest;
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_style_not_found_status_code() {
        let err = TileServerError::StyleNotFound("dark".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[cfg(feature = "raster")]
    #[test]
    fn test_raster_error_display_and_status() {
        let err = TileServerError::RasterError("gdal failed".to_string());
        assert_eq!(err.to_string(), "raster error: gdal failed");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[cfg(feature = "postgres")]
    #[test]
    fn test_postgres_error_variants() {
        let err = TileServerError::PostgresError("conn refused".to_string());
        assert_eq!(err.to_string(), "PostgreSQL error: conn refused");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let err = TileServerError::PostgresPoolError("exhausted".to_string());
        assert_eq!(err.to_string(), "PostgreSQL pool error: exhausted");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let err = TileServerError::PostgresVersionError("too old".to_string());
        assert_eq!(err.to_string(), "PostgreSQL version error: too old");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[cfg(feature = "duckdb")]
    #[test]
    fn test_duckdb_error_display_and_status() {
        let err = TileServerError::DuckDbError("syntax".to_string());
        assert_eq!(err.to_string(), "DuckDB error: syntax");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[cfg(feature = "geoparquet")]
    #[test]
    fn test_geoparquet_error_display_and_status() {
        let err = TileServerError::GeoParquetError("parse fail".to_string());
        assert_eq!(err.to_string(), "GeoParquet error: parse fail");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[cfg(feature = "stac")]
    #[test]
    fn test_stac_error_display_and_status() {
        let err = TileServerError::StacError("api 500".to_string());
        assert_eq!(err.to_string(), "stac error: api 500");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
