//! Integration tests for the `[llm]` server proxy (issue #1547):
//! `GET /api/llm/config` and `POST /api/llm/chat` against a mock upstream.

mod common;

use std::sync::{Arc, Mutex};

use axum::{
    Router,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::post,
};
use serde_json::{Value, json};
use tileserver_rs::config::{Config, LlmConfig, LlmProviderKind};

/// What the mock upstream saw for its last request.
#[derive(Default, Clone)]
struct Seen {
    path: String,
    headers: Vec<(String, String)>,
    body: Value,
}

type Recorder = Arc<Mutex<Option<Seen>>>;

const SSE_BODY: &str = "data: {\"choices\":[{\"delta\":{\"content\":\"Hi\"}}]}\n\ndata: [DONE]\n\n";

async fn record(
    State(rec): State<Recorder>,
    uri: axum::http::Uri,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let seen = Seen {
        path: uri.path().to_string(),
        headers: headers
            .iter()
            .map(|(k, v)| {
                (
                    k.as_str().to_string(),
                    v.to_str().unwrap_or_default().to_string(),
                )
            })
            .collect(),
        body: serde_json::from_slice(&body).unwrap_or(Value::Null),
    };
    *rec.lock().expect("lock") = Some(seen);
    ([("content-type", "text/event-stream")], SSE_BODY)
}

/// Start a mock provider on a random loopback port. Returns its base URL.
async fn mock_upstream() -> (String, Recorder) {
    let rec: Recorder = Arc::default();
    let app = Router::new()
        .route("/v1/chat/completions", post(record))
        .route("/v1/messages", post(record))
        .with_state(rec.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(async move { axum::serve(listener, app).await.expect("serve") });
    (format!("http://127.0.0.1:{}", addr.port()), rec)
}

fn llm_config(llm: LlmConfig) -> Config {
    Config {
        llm,
        ..Config::default()
    }
}

fn header<'a>(seen: &'a Seen, name: &str) -> Option<&'a str> {
    seen.headers
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.as_str())
}

#[tokio::test]
async fn config_endpoint_reports_disabled_by_default() {
    let server = common::test_server_with_config(Config::default());
    let resp = server.get("/api/llm/config").await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert_eq!(body["proxy_enabled"], false);
    assert_eq!(body["allow_user_keys"], true);
    assert_eq!(body["model"], Value::Null);
    assert_eq!(body["upstream_host"], Value::Null);
}

#[tokio::test]
async fn chat_returns_404_when_proxy_disabled() {
    let server = common::test_server_with_config(Config::default());
    let resp = server
        .post("/api/llm/chat")
        .json(&json!({"messages": []}))
        .await;
    resp.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn openai_proxy_streams_and_sets_model_and_bearer_key() {
    let (base, rec) = mock_upstream().await;
    // SAFETY: test-only env var with a name unique to this test.
    unsafe { std::env::set_var("TS_TEST_LLM_KEY_OPENAI", "sk-test-openai") };
    let server = common::test_server_with_config(llm_config(LlmConfig {
        enabled: true,
        base_url: Some(format!("{base}/v1")),
        api_key_env: Some("TS_TEST_LLM_KEY_OPENAI".to_string()),
        model: Some("server-model".to_string()),
        rate_limit_per_minute: 0,
        ..LlmConfig::default()
    }));

    let config: Value = server.get("/api/llm/config").await.json();
    assert_eq!(config["proxy_enabled"], true);
    assert_eq!(config["provider"], "openai-compatible");
    assert_eq!(config["model"], "server-model");
    assert!(
        config["upstream_host"]
            .as_str()
            .unwrap_or_default()
            .starts_with("127.0.0.1:")
    );
    assert!(
        !config.to_string().contains("sk-test-openai"),
        "key must never reach the client"
    );

    let resp = server
        .post("/api/llm/chat")
        .json(&json!({"model": "client-model", "stream": true, "messages": [{"role": "user", "content": "hi"}]}))
        .await;
    resp.assert_status_ok();
    assert_eq!(resp.header("content-type"), "text/event-stream");
    assert_eq!(resp.header("cache-control"), "no-store");
    assert_eq!(resp.text(), SSE_BODY);

    let seen = rec
        .lock()
        .expect("lock")
        .clone()
        .expect("upstream was called");
    assert_eq!(seen.path, "/v1/chat/completions");
    assert_eq!(
        seen.body["model"], "server-model",
        "server model overrides client model"
    );
    assert_eq!(seen.body["messages"][0]["content"], "hi");
    assert_eq!(
        header(&seen, "authorization"),
        Some("Bearer sk-test-openai")
    );
}

#[tokio::test]
async fn anthropic_proxy_uses_messages_endpoint_and_x_api_key() {
    let (base, rec) = mock_upstream().await;
    // SAFETY: test-only env var with a name unique to this test.
    unsafe { std::env::set_var("TS_TEST_LLM_KEY_ANTHROPIC", "sk-ant-test") };
    let server = common::test_server_with_config(llm_config(LlmConfig {
        enabled: true,
        provider: LlmProviderKind::Anthropic,
        base_url: Some(base),
        api_key_env: Some("TS_TEST_LLM_KEY_ANTHROPIC".to_string()),
        model: Some("claude-test".to_string()),
        rate_limit_per_minute: 0,
        ..LlmConfig::default()
    }));

    let resp = server
        .post("/api/llm/chat")
        .json(&json!({"max_tokens": 64, "messages": [{"role": "user", "content": "hi"}]}))
        .await;
    resp.assert_status_ok();

    let seen = rec
        .lock()
        .expect("lock")
        .clone()
        .expect("upstream was called");
    assert_eq!(seen.path, "/v1/messages");
    assert_eq!(seen.body["model"], "claude-test");
    assert_eq!(header(&seen, "x-api-key"), Some("sk-ant-test"));
    assert_eq!(header(&seen, "anthropic-version"), Some("2023-06-01"));
    assert_eq!(header(&seen, "authorization"), None);
}

#[tokio::test]
async fn keyless_local_upstream_sends_no_auth_header() {
    let (base, rec) = mock_upstream().await;
    let server = common::test_server_with_config(llm_config(LlmConfig {
        enabled: true,
        base_url: Some(format!("{base}/v1")),
        model: Some("llama".to_string()),
        rate_limit_per_minute: 0,
        ..LlmConfig::default()
    }));
    server
        .post("/api/llm/chat")
        .json(&json!({"messages": []}))
        .await
        .assert_status_ok();
    let seen = rec
        .lock()
        .expect("lock")
        .clone()
        .expect("upstream was called");
    assert_eq!(header(&seen, "authorization"), None);
}

#[tokio::test]
async fn chat_rejects_oversized_and_non_object_bodies() {
    let (base, _rec) = mock_upstream().await;
    let server = common::test_server_with_config(llm_config(LlmConfig {
        enabled: true,
        base_url: Some(format!("{base}/v1")),
        model: Some("m".to_string()),
        max_body_bytes: 64,
        rate_limit_per_minute: 0,
        ..LlmConfig::default()
    }));
    let big = json!({"messages": [{"role": "user", "content": "x".repeat(200)}]});
    server
        .post("/api/llm/chat")
        .json(&big)
        .await
        .assert_status(StatusCode::PAYLOAD_TOO_LARGE);
    server
        .post("/api/llm/chat")
        .json(&json!([1, 2]))
        .await
        .assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn chat_returns_502_when_upstream_is_unreachable() {
    let server = common::test_server_with_config(llm_config(LlmConfig {
        enabled: true,
        // Port 9 (discard) on loopback: nothing listens, connection is refused.
        base_url: Some("http://127.0.0.1:9/v1".to_string()),
        model: Some("m".to_string()),
        rate_limit_per_minute: 0,
        ..LlmConfig::default()
    }));
    let resp = server
        .post("/api/llm/chat")
        .json(&json!({"messages": []}))
        .await;
    resp.assert_status(StatusCode::BAD_GATEWAY);
    let body: Value = resp.json();
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("upstream")
    );
}

#[tokio::test]
async fn chat_rate_limit_returns_429_with_retry_after() {
    let (base, _rec) = mock_upstream().await;
    let server = common::test_server_with_config(llm_config(LlmConfig {
        enabled: true,
        base_url: Some(format!("{base}/v1")),
        model: Some("m".to_string()),
        rate_limit_per_minute: 2,
        ..LlmConfig::default()
    }));
    let mut statuses = Vec::new();
    for _ in 0..3 {
        let resp = server
            .post("/api/llm/chat")
            .json(&json!({"messages": []}))
            .await;
        statuses.push(resp.status_code());
        if resp.status_code() == StatusCode::TOO_MANY_REQUESTS {
            assert_eq!(resp.header("retry-after"), "60");
        }
    }
    assert_eq!(statuses.last(), Some(&StatusCode::TOO_MANY_REQUESTS));
}
