//! Server-side LLM proxy for the Map AI chat panel (issue #1547).
//!
//! - `GET /api/llm/config` tells the client whether the proxy is enabled,
//!   which provider family and model it uses, and the upstream host (for the
//!   "prompts are sent to <host>" notice). It never returns the API key.
//! - `POST /api/llm/chat` forwards an OpenAI Chat Completions or Anthropic
//!   Messages request body to the configured upstream and streams the
//!   response back unchanged. The server sets the model and the auth headers;
//!   the API key is read from the environment on every request.

use std::sync::{LazyLock, Mutex, PoisonError};
use std::time::{Duration, Instant};

use axum::{
    Json,
    body::{Body, Bytes},
    extract::State,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::Serialize;

use crate::config::{LlmProviderKind, is_allowed_llm_base_url};
use crate::reload::SharedState;

/// `anthropic-version` header sent to the Anthropic Messages API.
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Hard transport cap for `/api/llm/chat` bodies. The configurable
/// `[llm].max_body_bytes` check runs inside the handler, below this cap.
pub(crate) const MAX_TRANSPORT_BODY_BYTES: usize = 16 * 1024 * 1024;

/// Length of the fixed rate-limit window.
const RATE_WINDOW: Duration = Duration::from_secs(60);

static HTTP: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);

/// Fixed-window counter shared by all proxy requests: `(window start, count)`.
static RATE: Mutex<Option<(Instant, u32)>> = Mutex::new(None);

/// Public view of `[llm]` for the client. Contains no secrets.
#[derive(Debug, Serialize)]
pub(crate) struct LlmClientConfig {
    proxy_enabled: bool,
    provider: &'static str,
    model: Option<String>,
    upstream_host: Option<String>,
    allow_user_keys: bool,
}

/// Errors returned by the proxy routes.
#[derive(Debug, thiserror::Error)]
pub(crate) enum LlmProxyError {
    #[error("the LLM proxy is not enabled on this server")]
    Disabled,
    #[error("request body is larger than {0} bytes")]
    TooLarge(usize),
    #[error("rate limit reached; try again in a minute")]
    RateLimited,
    #[error("request body must be a JSON object")]
    BadBody,
    #[error("the LLM proxy is misconfigured: {0}")]
    Misconfigured(String),
    #[error("upstream LLM request failed: {0}")]
    Upstream(String),
}

impl IntoResponse for LlmProxyError {
    fn into_response(self) -> Response {
        let status = match self {
            Self::Disabled => StatusCode::NOT_FOUND,
            Self::TooLarge(_) => StatusCode::PAYLOAD_TOO_LARGE,
            Self::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            Self::BadBody => StatusCode::BAD_REQUEST,
            Self::Misconfigured(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Upstream(_) => StatusCode::BAD_GATEWAY,
        };
        let mut response = (
            status,
            Json(serde_json::json!({ "error": { "message": self.to_string() } })),
        )
            .into_response();
        if matches!(self, Self::RateLimited) {
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, HeaderValue::from_static("60"));
        }
        response
    }
}

/// `GET /api/llm/config`
pub(crate) async fn get_llm_config(State(state): State<SharedState>) -> Json<LlmClientConfig> {
    let config = state.config();
    let llm = &config.llm;
    Json(LlmClientConfig {
        proxy_enabled: llm.enabled,
        provider: llm.provider.as_str(),
        model: llm.enabled.then(|| llm.model.clone()).flatten(),
        upstream_host: llm
            .enabled
            .then(|| llm.base_url.as_deref().and_then(host_of))
            .flatten(),
        allow_user_keys: llm.allow_user_keys,
    })
}

/// `POST /api/llm/chat`
pub(crate) async fn post_llm_chat(
    State(state): State<SharedState>,
    body: Bytes,
) -> Result<Response, LlmProxyError> {
    let config = state.config();
    let llm = &config.llm;
    if !llm.enabled {
        return Err(LlmProxyError::Disabled);
    }
    if body.len() > llm.max_body_bytes {
        return Err(LlmProxyError::TooLarge(llm.max_body_bytes));
    }
    check_rate_limit(llm.rate_limit_per_minute)?;

    let mut payload: serde_json::Value =
        serde_json::from_slice(&body).map_err(|_| LlmProxyError::BadBody)?;
    let fields = payload.as_object_mut().ok_or(LlmProxyError::BadBody)?;
    let model = llm
        .model
        .as_deref()
        .ok_or_else(|| LlmProxyError::Misconfigured("llm.model is not set".to_string()))?;
    fields.insert("model".to_string(), serde_json::Value::from(model));

    let base_url = llm
        .base_url
        .as_deref()
        .filter(|url| is_allowed_llm_base_url(url))
        .ok_or_else(|| LlmProxyError::Misconfigured("llm.base_url is not valid".to_string()))?;
    let api_key = match llm.api_key_env.as_deref() {
        Some(var) => Some(std::env::var(var).map_err(|_| {
            LlmProxyError::Misconfigured(format!("environment variable '{var}' is not set"))
        })?),
        None => None,
    };

    let mut request = HTTP
        .post(upstream_url(llm.provider, base_url))
        .timeout(Duration::from_secs(llm.timeout_secs))
        .json(&payload);
    match llm.provider {
        LlmProviderKind::OpenaiCompatible => {
            if let Some(key) = &api_key {
                request = request.bearer_auth(key);
            }
        }
        LlmProviderKind::Anthropic => {
            request = request.header("anthropic-version", ANTHROPIC_VERSION);
            if let Some(key) = &api_key {
                request = request.header("x-api-key", key);
            }
        }
    }

    let upstream = request
        .send()
        .await
        .map_err(|e| LlmProxyError::Upstream(e.without_url().to_string()))?;

    let status =
        StatusCode::from_u16(upstream.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let content_type = upstream
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| HeaderValue::from_bytes(v.as_bytes()).ok())
        .unwrap_or_else(|| HeaderValue::from_static("application/json"));

    let mut response = Response::new(Body::from_stream(upstream.bytes_stream()));
    *response.status_mut() = status;
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, content_type);
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert("x-accel-buffering", HeaderValue::from_static("no"));
    Ok(response)
}

/// Count one request against the shared per-minute budget. `limit == 0`
/// disables the check.
fn check_rate_limit(limit: u32) -> Result<(), LlmProxyError> {
    if limit == 0 {
        return Ok(());
    }
    let now = Instant::now();
    let mut window = RATE.lock().unwrap_or_else(PoisonError::into_inner);
    let (start, count) = match *window {
        Some((start, count)) if now.duration_since(start) < RATE_WINDOW => (start, count),
        _ => (now, 0),
    };
    if count >= limit {
        *window = Some((start, count));
        return Err(LlmProxyError::RateLimited);
    }
    *window = Some((start, count + 1));
    Ok(())
}

/// Upstream endpoint for `provider` under `base_url`.
fn upstream_url(provider: LlmProviderKind, base_url: &str) -> String {
    let base = base_url.trim().trim_end_matches('/');
    match provider {
        LlmProviderKind::OpenaiCompatible => format!("{base}/chat/completions"),
        LlmProviderKind::Anthropic if base.ends_with("/v1") => format!("{base}/messages"),
        LlmProviderKind::Anthropic => format!("{base}/v1/messages"),
    }
}

/// Host (with port) of an `http(s)://` URL, without credentials or path.
fn host_of(url: &str) -> Option<String> {
    let rest = url.trim().split_once("://")?.1;
    let authority = rest.split(['/', '?', '#']).next()?;
    let host = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    (!host.is_empty()).then(|| host.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upstream_url_per_provider() {
        let openai = LlmProviderKind::OpenaiCompatible;
        let anthropic = LlmProviderKind::Anthropic;
        assert_eq!(
            upstream_url(openai, "https://openrouter.ai/api/v1/"),
            "https://openrouter.ai/api/v1/chat/completions"
        );
        assert_eq!(
            upstream_url(anthropic, "https://api.anthropic.com"),
            "https://api.anthropic.com/v1/messages"
        );
        assert_eq!(
            upstream_url(anthropic, "https://gateway.example/anthropic/v1"),
            "https://gateway.example/anthropic/v1/messages"
        );
    }

    #[test]
    fn host_of_strips_scheme_credentials_and_path() {
        assert_eq!(
            host_of("https://api.anthropic.com/v1").as_deref(),
            Some("api.anthropic.com")
        );
        assert_eq!(
            host_of("http://user:pw@localhost:11434/v1?x=1").as_deref(),
            Some("localhost:11434")
        );
        assert_eq!(host_of("not a url"), None);
        assert_eq!(host_of("https://"), None);
    }

    #[test]
    fn error_status_codes() {
        let cases = [
            (LlmProxyError::Disabled, StatusCode::NOT_FOUND),
            (LlmProxyError::TooLarge(1), StatusCode::PAYLOAD_TOO_LARGE),
            (LlmProxyError::RateLimited, StatusCode::TOO_MANY_REQUESTS),
            (LlmProxyError::BadBody, StatusCode::BAD_REQUEST),
            (
                LlmProxyError::Misconfigured("x".into()),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
            (LlmProxyError::Upstream("x".into()), StatusCode::BAD_GATEWAY),
        ];
        for (error, expected) in cases {
            let response = error.into_response();
            assert_eq!(response.status(), expected);
        }
        let limited = LlmProxyError::RateLimited.into_response();
        assert_eq!(limited.headers()[header::RETRY_AFTER], "60");
    }
}
