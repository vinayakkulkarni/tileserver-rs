//! Embeddable iframe map route handler (`GET /embed/{style}`).
//!
//! Serves a self-contained HTML page that boots MapLibre GL JS in an iframe.
//! This route is intentionally **not** gated by `disable_render`: it emits
//! HTML only and never touches the native renderer.

use std::collections::HashMap;

use axum::{
    extract::{Path, Query, State},
    http::{
        HeaderMap, HeaderValue,
        header::{CACHE_CONTROL, CONTENT_TYPE},
    },
    response::{IntoResponse, Response},
};

use crate::embed::{build_embed_html, parse_embed_query};
use crate::error::TileServerError;
use crate::reload::SharedState;

/// Serve the embeddable iframe HTML page for a style.
///
/// # Errors
///
/// Returns [`TileServerError::StyleNotFound`] (404) when the style id is
/// unknown, or [`TileServerError::InvalidTileRequest`] (400) when a query
/// parameter fails validation.
pub(crate) async fn get_embed(
    State(shared): State<SharedState>,
    Path(style): Path<String>,
    Query(raw): Query<HashMap<String, String>>,
) -> Result<Response, TileServerError> {
    let state = shared.load();

    let Some(loaded) = state.styles.get(&style) else {
        return Err(TileServerError::StyleNotFound(style));
    };

    let params = parse_embed_query(&raw)?;
    let html = build_embed_html(&params, &style, &state.base_url, &loaded.name);

    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    headers.insert(
        CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=300"),
    );

    Ok((headers, html).into_response())
}
