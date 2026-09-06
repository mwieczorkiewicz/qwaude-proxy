//! HTTP routing, request forwarding, and streaming pass-through.
//!
//! `POST /v1/chat/completions` is the only route that does real work: it
//! reads the request body (bounded to `config.max_request_body_size`),
//! coerces mid-conversation `role: "system"` messages via `transform`,
//! forwards the rewritten body to `VLLM_BASE_URL` with the inbound
//! `Authorization` header passed through unchanged, and streams the
//! upstream's response back to the client without buffering it — the same
//! `axum::body::Body` wrapping the upstream `hyper` body is used whether the
//! upstream response is a single JSON object or a chunked SSE stream, so
//! bytes are relayed to the client as they arrive rather than being
//! collected first.
//!
//! ## Body-size enforcement note
//!
//! The plan named axum's `DefaultBodyLimit` tower layer for bounding request
//! size. That layer is what the `Bytes`/`Json` *extractors* consult
//! internally; since this handler reads the raw `Request` itself (so it can
//! run the byte-level, targeted-mutation `transform::coerce_system_messages`
//! rather than deserializing into an owned struct), it enforces the limit
//! directly with `http_body_util::Limited`, which is the same underlying
//! mechanism `DefaultBodyLimit` itself is built on. Doing it directly here —
//! instead of via the tower layer plus a custom rejection handler — lets the
//! 413 case map straight into this crate's own `ProxyError::BodyTooLarge`
//! and its `{"error": {...}}` JSON shape rather than axum's default
//! plain-text rejection body.

use crate::config::ProxyConfig;
use crate::error::ProxyError;
use crate::transform;
use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{header, HeaderName, Request as HttpRequest};
use axum::response::Response;
use axum::routing::{get, post};
use axum::Router;
use http_body_util::{BodyExt, Limited};
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client as LegacyClient;
use hyper_util::rt::TokioExecutor;
use std::sync::Arc;
use std::time::Duration;

/// The outbound client used to forward requests to `VLLM_BASE_URL`. Plain
/// HTTP only (no TLS connector) — matches the documented default upstream
/// (`http://127.0.0.1:8000`); this proxy is not designed to sit in front of
/// an HTTPS-only upstream.
type HttpClient = LegacyClient<HttpConnector, Body>;

fn build_http_client(connect_timeout: Duration) -> HttpClient {
    let mut connector = HttpConnector::new();
    connector.set_connect_timeout(Some(connect_timeout));
    LegacyClient::builder(TokioExecutor::new()).build(connector)
}

/// Shared state handed to every request handler.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<ProxyConfig>,
    client: HttpClient,
}

impl AppState {
    pub fn new(config: ProxyConfig) -> Self {
        let client = build_http_client(config.connect_timeout);
        Self {
            config: Arc::new(config),
            client,
        }
    }
}

/// Build the full axum `Router`, bound to `state`.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/v1/chat/completions", post(chat_completions_handler))
        .route("/health", get(health_handler))
        .with_state(state)
}

async fn health_handler() -> &'static str {
    "ok"
}

/// Headers that must not be copied verbatim from the upstream response onto
/// the response we send our own client: connection-management and framing
/// headers that axum recomputes itself for the body we're about to attach.
fn is_hop_by_hop_header(name: &HeaderName) -> bool {
    matches!(
        name.as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailers"
            | "transfer-encoding"
            | "upgrade"
            | "content-length"
    )
}

#[tracing::instrument(skip_all)]
async fn chat_completions_handler(
    State(state): State<AppState>,
    request: Request,
) -> Result<Response, ProxyError> {
    let authorization = request.headers().get(header::AUTHORIZATION).cloned();

    let limited_body = Limited::new(request.into_body(), state.config.max_request_body_size);
    let collected = limited_body
        .collect()
        .await
        .map_err(|_| ProxyError::BodyTooLarge)?;
    let body_bytes = collected.to_bytes();

    let (rewritten_body, report) =
        transform::coerce_system_messages(&body_bytes, &state.config.notice_prefix)?;

    if !report.is_empty() {
        tracing::debug!(
            coerced_count = report.count(),
            coerced_indices = ?report.coerced_indices,
            "coerced mid-conversation system message role(s)"
        );
    }

    // Local-debugging-only escape hatch: never enabled against real traffic
    // (VERBOSE_PAYLOAD_LOGGING defaults to false). Everything above this
    // point logs only counts/indices/kinds -- never message content.
    if state.config.verbose_payload_logging {
        tracing::debug!(
            rewritten_body = %String::from_utf8_lossy(&rewritten_body),
            "rewritten request payload (verbose payload logging enabled)"
        );
    }

    let upstream_uri = format!(
        "{}/v1/chat/completions",
        state.config.vllm_base_url.trim_end_matches('/')
    );

    let mut outbound_builder = HttpRequest::builder()
        .method("POST")
        .uri(&upstream_uri)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(auth) = authorization {
        outbound_builder = outbound_builder.header(header::AUTHORIZATION, auth);
    }
    let outbound_request = outbound_builder
        .body(Body::from(rewritten_body))
        .map_err(|_| ProxyError::Internal)?;

    // Bounded until the response *headers* arrive (hyper's `.request()`
    // resolves as soon as the head is received, before the body is read),
    // not for the duration of the body — a streaming response is therefore
    // only bounded until its first byte, per NFR4.1.
    let upstream_response = tokio::time::timeout(
        state.config.total_timeout,
        state.client.request(outbound_request),
    )
    .await
    .map_err(|_| ProxyError::UpstreamTimeout)?
    .map_err(|err| {
        tracing::error!(error = %err, "upstream connection failed");
        ProxyError::UpstreamConnectFailed
    })?;

    let (parts, upstream_body) = upstream_response.into_parts();
    let mut response_builder = Response::builder().status(parts.status);
    for (name, value) in parts.headers.iter() {
        if is_hop_by_hop_header(name) {
            continue;
        }
        response_builder = response_builder.header(name.clone(), value.clone());
    }

    response_builder
        .body(Body::new(upstream_body))
        .map_err(|_| ProxyError::Internal)
}
