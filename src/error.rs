//! Request-level error type and its mapping to an HTTP response.
//!
//! `ProxyError` composes `transform::TransformError` plus upstream-forwarding
//! failures and is the *only* place in the crate that maps an error to an
//! HTTP status code and the confirmed `{"error": {"message", "type"}}` JSON
//! body. Never the raw upstream error text or a stack trace.

use crate::transform::TransformError;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

/// The `type` discriminant in the JSON error body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorType {
    InvalidRequest,
    UpstreamError,
    InternalError,
}

impl ErrorType {
    fn as_str(self) -> &'static str {
        match self {
            ErrorType::InvalidRequest => "invalid_request",
            ErrorType::UpstreamError => "upstream_error",
            ErrorType::InternalError => "internal_error",
        }
    }
}

/// Request/proxy-level error. Every request-path fallible operation that
/// isn't a pure transform failure returns this type; it is mapped to an
/// HTTP response by `IntoResponse` below, and nowhere else in the crate.
#[derive(Debug, thiserror::Error)]
pub enum ProxyError {
    /// The request body could not be parsed/coerced (malformed JSON, not an
    /// object, missing `messages` array).
    #[error("invalid request: {0}")]
    InvalidRequest(#[from] TransformError),

    /// The request body exceeded `MAX_REQUEST_BODY_SIZE`.
    #[error("request body too large")]
    BodyTooLarge,

    /// Failed to establish a connection to `VLLM_BASE_URL`, or the upstream
    /// connection was reset/dropped mid-request.
    #[error("upstream connection failed")]
    UpstreamConnectFailed,

    /// The upstream did not respond within the configured timeout.
    #[error("upstream request timed out")]
    UpstreamTimeout,

    /// Any other unexpected internal failure (e.g. failed to construct the
    /// outbound request). Kept generic on the wire — the real cause is only
    /// ever logged, never returned to the client.
    #[error("internal error")]
    Internal,
}

impl ProxyError {
    /// `pub(crate)` so `server.rs`'s metrics wrapper can label
    /// `proxy_requests_total` with the same status this error maps to,
    /// without duplicating the match.
    pub(crate) fn status(&self) -> StatusCode {
        match self {
            ProxyError::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            ProxyError::BodyTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            ProxyError::UpstreamConnectFailed => StatusCode::BAD_GATEWAY,
            ProxyError::UpstreamTimeout => StatusCode::GATEWAY_TIMEOUT,
            ProxyError::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// The `kind` label value for `proxy_upstream_errors_total`, per
    /// `nfr-design/observability-design.md` NFR5.3 (`connect_timeout`,
    /// `request_timeout`, `non_2xx`). `None` for errors that never reached
    /// the upstream at all (a malformed request, an oversized body).
    /// `non_2xx` itself is recorded directly in `server.rs`, next to where
    /// the upstream's status is read — a non-2xx upstream response is not a
    /// `ProxyError` at all, since it is still passed through verbatim.
    pub(crate) fn upstream_error_kind(&self) -> Option<&'static str> {
        match self {
            ProxyError::UpstreamConnectFailed => Some("connect_timeout"),
            ProxyError::UpstreamTimeout => Some("request_timeout"),
            ProxyError::InvalidRequest(_) | ProxyError::BodyTooLarge | ProxyError::Internal => None,
        }
    }

    fn error_type(&self) -> ErrorType {
        match self {
            ProxyError::InvalidRequest(_) | ProxyError::BodyTooLarge => ErrorType::InvalidRequest,
            ProxyError::UpstreamConnectFailed | ProxyError::UpstreamTimeout => {
                ErrorType::UpstreamError
            }
            ProxyError::Internal => ErrorType::InternalError,
        }
    }

    /// The short, generic, status-appropriate message placed on the wire.
    /// Deliberately never includes the upstream's own error text.
    fn public_message(&self) -> &'static str {
        match self {
            ProxyError::InvalidRequest(_) => {
                "the request body is not a valid chat-completion request"
            }
            ProxyError::BodyTooLarge => "the request body exceeds the maximum allowed size",
            ProxyError::UpstreamConnectFailed => "failed to reach the upstream model server",
            ProxyError::UpstreamTimeout => "the upstream model server did not respond in time",
            ProxyError::Internal => "an internal error occurred",
        }
    }
}

#[derive(Serialize)]
struct ErrorBody<'a> {
    error: ErrorDetail<'a>,
}

#[derive(Serialize)]
struct ErrorDetail<'a> {
    message: &'a str,
    r#type: &'a str,
}

impl IntoResponse for ProxyError {
    fn into_response(self) -> Response {
        let status = self.status();
        let body = ErrorBody {
            error: ErrorDetail {
                message: self.public_message(),
                r#type: self.error_type().as_str(),
            },
        };
        (status, Json(body)).into_response()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use serde_json::Value;

    async fn body_json(err: ProxyError) -> (StatusCode, Value) {
        let response = err.into_response();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value: Value = serde_json::from_slice(&bytes).unwrap();
        (status, value)
    }

    #[tokio::test]
    async fn invalid_request_maps_to_400() {
        let source = TransformError::MissingMessagesArray;
        let (status, body) = body_json(ProxyError::InvalidRequest(source)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["error"]["type"], "invalid_request");
        assert!(body["error"]["message"].is_string());
    }

    #[tokio::test]
    async fn body_too_large_maps_to_413() {
        let (status, body) = body_json(ProxyError::BodyTooLarge).await;
        assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(body["error"]["type"], "invalid_request");
    }

    #[tokio::test]
    async fn upstream_connect_failed_maps_to_502() {
        let (status, body) = body_json(ProxyError::UpstreamConnectFailed).await;
        assert_eq!(status, StatusCode::BAD_GATEWAY);
        assert_eq!(body["error"]["type"], "upstream_error");
    }

    #[tokio::test]
    async fn upstream_timeout_maps_to_504() {
        let (status, body) = body_json(ProxyError::UpstreamTimeout).await;
        assert_eq!(status, StatusCode::GATEWAY_TIMEOUT);
        assert_eq!(body["error"]["type"], "upstream_error");
    }

    #[tokio::test]
    async fn internal_error_maps_to_500_and_never_leaks_detail() {
        let (status, body) = body_json(ProxyError::Internal).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body["error"]["type"], "internal_error");
        assert_eq!(body["error"]["message"], "an internal error occurred");
    }

    #[test]
    fn upstream_error_kind_matches_observability_design_vocabulary() {
        assert_eq!(
            ProxyError::UpstreamConnectFailed.upstream_error_kind(),
            Some("connect_timeout")
        );
        assert_eq!(
            ProxyError::UpstreamTimeout.upstream_error_kind(),
            Some("request_timeout")
        );
        assert_eq!(ProxyError::BodyTooLarge.upstream_error_kind(), None);
        assert_eq!(ProxyError::Internal.upstream_error_kind(), None);
    }

    #[test]
    fn error_body_never_contains_extra_top_level_fields() {
        let body = ErrorBody {
            error: ErrorDetail {
                message: "x",
                r#type: "invalid_request",
            },
        };
        let value = serde_json::to_value(&body).unwrap();
        let obj = value.as_object().unwrap();
        assert_eq!(obj.len(), 1);
        assert!(obj.contains_key("error"));
    }
}
