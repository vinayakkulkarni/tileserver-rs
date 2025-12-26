use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

#[derive(Error, Debug)]
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

    #[error("Failed to read file: {0}")]
    FileError(#[from] std::io::Error),

    #[error("Failed to parse metadata: {0}")]
    MetadataError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Render error: {0}")]
    RenderError(String),

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
            TileServerError::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
        };

        (status, message).into_response()
    }
}

pub type Result<T> = std::result::Result<T, TileServerError>;
