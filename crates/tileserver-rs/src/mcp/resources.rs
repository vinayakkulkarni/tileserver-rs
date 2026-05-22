//! MCP resource templates for styles and TileJSON metadata.
//!
//! Two read-only URI schemes are exposed:
//!
//! - `tileserver://styles/{id}` — full MapLibre style JSON for a registered style.
//! - `tileserver://data/{id}.json` — TileJSON 3.0 metadata for a registered source.
//!
//! Resources are *application-controlled* in the MCP model: the client may
//! list and read them but the LLM does not invoke them as actions. Mutating
//! operations (rendering, queries) live in [`crate::mcp::handlers`] as tools.

use std::sync::Arc;

use rmcp::ErrorData as McpError;
use rmcp::model::{
    ListResourceTemplatesResult, RawResourceTemplate, ReadResourceResult, ResourceContents,
    ResourceTemplate,
};

use crate::reload::AppState;

/// URI scheme used for both resource templates.
const URI_SCHEME: &str = "tileserver://";

/// Resource template URI for style JSON.
const STYLE_URI_TEMPLATE: &str = "tileserver://styles/{id}";

/// Resource template URI for TileJSON metadata.
const DATA_URI_TEMPLATE: &str = "tileserver://data/{id}.json";

/// Build the static list of resource templates the server exposes.
///
/// Returned once per `resources/templates/list` request. The list never
/// changes at runtime — sources/styles can be added or removed via hot
/// reload, but the templates themselves are constant.
#[must_use]
pub fn list_resource_templates() -> ListResourceTemplatesResult {
    let templates: Vec<ResourceTemplate> = vec![
        ResourceTemplate::new(
            RawResourceTemplate::new(STYLE_URI_TEMPLATE, "style")
                .with_title("MapLibre style JSON")
                .with_description(
                    "Returns the raw style.json for a registered map style. \
                     Substitute {id} with the style id from tileserver_list_styles.",
                )
                .with_mime_type("application/json"),
            None,
        ),
        ResourceTemplate::new(
            RawResourceTemplate::new(DATA_URI_TEMPLATE, "tilejson")
                .with_title("TileJSON 3.0 metadata")
                .with_description(
                    "Returns TileJSON 3.0 metadata (bounds, zoom range, tile URL template) \
                     for a registered tile source. Substitute {id} with the source id \
                     from tileserver_list_sources.",
                )
                .with_mime_type("application/json"),
            None,
        ),
    ];

    ListResourceTemplatesResult::with_all_items(templates)
}

/// Resolve a resource URI to its contents.
///
/// # Errors
///
/// Returns [`McpError::resource_not_found`] when:
/// - the URI does not match either supported template
/// - the referenced source or style id is not registered
/// - the URI is otherwise malformed (e.g. extra path segments)
pub fn read_resource(uri: &str, state: &Arc<AppState>) -> Result<ReadResourceResult, McpError> {
    let Some(rest) = uri.strip_prefix(URI_SCHEME) else {
        return Err(McpError::resource_not_found(
            format!("unsupported URI scheme: {uri}"),
            None,
        ));
    };

    if let Some(style_id) = rest.strip_prefix("styles/") {
        return read_style(style_id, uri, state);
    }

    if let Some(rest) = rest.strip_prefix("data/")
        && let Some(source_id) = rest.strip_suffix(".json")
    {
        return read_tilejson(source_id, uri, state);
    }

    Err(McpError::resource_not_found(
        format!("URI does not match any known template: {uri}"),
        None,
    ))
}

fn read_style(
    style_id: &str,
    uri: &str,
    state: &Arc<AppState>,
) -> Result<ReadResourceResult, McpError> {
    if style_id.is_empty() || style_id.contains('/') {
        return Err(McpError::resource_not_found(
            format!("invalid style id in URI: {uri}"),
            None,
        ));
    }

    let style = state.styles.get(style_id).ok_or_else(|| {
        McpError::resource_not_found(format!("style not found: {style_id}"), None)
    })?;

    let body = serde_json::to_string(&style.style_json).map_err(|e| {
        McpError::internal_error(format!("failed to serialize style JSON: {e}"), None)
    })?;

    Ok(ReadResourceResult::new(vec![
        ResourceContents::text(body, uri.to_string()).with_mime_type("application/json"),
    ]))
}

fn read_tilejson(
    source_id: &str,
    uri: &str,
    state: &Arc<AppState>,
) -> Result<ReadResourceResult, McpError> {
    if source_id.is_empty() || source_id.contains('/') {
        return Err(McpError::resource_not_found(
            format!("invalid source id in URI: {uri}"),
            None,
        ));
    }

    let source = state.sources.get(source_id).ok_or_else(|| {
        McpError::resource_not_found(format!("source not found: {source_id}"), None)
    })?;

    let tilejson = source.metadata().to_tilejson(&state.base_url);
    let body = serde_json::to_string(&tilejson).map_err(|e| {
        McpError::internal_error(format!("failed to serialize TileJSON: {e}"), None)
    })?;

    Ok(ReadResourceResult::new(vec![
        ResourceContents::text(body, uri.to_string()).with_mime_type("application/json"),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reload::AppState;
    use crate::sources::SourceManager;
    use crate::styles::StyleManager;

    fn minimal_state() -> Arc<AppState> {
        Arc::new(AppState {
            sources: Arc::new(SourceManager::new()),
            styles: Arc::new(StyleManager::new()),
            renderer: None,
            base_url: "http://localhost:8080".into(),
            render_base_url: "http://127.0.0.1:8080".into(),
            ui_enabled: false,
            fonts_dir: None,
            files_dir: None,
            upload_dir: None,
        })
    }

    #[test]
    fn list_returns_both_templates() {
        let result = list_resource_templates();
        assert_eq!(result.resource_templates.len(), 2);
        let uris: Vec<&str> = result
            .resource_templates
            .iter()
            .map(|t| t.uri_template.as_str())
            .collect();
        assert!(uris.contains(&STYLE_URI_TEMPLATE));
        assert!(uris.contains(&DATA_URI_TEMPLATE));
    }

    #[test]
    fn unsupported_scheme_returns_not_found() {
        let state = minimal_state();
        let err = read_resource("http://example.com/foo", &state)
            .expect_err("non-tileserver URI must be rejected");
        let _: &McpError = &err;
    }

    #[test]
    fn unknown_style_returns_not_found() {
        let state = minimal_state();
        let err = read_resource("tileserver://styles/ghost", &state)
            .expect_err("unknown style must be rejected");
        let _: &McpError = &err;
    }

    #[test]
    fn unknown_source_returns_not_found() {
        let state = minimal_state();
        let err = read_resource("tileserver://data/ghost.json", &state)
            .expect_err("unknown source must be rejected");
        let _: &McpError = &err;
    }

    #[test]
    fn invalid_template_returns_not_found() {
        let state = minimal_state();
        assert!(read_resource("tileserver://", &state).is_err());
        assert!(read_resource("tileserver://other/foo", &state).is_err());
        assert!(read_resource("tileserver://styles/foo/bar", &state).is_err());
        assert!(read_resource("tileserver://data/foo", &state).is_err());
    }
}
