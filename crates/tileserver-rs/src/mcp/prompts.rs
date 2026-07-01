//! MCP prompt templates exposed via the `prompts/list` and `prompts/get`
//! endpoints.
//!
//! Each prompt is a small, named template that an LLM can fill in by
//! invoking `prompts/get` with a JSON `arguments` object. The handler
//! returns a single user-role text message with the placeholders
//! substituted. Prompts NEVER invoke tools themselves — they are
//! suggestions the model can use to scaffold its own tool calls.
//!
//! Spec: <https://modelcontextprotocol.io/specification/2025-06-18/server/prompts>

use rmcp::ErrorData as McpError;
use rmcp::model::{
    GetPromptRequestParams, GetPromptResult, ListPromptsResult, Prompt, PromptArgument,
    PromptMessage, Role,
};
use serde_json::Value;

/// Default zoom for `render_location_preview` when the caller omits the
/// `zoom` argument.
const DEFAULT_ZOOM: u32 = 12;

/// Build the static list of prompt templates the server exposes.
///
/// The list never changes at runtime, so we return all four prompts in
/// a single page (`next_cursor` is `None`).
#[must_use]
pub fn list_prompts() -> ListPromptsResult {
    let prompts: Vec<Prompt> = vec![
        Prompt::new(
            "describe_style",
            Some(
                "Ask the model to describe a registered MapLibre style — its layer structure, \
                 color palette, and intended use case.",
            ),
            Some(vec![
                PromptArgument::new("style_id")
                    .with_description("Style id as returned by `tileserver_list_styles`.")
                    .with_required(true),
            ]),
        ),
        Prompt::new(
            "suggest_cql2_filter",
            Some(
                "Ask the model to translate a natural-language intent into a CQL2 text filter \
                 expression against a table source's queryables.",
            ),
            Some(vec![
                PromptArgument::new("table_id")
                    .with_description("Table source id (must be a PostgreSQL table source).")
                    .with_required(true),
                PromptArgument::new("intent")
                    .with_description("Plain-English description of the features to find.")
                    .with_required(true),
            ]),
        ),
        Prompt::new(
            "render_location_preview",
            Some(
                "Ask the model to render a static map preview of a named location using \
                 `tileserver_render_static_map`.",
            ),
            Some(vec![
                PromptArgument::new("location")
                    .with_description("Human-readable place name or address (e.g. \"Tokyo\").")
                    .with_required(true),
                PromptArgument::new("zoom")
                    .with_description("MapLibre zoom level 0-22 (defaults to 12 if omitted).")
                    .with_required(false),
            ]),
        ),
        Prompt::new(
            "explain_tile_metadata",
            Some(
                "Ask the model to explain the structure of a tile source — its layers, zoom \
                 range, bounds, and the data each layer holds.",
            ),
            Some(vec![
                PromptArgument::new("source_id")
                    .with_description("Source id as returned by `tileserver_list_sources`.")
                    .with_required(true),
            ]),
        ),
    ];

    ListPromptsResult::with_all_items(prompts)
}

/// Resolve a prompt request to a fully-substituted [`GetPromptResult`].
///
/// # Errors
///
/// Returns [`McpError::invalid_params`] (JSON-RPC code `-32602`) when:
///
/// - The prompt name is not one of the four registered prompts; OR
/// - A required argument is missing from `request.arguments`.
pub fn get_prompt(request: &GetPromptRequestParams) -> Result<GetPromptResult, McpError> {
    match request.name.as_str() {
        "describe_style" => {
            let style_id = required_arg(request, "style_id")?;
            let text = format!(
                "Describe the visual design and intended use case of the map style currently \
                 loaded as `{style_id}`. Reference its layer structure and color palette."
            );
            Ok(text_prompt("Describe a registered MapLibre style.", text))
        }
        "suggest_cql2_filter" => {
            let table_id = required_arg(request, "table_id")?;
            let intent = required_arg(request, "intent")?;
            let text = format!(
                "I want to find features in table `{table_id}` matching this intent: \
                 `{intent}`. Suggest a CQL2 text filter expression that satisfies the intent. \
                 Use only fields documented in the table's queryables."
            );
            Ok(text_prompt(
                "Translate a natural-language intent into a CQL2 filter.",
                text,
            ))
        }
        "render_location_preview" => {
            let location = required_arg(request, "location")?;
            let zoom = optional_arg(request, "zoom").unwrap_or_else(|| DEFAULT_ZOOM.to_string());
            let text = format!(
                "Render a static map preview of `{location}` at zoom `{zoom}`. Use \
                 `tileserver_render_static_map` with the first available style."
            );
            Ok(text_prompt(
                "Render a static map preview of a named location.",
                text,
            ))
        }
        "explain_tile_metadata" => {
            let source_id = required_arg(request, "source_id")?;
            let text = format!(
                "Explain the structure and contents of source `{source_id}` based on \
                 `tileserver_get_tile_metadata`. Describe its layers, zoom range, bounds, and \
                 what types of data each layer holds."
            );
            Ok(text_prompt(
                "Explain the layer structure of a tile source.",
                text,
            ))
        }
        other => Err(McpError::invalid_params(
            format!("unknown prompt: {other}"),
            None,
        )),
    }
}

/// Read a required argument from `request.arguments`.
///
/// Coerces strings, numbers, and booleans to their `Display` representation
/// so callers can use either `{"zoom": 12}` or `{"zoom": "12"}` interchangeably.
fn required_arg(request: &GetPromptRequestParams, key: &str) -> Result<String, McpError> {
    match request.arguments.as_ref().and_then(|m| m.get(key)) {
        Some(v) => coerce_to_string(v).ok_or_else(|| {
            McpError::invalid_params(
                format!("argument `{key}` must be a string, number, or boolean"),
                None,
            )
        }),
        None => Err(McpError::invalid_params(
            format!("missing required argument `{key}`"),
            None,
        )),
    }
}

/// Read an optional argument from `request.arguments`, returning `None`
/// when absent or when the value cannot be coerced to a string.
fn optional_arg(request: &GetPromptRequestParams, key: &str) -> Option<String> {
    request
        .arguments
        .as_ref()
        .and_then(|m| m.get(key))
        .and_then(coerce_to_string)
}

fn coerce_to_string(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Null | Value::Array(_) | Value::Object(_) => None,
    }
}

fn text_prompt(description: &str, text: String) -> GetPromptResult {
    GetPromptResult::new(vec![PromptMessage::new_text(Role::User, text)])
        .with_description(description.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Map, json};

    fn req(name: &str, args: Value) -> GetPromptRequestParams {
        let map = args.as_object().cloned().unwrap_or_else(Map::new);
        GetPromptRequestParams::new(name).with_arguments(map)
    }

    #[test]
    fn list_returns_four_prompts() {
        let result = list_prompts();
        assert_eq!(result.prompts.len(), 4);
        assert!(result.next_cursor.is_none());
    }

    #[test]
    fn describe_style_substitutes_id() {
        let r = get_prompt(&req("describe_style", json!({ "style_id": "demo" })))
            .expect("get_prompt succeeded");
        assert_eq!(r.messages.len(), 1);
        match &r.messages[0].content {
            rmcp::model::ContentBlock::Text(text_content) => {
                let text = &text_content.text;
                assert!(text.contains("demo"), "style_id not substituted: {text}");
            }
            other => panic!("expected text content, got {other:?}"),
        }
    }

    #[test]
    fn render_location_preview_uses_default_zoom() {
        let r = get_prompt(&req(
            "render_location_preview",
            json!({ "location": "Mumbai" }),
        ))
        .expect("get_prompt succeeded");
        match &r.messages[0].content {
            rmcp::model::ContentBlock::Text(text_content) => {
                let text = &text_content.text;
                assert!(text.contains("Mumbai"));
                assert!(text.contains("12"), "default zoom not in `{text}`");
            }
            other => panic!("expected text content, got {other:?}"),
        }
    }

    #[test]
    fn render_location_preview_respects_numeric_zoom() {
        let r = get_prompt(&req(
            "render_location_preview",
            json!({ "location": "Berlin", "zoom": 5 }),
        ))
        .expect("get_prompt succeeded");
        match &r.messages[0].content {
            rmcp::model::ContentBlock::Text(text_content) => {
                let text = &text_content.text;
                assert!(text.contains("5"));
            }
            other => panic!("expected text content, got {other:?}"),
        }
    }

    #[test]
    fn missing_required_arg_returns_invalid_params() {
        let err = get_prompt(&req("describe_style", json!({}))).expect_err("must error");
        assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_PARAMS);
    }

    #[test]
    fn unknown_prompt_returns_invalid_params() {
        let err = get_prompt(&req("ghost", json!({}))).expect_err("must error");
        assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_PARAMS);
    }
}
