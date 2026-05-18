//! Error mapping from [`TileServerError`] to MCP `CallToolResult`.
//!
//! MCP draws a hard line between two kinds of failures (per
//! [SEP-2140](https://github.com/modelcontextprotocol/specification/discussions/2140)):
//!
//! - **Protocol errors** — malformed requests, unknown methods, transport
//!   failures. These return a JSON-RPC error object.
//! - **Tool execution errors** — anything that goes wrong *inside* a tool
//!   after the request was validly dispatched (source not found, render
//!   failure, database timeout). These return `CallToolResult { is_error:
//!   Some(true), content: [...] }` so the LLM can read the error text and
//!   decide whether to retry or surface it to the user.
//!
//! Every tool in this crate funnels backend failures through
//! [`tile_error_to_call_result`] to keep that contract uniform.

use rmcp::model::{CallToolResult, Content};

use crate::error::TileServerError;

/// Convert a [`TileServerError`] into a `CallToolResult` flagged as a tool
/// execution error.
///
/// The resulting result has `is_error: Some(true)` and a single text content
/// item carrying the `Display` impl of the error. Use this from inside an MCP
/// tool handler whenever a backend call returns `Err`.
///
/// # Example
///
/// ```ignore
/// use tileserver_rs::mcp::error::tile_error_to_call_result;
///
/// let result = match self.state.sources.get_tile("missing", 0, 0, 0).await {
///     Ok(Some(tile)) => CallToolResult::success(vec![/* ... */]),
///     Ok(None) => tile_error_to_call_result(TileServerError::TileNotFound { z: 0, x: 0, y: 0 }),
///     Err(e) => tile_error_to_call_result(e),
/// };
/// ```
#[must_use]
pub fn tile_error_to_call_result(err: TileServerError) -> CallToolResult {
    CallToolResult::error(vec![Content::text(err.to_string())])
}

/// Build a tool-execution error from a free-form message.
///
/// Use for validation failures that do not have a corresponding
/// [`TileServerError`] variant (e.g. "renderer is disabled", "dimension
/// exceeds limit").
#[must_use]
pub fn tool_error<S: Into<String>>(message: S) -> CallToolResult {
    CallToolResult::error(vec![Content::text(message.into())])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_source_not_found_to_is_error() {
        let result = tile_error_to_call_result(TileServerError::SourceNotFound("foo".into()));
        assert_eq!(result.is_error, Some(true));
        assert_eq!(result.content.len(), 1);
        let text = result.content[0].as_text().expect("expected text content");
        assert!(text.text.contains("source not found"));
        assert!(text.text.contains("foo"));
    }

    #[test]
    fn maps_tile_not_found_to_is_error() {
        let result = tile_error_to_call_result(TileServerError::TileNotFound { z: 5, x: 1, y: 2 });
        assert_eq!(result.is_error, Some(true));
        let text = &result.content[0]
            .as_text()
            .expect("expected text content")
            .text;
        assert!(text.contains("z=5"));
        assert!(text.contains("x=1"));
        assert!(text.contains("y=2"));
    }

    #[test]
    fn maps_style_not_found_to_is_error() {
        let result = tile_error_to_call_result(TileServerError::StyleNotFound("bright".into()));
        assert_eq!(result.is_error, Some(true));
        assert!(
            result.content[0]
                .as_text()
                .expect("text")
                .text
                .contains("bright")
        );
    }

    #[test]
    fn maps_render_error_to_is_error() {
        let result = tile_error_to_call_result(TileServerError::RenderError("boom".into()));
        assert_eq!(result.is_error, Some(true));
    }

    #[test]
    fn maps_invalid_coordinates_to_is_error() {
        let result =
            tile_error_to_call_result(TileServerError::InvalidCoordinates { z: 30, x: 0, y: 0 });
        assert_eq!(result.is_error, Some(true));
    }

    #[test]
    fn maps_internal_error_to_is_error() {
        let result =
            tile_error_to_call_result(TileServerError::Internal(anyhow::anyhow!("kaboom")));
        assert_eq!(result.is_error, Some(true));
        assert!(
            result.content[0]
                .as_text()
                .expect("text")
                .text
                .contains("kaboom")
        );
    }

    #[test]
    fn tool_error_helper_carries_message() {
        let result = tool_error("renderer disabled");
        assert_eq!(result.is_error, Some(true));
        assert_eq!(
            result.content[0].as_text().expect("text").text,
            "renderer disabled"
        );
    }
}
