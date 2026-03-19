use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum TileServerError {
    #[error("Source not found: {0}")]
    SourceNotFound(String),

    #[error("Tile not found: z={z}, x={x}, y={y}")]
    TileNotFound { z: u8, x: u32, y: u32 },

    #[error("Invalid tile coordinates: z={z}, x={x}, y={y}")]
    InvalidCoordinates { z: u8, x: u32, y: u32 },

    #[error("Invalid tile request format")]
    InvalidTileRequest,

    #[error("Style not found: {0}")]
    StyleNotFound(String),

    #[error("Sprite not found: {0}")]
    SpriteNotFound(String),

    #[error("Font not found: {0}")]
    FontNotFound(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Failed to read file: {0}")]
    FileError(#[from] std::io::Error),

    #[error("Failed to parse metadata: {0}")]
    MetadataError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Render error: {0}")]
    RenderError(String),

    #[error("MBTiles error: {0}")]
    MbTilesError(String),

    #[error("MLT encode error: {0}")]
    MltEncodeError(String),

    #[error("MLT decode error: {0}")]
    MltDecodeError(String),

    #[error("transcoding not supported: {from} -> {to}")]
    TranscodeUnsupported { from: String, to: String },

    #[cfg(feature = "raster")]
    #[error("Raster error: {0}")]
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

    #[error("upload error: {0}")]
    UploadError(String),

    #[error("file too large")]
    UploadTooLarge,

    #[error("Internal error: {0}")]
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
            TileServerError::UploadError(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            TileServerError::UploadTooLarge => (StatusCode::PAYLOAD_TOO_LARGE, self.to_string()),
            TileServerError::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
        };

        (status, message).into_response()
    }
}

pub type Result<T> = std::result::Result<T, TileServerError>;
