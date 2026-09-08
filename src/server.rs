//! HTTP routing, request forwarding, and streaming pass-through.
//!
//! `POST /v1/chat/completions` and `POST /v1/messages` coerce mid-conversation
//! system roles before forwarding. Other OpenAI-compatible routes (model
//! listing, legacy completions, embeddings) are forwarded transparently with
//! no body rewrite.
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
use axum::http::{header, HeaderMap, HeaderName, Method, Request as HttpRequest};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use http_body_util::{BodyExt, Limited};
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client as LegacyClient;
use hyper_util::rt::TokioExecutor;
use metrics_exporter_prometheus::PrometheusHandle;
use std::sync::Arc;
use std::time::{Duration, Instant};

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
    metrics_handle: PrometheusHandle,
}

impl AppState {
    pub fn new(config: ProxyConfig) -> Self {
        let client = build_http_client(config.connect_timeout);
        Self {
            config: Arc::new(config),
            client,
            metrics_handle: crate::metrics::shared_handle(),
        }
    }
}

/// Build the full axum `Router`, bound to `state`.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/v1/chat/completions", post(chat_completions_handler))
        .route("/v1/messages", post(messages_handler))
        .route(
            "/v1/models",
            get(transparent_handler).head(transparent_handler),
        )
        .route(
            "/v1/models/{model_id}",
            get(transparent_handler).head(transparent_handler),
        )
        .route("/v1/completions", post(transparent_handler))
        .route("/v1/embeddings", post(transparent_handler))
        .route("/health", get(health_handler))
        .route("/metrics", get(metrics_handler))
        .with_state(state)
}

async fn health_handler() -> &'static str {
    "ok"
}

async fn metrics_handler(State(state): State<AppState>) -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        state.metrics_handle.render(),
    )
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

async fn chat_completions_handler(State(state): State<AppState>, request: Request) -> Response {
    proxy_handler(&state, request, "/v1/chat/completions").await
}

async fn messages_handler(State(state): State<AppState>, request: Request) -> Response {
    proxy_handler(&state, request, "/v1/messages").await
}

async fn transparent_handler(State(state): State<AppState>, request: Request) -> Response {
    transparent_proxy_handler(&state, request).await
}

/// Transparent pass-through: same method, path, query, headers, and body.
async fn transparent_proxy_handler(state: &AppState, request: Request) -> Response {
    let start = Instant::now();
    let result = handle_transparent_forward(state, request).await;
    finish_handler_response(start, result)
}

/// The actual axum route handler: records `proxy_requests_total` (labeled
/// by final status) and `proxy_request_duration_seconds` around
/// [`handle_coerced_forward`] regardless of outcome, then converts its
/// `Result` into a `Response` (via `ProxyError`'s own `IntoResponse` impl on
/// the error path).
#[tracing::instrument(skip_all, fields(upstream_path = upstream_path))]
async fn proxy_handler(state: &AppState, request: Request, upstream_path: &str) -> Response {
    let start = Instant::now();
    let result = handle_coerced_forward(state, request, upstream_path).await;
    finish_handler_response(start, result)
}

fn finish_handler_response(start: Instant, result: Result<Response, ProxyError>) -> Response {
    let elapsed = start.elapsed();

    let status = match &result {
        Ok(response) => response.status(),
        Err(err) => err.status(),
    };
    metrics::counter!(
        crate::metrics::REQUESTS_TOTAL,
        "status" => status.as_u16().to_string(),
    )
    .increment(1);
    metrics::histogram!(crate::metrics::REQUEST_DURATION_SECONDS).record(elapsed.as_secs_f64());

    if let Err(err) = &result {
        if let Some(kind) = err.upstream_error_kind() {
            metrics::counter!(crate::metrics::UPSTREAM_ERRORS_TOTAL, "kind" => kind).increment(1);
        }
    }

    match result {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

fn is_request_hop_by_hop_header(name: &HeaderName) -> bool {
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
            | "host"
    )
}

fn apply_inbound_headers(
    inbound: &HeaderMap,
    mut builder: hyper::http::request::Builder,
) -> hyper::http::request::Builder {
    for (name, value) in inbound.iter() {
        if is_request_hop_by_hop_header(name) {
            continue;
        }
        builder = builder.header(name, value);
    }
    builder
}

async fn send_upstream_request(
    state: &AppState,
    outbound_request: HttpRequest<Body>,
) -> Result<Response, ProxyError> {
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

    if !upstream_response.status().is_success() {
        metrics::counter!(crate::metrics::UPSTREAM_ERRORS_TOTAL, "kind" => "non_2xx").increment(1);
    }

    upstream_response_to_response(upstream_response)
}

fn upstream_response_to_response(
    upstream_response: hyper::Response<hyper::body::Incoming>,
) -> Result<Response, ProxyError> {
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

async fn handle_transparent_forward(
    state: &AppState,
    request: Request,
) -> Result<Response, ProxyError> {
    let (parts, body) = request.into_parts();
    let path_and_query = parts
        .uri
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");
    let upstream_uri = format!(
        "{}{}",
        state.config.vllm_base_url.trim_end_matches('/'),
        path_and_query
    );

    let outbound_body = if parts.method == Method::GET || parts.method == Method::HEAD {
        Body::empty()
    } else {
        let limited_body = Limited::new(body, state.config.max_request_body_size);
        let collected = limited_body
            .collect()
            .await
            .map_err(|_| ProxyError::BodyTooLarge)?;
        Body::from(collected.to_bytes())
    };

    let outbound_builder = apply_inbound_headers(
        &parts.headers,
        HttpRequest::builder()
            .method(parts.method)
            .uri(&upstream_uri),
    );
    let outbound_request = outbound_builder
        .body(outbound_body)
        .map_err(|_| ProxyError::Internal)?;

    send_upstream_request(state, outbound_request).await
}

async fn handle_coerced_forward(
    state: &AppState,
    request: Request,
    upstream_path: &str,
) -> Result<Response, ProxyError> {
    let (parts, body) = request.into_parts();
    let authorization = parts.headers.get(header::AUTHORIZATION).cloned();
    let x_api_key = parts
        .headers
        .get(HeaderName::from_static("x-api-key"))
        .cloned();
    let anthropic_version = parts
        .headers
        .get(HeaderName::from_static("anthropic-version"))
        .cloned();
    let anthropic_beta = parts
        .headers
        .get(HeaderName::from_static("anthropic-beta"))
        .cloned();

    let limited_body = Limited::new(body, state.config.max_request_body_size);
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
        metrics::counter!(crate::metrics::MESSAGES_COERCED_TOTAL).increment(report.count() as u64);
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
        "{}{}",
        state.config.vllm_base_url.trim_end_matches('/'),
        upstream_path
    );

    let mut outbound_builder = HttpRequest::builder()
        .method("POST")
        .uri(&upstream_uri)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(auth) = authorization {
        outbound_builder = outbound_builder.header(header::AUTHORIZATION, auth);
    }
    if let Some(key) = x_api_key {
        outbound_builder = outbound_builder.header("x-api-key", key);
    }
    if let Some(version) = anthropic_version {
        outbound_builder = outbound_builder.header("anthropic-version", version);
    }
    if let Some(beta) = anthropic_beta {
        outbound_builder = outbound_builder.header("anthropic-beta", beta);
    }
    let outbound_request = outbound_builder
        .body(Body::from(rewritten_body))
        .map_err(|_| ProxyError::Internal)?;

    send_upstream_request(state, outbound_request).await
}
