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
//!
//! # Path / secret redaction
//!
//! [`TileServerError::FileError`] wraps [`std::io::Error`] whose `Display`
//! embeds the OS path the IO operation was attempted on (e.g.
//! `/var/lib/tileserver/private.key`). Surfacing that to an MCP client
//! would hand a misbehaving LLM a directory enumeration primitive. We
//! redact the path before returning, replacing the body with a generic
//! "the configured file path could not be read" message. The full original
//! error is still logged server-side via `tracing::warn!` so operators can
//! debug.

use rmcp::model::{CallToolResult, Content};

use crate::error::TileServerError;

/// Convert a [`TileServerError`] into a `CallToolResult` flagged as a tool
/// execution error.
///
/// The resulting result has `is_error: Some(true)` and a single text content
/// item carrying the `Display` impl of the error — except for
/// [`TileServerError::FileError`], whose underlying [`std::io::Error`]
/// Display leaks filesystem paths and is rewritten to a generic message
/// before returning. The original error is logged at `warn` level.
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
    let body = match err {
        TileServerError::FileError(ref io_err) => {
            tracing::warn!(
                error = %io_err,
                "MCP tool encountered FileError; redacted from response to caller"
            );
            "the configured file path could not be read (see server logs for details)".to_string()
        }
        _ => err.to_string(),
    };
    CallToolResult::error(vec![Content::text(body)])
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

    #[test]
    fn file_error_redacts_filesystem_path_from_response() {
        let secret_path = "/var/lib/tileserver/private/jwt-signing.key";
        let io_err = std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!("failed to read {secret_path}: permission denied"),
        );
        let result = tile_error_to_call_result(TileServerError::FileError(io_err));

        assert_eq!(result.is_error, Some(true));
        let text = &result.content[0].as_text().expect("text").text;
        assert!(
            !text.contains(secret_path),
            "FileError response must NOT leak the OS path; got: {text}",
        );
        assert!(
            !text.contains("permission denied"),
            "FileError response must NOT leak the underlying IO kind/message; got: {text}",
        );
        assert!(
            text.contains("could not be read"),
            "FileError response should carry the generic redaction message; got: {text}",
        );
    }
}
