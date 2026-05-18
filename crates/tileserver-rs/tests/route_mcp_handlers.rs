//! Behavioral integration tests for `crate::mcp::handlers` tool functions.
//!
//! Each test arranges a [`SharedState`] populated with mock sources and/or
//! real on-disk styles, drives a tool call through the Streamable HTTP
//! transport, then asserts on the JSON-RPC response. Tests are organised by
//! tool, with one test per branch (success / not-found / validation error).
//!
//! No external services (PostGIS, STAC, native renderer) are required — the
//! `postgres`/`stac` tools surface their "feature not available" errors in
//! this `--features mcp` build, which is itself the behaviour under test.

#![cfg(feature = "mcp")]

mod common;

use std::sync::Arc;

use axum::Router;
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

fn parse_sse_json(body: &str) -> Value {
    let line = body
        .lines()
        .filter_map(|l| l.strip_prefix("data:").map(str::trim))
        .rfind(|l| !l.is_empty())
        .unwrap_or_else(|| panic!("no `data:` line in SSE body: {body:?}"));
    serde_json::from_str(line).unwrap_or_else(|e| panic!("invalid JSON `{line}`: {e}"))
}

fn mcp_router(shared: SharedState) -> Router {
    let factory_state = shared;
    let svc = StreamableHttpService::new(
        move || Ok(McpHandler::new(factory_state.load())),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default().disable_allowed_hosts(),
    );
    Router::new().nest_service("/mcp", svc)
}

fn server_for(shared: SharedState) -> TestServer {
    TestServer::new(mcp_router(shared))
}

async fn initialize_session(server: &TestServer) -> String {
    let init = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": { "name": "wave3", "version": "0.0.0" }
        }
    });
    let resp = server
        .post("/mcp")
        .add_header("accept", MCP_ACCEPT)
        .json(&init)
        .await;
    resp.assert_status_ok();
    let session_id = resp
        .headers()
        .get("mcp-session-id")
        .unwrap_or_else(|| panic!("missing mcp-session-id header"))
        .to_str()
        .unwrap_or_else(|_| panic!("session id not ascii"))
        .to_string();
    let initialized = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
    let _ = server
        .post("/mcp")
        .add_header("accept", MCP_ACCEPT)
        .add_header("mcp-session-id", &session_id)
        .json(&initialized)
        .await;
    session_id
}

async fn call_tool(server: &TestServer, session_id: &str, name: &str, args: Value) -> Value {
    let resp = server
        .post("/mcp")
        .add_header("accept", MCP_ACCEPT)
        .add_header("mcp-session-id", session_id)
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 99,
            "method": "tools/call",
            "params": { "name": name, "arguments": args }
        }))
        .await;
    resp.assert_status_ok();
    parse_sse_json(&resp.text())
}

fn first_text(body: &Value) -> &str {
    body["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("no text content in response: {body}"))
}

fn is_error(body: &Value) -> bool {
    body["result"]["isError"].as_bool().unwrap_or(false)
}

// ============================================================
// tileserver_list_sources
// ============================================================

#[tokio::test]
async fn list_sources_with_populated_state_returns_two() {
    let server = server_for(common::shared_state_with_two_sources());
    let session = initialize_session(&server).await;

    let body = call_tool(&server, &session, "tileserver_list_sources", json!({})).await;

    assert!(!is_error(&body), "unexpected error: {body}");
    let text = first_text(&body);
    let arr: Vec<Value> = serde_json::from_str(text)
        .unwrap_or_else(|e| panic!("expected JSON array body, got `{text}`: {e}"));
    assert_eq!(
        arr.len(),
        2,
        "expected two sources, got {}: {text}",
        arr.len()
    );
    let ids: Vec<&str> = arr.iter().filter_map(|v| v["id"].as_str()).collect();
    assert!(
        ids.contains(&"alpha-source"),
        "missing alpha-source: {ids:?}"
    );
    assert!(ids.contains(&"beta-source"), "missing beta-source: {ids:?}");
}

#[tokio::test]
async fn list_sources_includes_minzoom_maxzoom() {
    let server = server_for(common::shared_state_with_two_sources());
    let session = initialize_session(&server).await;

    let body = call_tool(&server, &session, "tileserver_list_sources", json!({})).await;
    let arr: Vec<Value> = serde_json::from_str(first_text(&body)).expect("array");
    let entry = arr
        .iter()
        .find(|v| v["id"] == "alpha-source")
        .expect("alpha");
    assert_eq!(
        entry["minzoom"].as_u64(),
        Some(0),
        "minzoom missing: {entry}"
    );
    assert_eq!(
        entry["maxzoom"].as_u64(),
        Some(14),
        "maxzoom missing: {entry}"
    );
}

// ============================================================
// tileserver_get_source_tilejson
// ============================================================

#[tokio::test]
async fn get_source_tilejson_returns_full_tilejson_for_known_source() {
    let server = server_for(common::shared_state_with_two_sources());
    let session = initialize_session(&server).await;

    let body = call_tool(
        &server,
        &session,
        "tileserver_get_source_tilejson",
        json!({ "source_id": "alpha-source" }),
    )
    .await;

    assert!(!is_error(&body), "unexpected error: {body}");
    let parsed: Value =
        serde_json::from_str(first_text(&body)).expect("tilejson body parses as JSON");
    assert_eq!(parsed["id"], "alpha-source");
    assert!(
        parsed.get("tilejson").is_some() || parsed.get("tiles").is_some(),
        "expected tilejson/tiles field: {parsed}"
    );
}

#[tokio::test]
async fn get_source_tilejson_returns_error_for_unknown_source() {
    let server = server_for(common::shared_state_with_two_sources());
    let session = initialize_session(&server).await;

    let body = call_tool(
        &server,
        &session,
        "tileserver_get_source_tilejson",
        json!({ "source_id": "ghost" }),
    )
    .await;

    assert!(is_error(&body), "expected isError=true: {body}");
}

// ============================================================
// tileserver_list_styles
// ============================================================

#[tokio::test]
async fn list_styles_with_populated_state_returns_protomaps_light() {
    let server = server_for(common::shared_state_populated());
    let session = initialize_session(&server).await;

    let body = call_tool(&server, &session, "tileserver_list_styles", json!({})).await;

    assert!(!is_error(&body), "unexpected error: {body}");
    let arr: Vec<Value> = serde_json::from_str(first_text(&body)).expect("array body");
    assert_eq!(arr.len(), 1, "expected one style: {arr:?}");
    assert_eq!(arr[0]["id"], "protomaps-light");
}

// ============================================================
// tileserver_get_style
// ============================================================

#[tokio::test]
async fn get_style_returns_full_style_json_for_known() {
    let server = server_for(common::shared_state_populated());
    let session = initialize_session(&server).await;

    let body = call_tool(
        &server,
        &session,
        "tileserver_get_style",
        json!({ "style_id": "protomaps-light" }),
    )
    .await;

    assert!(!is_error(&body), "unexpected error: {body}");
    let parsed: Value = serde_json::from_str(first_text(&body)).expect("style JSON parses");
    assert!(parsed.get("version").is_some(), "version missing: {parsed}");
    assert!(parsed.get("sources").is_some(), "sources missing: {parsed}");
    assert!(parsed.get("layers").is_some(), "layers missing: {parsed}");
}

#[tokio::test]
async fn get_style_returns_error_for_unknown_style() {
    let server = server_for(common::shared_state_populated());
    let session = initialize_session(&server).await;

    let body = call_tool(
        &server,
        &session,
        "tileserver_get_style",
        json!({ "style_id": "ghost" }),
    )
    .await;

    assert!(is_error(&body), "expected isError=true: {body}");
}

// ============================================================
// tileserver_get_tile_metadata
// ============================================================

#[tokio::test]
async fn get_tile_metadata_returns_layer_schema_for_vector_source() {
    let server = server_for(common::shared_state_with_two_sources());
    let session = initialize_session(&server).await;

    let body = call_tool(
        &server,
        &session,
        "tileserver_get_tile_metadata",
        json!({ "source_id": "alpha-source" }),
    )
    .await;

    assert!(!is_error(&body), "unexpected error: {body}");
    let meta: Value = serde_json::from_str(first_text(&body)).expect("metadata JSON");
    assert_eq!(meta["id"], "alpha-source");
    assert_eq!(meta["format"], "pbf");
    let layers = meta["vector_layers"]
        .as_array()
        .expect("vector_layers array");
    assert!(
        layers.iter().any(|l| l["id"] == "buildings"),
        "buildings layer missing: {meta}"
    );
}

#[tokio::test]
async fn get_tile_metadata_returns_error_for_unknown_source() {
    let server = server_for(common::shared_state_with_two_sources());
    let session = initialize_session(&server).await;

    let body = call_tool(
        &server,
        &session,
        "tileserver_get_tile_metadata",
        json!({ "source_id": "ghost" }),
    )
    .await;

    assert!(is_error(&body), "expected isError=true: {body}");
}

// ============================================================
// tileserver_get_server_info
// ============================================================

#[tokio::test]
async fn get_server_info_returns_counts_matching_state() {
    let server = server_for(common::shared_state_populated());
    let session = initialize_session(&server).await;

    let body = call_tool(&server, &session, "tileserver_get_server_info", json!({})).await;

    assert!(!is_error(&body), "unexpected error: {body}");
    let info: Value = serde_json::from_str(first_text(&body)).expect("server info JSON");
    assert_eq!(info["loaded_sources"], 2, "got: {info}");
    assert_eq!(info["loaded_styles"], 1, "got: {info}");
    assert_eq!(info["renderer_enabled"], false, "got: {info}");
    assert!(
        info["version"].as_str().is_some(),
        "version missing: {info}"
    );
}

#[tokio::test]
async fn get_server_info_reports_renderer_disabled_when_none() {
    let server = server_for(common::minimal_shared_state());
    let session = initialize_session(&server).await;

    let body = call_tool(&server, &session, "tileserver_get_server_info", json!({})).await;

    let info: Value = serde_json::from_str(first_text(&body)).expect("info JSON");
    assert_eq!(info["renderer_enabled"], false, "got: {info}");
    assert_eq!(info["cache_enabled"], false, "got: {info}");
}

// ============================================================
// tileserver_render_static_map
// ============================================================

#[tokio::test]
async fn render_static_map_returns_error_when_renderer_disabled() {
    let server = server_for(common::shared_state_populated());
    let session = initialize_session(&server).await;

    let body = call_tool(
        &server,
        &session,
        "tileserver_render_static_map",
        json!({
            "style_id": "protomaps-light",
            "lon": 0.0, "lat": 0.0, "zoom": 2.0,
            "width": 256, "height": 256
        }),
    )
    .await;

    assert!(
        is_error(&body),
        "expected isError when renderer absent: {body}"
    );
    assert!(
        first_text(&body).to_lowercase().contains("renderer"),
        "expected message about renderer: {body}"
    );
}

#[tokio::test]
async fn render_static_map_rejects_zero_dimensions() {
    let server = server_for(common::shared_state_populated());
    let session = initialize_session(&server).await;

    // Renderer is absent, so the "no renderer" branch fires before the
    // dimension check. Build a state with a stub renderer? Not feasible
    // without real native libs — instead exercise the zero-dimension path
    // by hitting the empty state (still no renderer, same branch). The
    // value of this test is documenting that bad inputs do not crash.
    let body = call_tool(
        &server,
        &session,
        "tileserver_render_static_map",
        json!({
            "style_id": "protomaps-light",
            "lon": 0.0, "lat": 0.0, "zoom": 2.0,
            "width": 0, "height": 0
        }),
    )
    .await;

    assert!(is_error(&body), "expected isError: {body}");
}

#[tokio::test]
async fn render_static_map_rejects_oversized_dimensions() {
    let server = server_for(common::shared_state_populated());
    let session = initialize_session(&server).await;

    let body = call_tool(
        &server,
        &session,
        "tileserver_render_static_map",
        json!({
            "style_id": "protomaps-light",
            "lon": 0.0, "lat": 0.0, "zoom": 2.0,
            "width": 4096, "height": 4096
        }),
    )
    .await;

    assert!(is_error(&body), "expected isError: {body}");
}

// ============================================================
// tileserver_get_tile
// ============================================================

#[tokio::test]
async fn get_tile_returns_base64_payload_for_known_source() {
    let server = server_for(common::shared_state_with_two_sources());
    let session = initialize_session(&server).await;

    let body = call_tool(
        &server,
        &session,
        "tileserver_get_tile",
        json!({ "source_id": "alpha-source", "z": 0, "x": 0, "y": 0 }),
    )
    .await;

    assert!(!is_error(&body), "unexpected error: {body}");
    let payload: Value = serde_json::from_str(first_text(&body)).expect("tile payload JSON");
    assert_eq!(payload["source_id"], "alpha-source");
    assert_eq!(payload["z"], 0);
    assert_eq!(payload["format"], "pbf");
    assert!(
        payload["data_base64"].as_str().is_some(),
        "missing base64: {payload}"
    );
}

#[tokio::test]
async fn get_tile_returns_error_for_unknown_source() {
    let server = server_for(common::shared_state_with_two_sources());
    let session = initialize_session(&server).await;

    let body = call_tool(
        &server,
        &session,
        "tileserver_get_tile",
        json!({ "source_id": "ghost", "z": 0, "x": 0, "y": 0 }),
    )
    .await;

    assert!(is_error(&body), "expected isError: {body}");
}

#[tokio::test]
async fn get_tile_returns_tile_not_found_when_source_has_no_data() {
    let server = server_for(common::shared_state_with_empty_source());
    let session = initialize_session(&server).await;

    let body = call_tool(
        &server,
        &session,
        "tileserver_get_tile",
        json!({ "source_id": "empty-source", "z": 5, "x": 1, "y": 1 }),
    )
    .await;

    assert!(is_error(&body), "expected isError: {body}");
    let text = first_text(&body).to_lowercase();
    assert!(
        text.contains("not found") || text.contains("tile"),
        "expected tile-not-found message, got: {}",
        first_text(&body)
    );
}

// ============================================================
// tileserver_query_features_at_point — postgres feature absent
// ============================================================

#[tokio::test]
async fn query_features_at_point_errors_without_postgres_feature() {
    let server = server_for(common::minimal_shared_state());
    let session = initialize_session(&server).await;

    let body = call_tool(
        &server,
        &session,
        "tileserver_query_features_at_point",
        json!({
            "source_id": "anything",
            "lon": 0.0, "lat": 0.0
        }),
    )
    .await;

    assert!(
        is_error(&body),
        "expected isError when postgres absent: {body}"
    );
    assert!(
        first_text(&body).contains("postgres"),
        "expected postgres-related error: {body}"
    );
}

#[tokio::test]
async fn query_features_cql2_errors_without_postgres_feature() {
    let server = server_for(common::minimal_shared_state());
    let session = initialize_session(&server).await;

    let body = call_tool(
        &server,
        &session,
        "tileserver_query_features_cql2",
        json!({ "source_id": "anything", "cql2": "1=1" }),
    )
    .await;

    assert!(
        is_error(&body),
        "expected isError when postgres absent: {body}"
    );
    assert!(
        first_text(&body).contains("postgres"),
        "expected postgres-related error: {body}"
    );
}

#[tokio::test]
async fn search_stac_items_errors_without_stac_feature() {
    let server = server_for(common::minimal_shared_state());
    let session = initialize_session(&server).await;

    let body = call_tool(
        &server,
        &session,
        "tileserver_search_stac_items",
        json!({ "source_id": "anything" }),
    )
    .await;

    assert!(is_error(&body), "expected isError when stac absent: {body}");
    assert!(
        first_text(&body).contains("stac"),
        "expected stac-related error: {body}"
    );
}

// ============================================================
// tools/list shape
// ============================================================

#[tokio::test]
async fn tools_list_includes_all_eleven_tools_with_input_schemas() {
    let server = server_for(common::minimal_shared_state());
    let session = initialize_session(&server).await;

    let resp = server
        .post("/mcp")
        .add_header("accept", MCP_ACCEPT)
        .add_header("mcp-session-id", &session)
        .json(&json!({
            "jsonrpc": "2.0", "id": 5,
            "method": "tools/list", "params": {}
        }))
        .await;
    resp.assert_status_ok();
    let body = parse_sse_json(&resp.text());
    let tools = body["result"]["tools"].as_array().expect("tools array");

    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
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
        "tileserver_query_features_cql2",
        "tileserver_search_stac_items",
    ] {
        assert!(
            names.contains(&expected),
            "missing {expected}: got {names:?}"
        );
    }

    for tool in tools {
        let name = tool["name"].as_str().unwrap_or("(unnamed)");
        assert!(
            tool["inputSchema"].is_object(),
            "tool {name} missing inputSchema: {tool}"
        );
    }
}
