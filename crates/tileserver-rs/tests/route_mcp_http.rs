//! Integration tests for the MCP Streamable HTTP transport.
//!
//! Exercises:
//! - `initialize` handshake → `Mcp-Session-Id` header is returned.
//! - `tools/list` includes the Tier A tool set.
//! - `tools/call` of `tileserver_list_sources` returns an array.
//! - `tools/call` of `tileserver_render_static_map` returns
//!   `is_error: true` when no renderer is configured.
//! - `resources/read` of a style URI returns JSON contents.
//! - Bearer auth: missing token is rejected, valid token is accepted.
//!
//! Responses are SSE-framed (the default for stateful mode); a small
//! helper [`parse_sse_json`] extracts the `data:` line and parses it as
//! JSON for assertion.

#![cfg(feature = "mcp")]

mod common;

use std::sync::Arc;

use axum::Router;
use axum::extract::Request;
use axum::http::{StatusCode, header::AUTHORIZATION};
use axum::middleware::{self, Next};
use axum::response::Response;
use axum_test::TestServer;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::tower::{
    StreamableHttpServerConfig, StreamableHttpService,
};
use serde_json::{Value, json};
use tileserver_rs::mcp::McpHandler;
use tileserver_rs::reload::SharedState;

const MCP_ACCEPT: &str = "application/json, text/event-stream";
const PROTOCOL_VERSION: &str = "2025-03-26";

/// Parse the most recent `data:` line out of an SSE response body and
/// return it as JSON. Panics on malformed input — tests already control
/// the inputs so this is acceptable.
fn parse_sse_json(body: &str) -> Value {
    let line = body
        .lines()
        .filter_map(|l| l.strip_prefix("data:").map(str::trim))
        .rfind(|l| !l.is_empty())
        .unwrap_or_else(|| panic!("no `data:` line in SSE body: {body:?}"));
    serde_json::from_str(line).unwrap_or_else(|e| panic!("invalid JSON `{line}`: {e}"))
}

fn mcp_test_router(shared: SharedState, auth_token: Option<String>) -> Router {
    let factory_state = shared;
    let svc = StreamableHttpService::new(
        move || Ok(McpHandler::new(factory_state.load())),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default().disable_allowed_hosts(),
    );
    let mut router = Router::new().nest_service("/mcp", svc);
    if let Some(token) = auth_token {
        let state = TestAuthToken(Arc::new(token));
        router = router.layer(middleware::from_fn_with_state(state, test_bearer_auth));
    }
    router
}

#[derive(Clone)]
struct TestAuthToken(Arc<String>);

async fn test_bearer_auth(
    axum::extract::State(token): axum::extract::State<TestAuthToken>,
    req: Request,
    next: Next,
) -> std::result::Result<Response, StatusCode> {
    let provided = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    if provided == Some(token.0.as_str()) {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

fn empty_mcp_server() -> TestServer {
    let shared = common::minimal_shared_state();
    let router = mcp_test_router(shared, None);
    TestServer::new(router)
}

fn initialize_payload() -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": {
                "name": "tileserver-rs-tests",
                "version": "0.0.0"
            }
        }
    })
}

#[tokio::test]
async fn mcp_initialize_returns_server_info() {
    let server = empty_mcp_server();
    let resp = server
        .post("/mcp")
        .add_header("accept", MCP_ACCEPT)
        .json(&initialize_payload())
        .await;

    resp.assert_status_ok();
    let body = parse_sse_json(&resp.text());
    assert_eq!(body["result"]["serverInfo"]["name"], "tileserver-rs");
    assert!(
        body["result"]["capabilities"].is_object(),
        "capabilities missing: {body}"
    );
}

async fn initialize_and_get_session_id(server: &TestServer) -> String {
    let resp = server
        .post("/mcp")
        .add_header("accept", MCP_ACCEPT)
        .json(&initialize_payload())
        .await;
    resp.assert_status_ok();
    let session_id = resp
        .headers()
        .get("mcp-session-id")
        .expect("server must return Mcp-Session-Id header")
        .to_str()
        .expect("header is ASCII")
        .to_string();
    let initialized = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });
    let _ = server
        .post("/mcp")
        .add_header("accept", MCP_ACCEPT)
        .add_header("mcp-session-id", &session_id)
        .json(&initialized)
        .await;
    session_id
}

#[tokio::test]
async fn mcp_tools_list_includes_tier_a() {
    let server = empty_mcp_server();
    let session_id = initialize_and_get_session_id(&server).await;
    let resp = server
        .post("/mcp")
        .add_header("accept", MCP_ACCEPT)
        .add_header("mcp-session-id", &session_id)
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        }))
        .await;
    resp.assert_status_ok();
    let body = parse_sse_json(&resp.text());
    let tools = body["result"]["tools"]
        .as_array()
        .expect("tools must be array");
    let names: Vec<&str> = tools
        .iter()
        .filter_map(|t| t.get("name").and_then(Value::as_str))
        .collect();
    for expected in [
        "tileserver_list_sources",
        "tileserver_get_source_tilejson",
        "tileserver_list_styles",
        "tileserver_get_style",
        "tileserver_get_tile_metadata",
        "tileserver_get_server_info",
        "tileserver_render_static_map",
        "tileserver_get_tile",
        "tileserver_query_features_at_point",
    ] {
        assert!(
            names.contains(&expected),
            "missing tool {expected}; got {names:?}"
        );
    }
}

#[tokio::test]
async fn mcp_call_list_sources_returns_array_on_empty_state() {
    let server = empty_mcp_server();
    let session_id = initialize_and_get_session_id(&server).await;
    let resp = server
        .post("/mcp")
        .add_header("accept", MCP_ACCEPT)
        .add_header("mcp-session-id", &session_id)
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "tileserver_list_sources",
                "arguments": {}
            }
        }))
        .await;
    resp.assert_status_ok();
    let body = parse_sse_json(&resp.text());
    let content = &body["result"]["content"];
    assert!(content.is_array(), "content not array: {body}");
    assert_eq!(content[0]["type"], "text");
}

#[tokio::test]
async fn mcp_call_render_static_map_without_renderer_returns_is_error() {
    let server = empty_mcp_server();
    let session_id = initialize_and_get_session_id(&server).await;
    let resp = server
        .post("/mcp")
        .add_header("accept", MCP_ACCEPT)
        .add_header("mcp-session-id", &session_id)
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "tileserver_render_static_map",
                "arguments": {
                    "style_id": "ghost",
                    "lon": 0.0,
                    "lat": 0.0,
                    "zoom": 2.0,
                    "width": 256,
                    "height": 256
                }
            }
        }))
        .await;
    resp.assert_status_ok();
    let body = parse_sse_json(&resp.text());
    assert_eq!(
        body["result"]["isError"], true,
        "expected isError=true when renderer is absent: {body}"
    );
}

#[tokio::test]
async fn mcp_resource_read_unknown_style_returns_error() {
    let server = empty_mcp_server();
    let session_id = initialize_and_get_session_id(&server).await;
    let resp = server
        .post("/mcp")
        .add_header("accept", MCP_ACCEPT)
        .add_header("mcp-session-id", &session_id)
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "resources/read",
            "params": { "uri": "tileserver://styles/ghost" }
        }))
        .await;
    resp.assert_status_ok();
    let body = parse_sse_json(&resp.text());
    assert!(body.get("error").is_some(), "expected error: {body}");
}

#[tokio::test]
async fn mcp_resource_templates_listed() {
    let server = empty_mcp_server();
    let session_id = initialize_and_get_session_id(&server).await;
    let resp = server
        .post("/mcp")
        .add_header("accept", MCP_ACCEPT)
        .add_header("mcp-session-id", &session_id)
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "resources/templates/list",
            "params": {}
        }))
        .await;
    resp.assert_status_ok();
    let body = parse_sse_json(&resp.text());
    let templates = body["result"]["resourceTemplates"]
        .as_array()
        .expect("resourceTemplates array");
    let uris: Vec<&str> = templates
        .iter()
        .filter_map(|t| t["uriTemplate"].as_str())
        .collect();
    assert!(uris.contains(&"tileserver://styles/{id}"));
    assert!(uris.contains(&"tileserver://data/{id}.json"));
}

#[tokio::test]
async fn prompts_list_returns_four_prompts() {
    // Arrange
    let server = empty_mcp_server();
    let session_id = initialize_and_get_session_id(&server).await;

    // Act
    let resp = server
        .post("/mcp")
        .add_header("accept", MCP_ACCEPT)
        .add_header("mcp-session-id", &session_id)
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 10,
            "method": "prompts/list",
            "params": {}
        }))
        .await;

    // Assert
    resp.assert_status_ok();
    let body = parse_sse_json(&resp.text());
    let prompts = body["result"]["prompts"]
        .as_array()
        .unwrap_or_else(|| panic!("prompts must be array: {body}"));
    assert_eq!(prompts.len(), 4, "expected 4 prompts: {body}");
    let names: Vec<&str> = prompts.iter().filter_map(|p| p["name"].as_str()).collect();
    for expected in [
        "describe_style",
        "suggest_cql2_filter",
        "render_location_preview",
        "explain_tile_metadata",
    ] {
        assert!(
            names.contains(&expected),
            "missing prompt {expected}; got {names:?}"
        );
    }
}

#[tokio::test]
async fn prompts_list_includes_describe_style_with_required_arg() {
    // Arrange
    let server = empty_mcp_server();
    let session_id = initialize_and_get_session_id(&server).await;

    // Act
    let resp = server
        .post("/mcp")
        .add_header("accept", MCP_ACCEPT)
        .add_header("mcp-session-id", &session_id)
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 11,
            "method": "prompts/list",
            "params": {}
        }))
        .await;

    // Assert
    resp.assert_status_ok();
    let body = parse_sse_json(&resp.text());
    let prompts = body["result"]["prompts"]
        .as_array()
        .unwrap_or_else(|| panic!("prompts must be array: {body}"));
    let describe = prompts
        .iter()
        .find(|p| p["name"] == "describe_style")
        .unwrap_or_else(|| panic!("describe_style missing: {body}"));
    let args = describe["arguments"]
        .as_array()
        .unwrap_or_else(|| panic!("arguments must be array: {describe}"));
    let style_id_arg = args
        .iter()
        .find(|a| a["name"] == "style_id")
        .unwrap_or_else(|| panic!("style_id arg missing: {describe}"));
    assert_eq!(style_id_arg["required"], true, "style_id must be required");
}

#[tokio::test]
async fn prompts_get_describe_style_substitutes_style_id() {
    // Arrange
    let server = empty_mcp_server();
    let session_id = initialize_and_get_session_id(&server).await;

    // Act
    let resp = server
        .post("/mcp")
        .add_header("accept", MCP_ACCEPT)
        .add_header("mcp-session-id", &session_id)
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 12,
            "method": "prompts/get",
            "params": {
                "name": "describe_style",
                "arguments": { "style_id": "osm-bright" }
            }
        }))
        .await;

    // Assert
    resp.assert_status_ok();
    let body = parse_sse_json(&resp.text());
    let messages = body["result"]["messages"]
        .as_array()
        .unwrap_or_else(|| panic!("messages must be array: {body}"));
    assert_eq!(messages.len(), 1, "expected one message: {body}");
    let msg = &messages[0];
    assert_eq!(msg["role"], "user", "role must be user");
    assert_eq!(msg["content"]["type"], "text");
    let text = msg["content"]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("text missing: {msg}"));
    assert!(
        text.contains("osm-bright"),
        "style_id not substituted in `{text}`"
    );
}

#[tokio::test]
async fn prompts_get_suggest_cql2_substitutes_both_args() {
    // Arrange
    let server = empty_mcp_server();
    let session_id = initialize_and_get_session_id(&server).await;

    // Act
    let resp = server
        .post("/mcp")
        .add_header("accept", MCP_ACCEPT)
        .add_header("mcp-session-id", &session_id)
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 13,
            "method": "prompts/get",
            "params": {
                "name": "suggest_cql2_filter",
                "arguments": {
                    "table_id": "buildings",
                    "intent": "tall structures over 100m"
                }
            }
        }))
        .await;

    // Assert
    resp.assert_status_ok();
    let body = parse_sse_json(&resp.text());
    let text = body["result"]["messages"][0]["content"]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("text missing: {body}"));
    assert!(
        text.contains("buildings"),
        "table_id not substituted: {text}"
    );
    assert!(
        text.contains("tall structures over 100m"),
        "intent not substituted: {text}"
    );
}

#[tokio::test]
async fn prompts_get_render_location_preview_uses_zoom_default() {
    // Arrange
    let server = empty_mcp_server();
    let session_id = initialize_and_get_session_id(&server).await;

    // Act
    let resp = server
        .post("/mcp")
        .add_header("accept", MCP_ACCEPT)
        .add_header("mcp-session-id", &session_id)
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 14,
            "method": "prompts/get",
            "params": {
                "name": "render_location_preview",
                "arguments": { "location": "Tokyo" }
            }
        }))
        .await;

    // Assert
    resp.assert_status_ok();
    let body = parse_sse_json(&resp.text());
    let text = body["result"]["messages"][0]["content"]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("text missing: {body}"));
    assert!(text.contains("Tokyo"), "location missing: {text}");
    assert!(text.contains("12"), "default zoom 12 missing: {text}");
}

#[tokio::test]
async fn prompts_get_unknown_name_returns_error() {
    // Arrange
    let server = empty_mcp_server();
    let session_id = initialize_and_get_session_id(&server).await;

    // Act
    let resp = server
        .post("/mcp")
        .add_header("accept", MCP_ACCEPT)
        .add_header("mcp-session-id", &session_id)
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 15,
            "method": "prompts/get",
            "params": { "name": "ghost_prompt" }
        }))
        .await;

    // Assert
    resp.assert_status_ok();
    let body = parse_sse_json(&resp.text());
    assert!(
        body.get("error").is_some(),
        "expected JSON-RPC error for unknown prompt: {body}"
    );
}

#[tokio::test]
async fn prompts_get_missing_required_arg_returns_error() {
    // Arrange
    let server = empty_mcp_server();
    let session_id = initialize_and_get_session_id(&server).await;

    // Act
    let resp = server
        .post("/mcp")
        .add_header("accept", MCP_ACCEPT)
        .add_header("mcp-session-id", &session_id)
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 16,
            "method": "prompts/get",
            "params": {
                "name": "describe_style",
                "arguments": {}
            }
        }))
        .await;

    // Assert
    resp.assert_status_ok();
    let body = parse_sse_json(&resp.text());
    let err = body
        .get("error")
        .unwrap_or_else(|| panic!("expected error for missing required arg: {body}"));
    assert_eq!(err["code"], -32602, "expected INVALID_PARAMS: {err}");
}

#[tokio::test]
async fn mcp_initialize_advertises_prompts_capability() {
    // Arrange
    let server = empty_mcp_server();

    // Act
    let resp = server
        .post("/mcp")
        .add_header("accept", MCP_ACCEPT)
        .json(&initialize_payload())
        .await;

    // Assert
    resp.assert_status_ok();
    let body = parse_sse_json(&resp.text());
    assert!(
        body["result"]["capabilities"]["prompts"].is_object(),
        "prompts capability missing: {body}"
    );
}

#[tokio::test]
async fn mcp_bearer_auth_rejects_missing_token() {
    let shared = common::minimal_shared_state();
    let router = mcp_test_router(shared, Some("test-secret".into()));
    let server = TestServer::new(router);
    let resp = server
        .post("/mcp")
        .add_header("accept", MCP_ACCEPT)
        .json(&initialize_payload())
        .await;
    assert_eq!(resp.status_code(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn mcp_bearer_auth_accepts_valid_token() {
    let shared = common::minimal_shared_state();
    let router = mcp_test_router(shared, Some("test-secret".into()));
    let server = TestServer::new(router);
    let resp = server
        .post("/mcp")
        .add_header("accept", MCP_ACCEPT)
        .add_header(AUTHORIZATION, "Bearer test-secret")
        .json(&initialize_payload())
        .await;
    resp.assert_status_ok();
}
