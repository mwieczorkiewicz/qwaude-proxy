//! Structured-logging verification (Step 8): capture emitted log fields for
//! a coercing and a non-coercing request, and assert no payload content
//! leaks into the logs by default (`VERBOSE_PAYLOAD_LOGGING` off).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use std::io;
use std::sync::{Arc, Mutex};
use tower::ServiceExt;
use tracing_subscriber::fmt::MakeWriter;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// A `tracing_subscriber` writer that appends into a shared, readable
/// in-memory buffer, so a test can assert on exactly what was logged
/// without a global subscriber (which `tracing` only allows setting once
/// per process).
#[derive(Clone, Default)]
struct SharedBuffer(Arc<Mutex<Vec<u8>>>);

impl SharedBuffer {
    fn contents(&self) -> String {
        String::from_utf8_lossy(&self.0.lock().expect("buffer lock")).into_owned()
    }
}

impl io::Write for SharedBuffer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().expect("buffer lock").extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for SharedBuffer {
    type Writer = SharedBuffer;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

/// Run one POST /v1/chat/completions through the router with a scoped
/// (thread-local, not global) tracing subscriber at `directive`, and return
/// everything it logged as text.
async fn run_with_captured_logs(directive: &str, body: serde_json::Value) -> String {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))
        .mount(&mock_server)
        .await;

    let buffer = SharedBuffer::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(buffer.clone())
        .with_env_filter(tracing_subscriber::EnvFilter::new(directive))
        .with_ansi(false)
        .finish();

    let router = common::test_router(mock_server.uri());

    // `set_default` scopes the subscriber to this thread only, via a guard
    // dropped at the end of this block -- `#[tokio::test]` runs on a single
    // current-thread runtime, so it stays active across every `.await` in
    // between without needing (or risking corrupting) a process-global one.
    let _guard = tracing::subscriber::set_default(subscriber);
    let request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&body).expect("serialize test body"),
        ))
        .expect("build request");
    let response = router.oneshot(request).await.expect("router is infallible");
    assert_eq!(response.status(), StatusCode::OK);
    drop(_guard);

    buffer.contents()
}

#[tokio::test]
async fn coercing_request_logs_count_and_indices_but_not_payload_content() {
    let secret_content = "super-secret-system-reminder-payload";
    let body = serde_json::json!({
        "model": "qwen",
        "stream": false,
        "messages": [
            {"role": "user", "content": "hi"},
            {"role": "system", "content": secret_content},
        ]
    });

    let logs = run_with_captured_logs("debug", body).await;

    assert!(
        logs.contains("coerced_count"),
        "expected a coercion count field in the logs: {logs}"
    );
    assert!(
        logs.contains("coerced_indices"),
        "expected a coercion indices field in the logs: {logs}"
    );
    assert!(
        !logs.contains(secret_content),
        "message payload content leaked into the logs by default: {logs}"
    );
}

#[tokio::test]
async fn non_coercing_request_logs_no_coercion_line() {
    let body = serde_json::json!({
        "model": "qwen",
        "stream": false,
        "messages": [{"role": "user", "content": "hi"}],
    });

    let logs = run_with_captured_logs("debug", body).await;

    assert!(
        !logs.contains("coerced mid-conversation"),
        "expected no coercion log line for a request with nothing to coerce: {logs}"
    );
}

#[tokio::test]
async fn info_level_does_not_emit_the_debug_coercion_line() {
    let body = serde_json::json!({
        "model": "qwen",
        "stream": false,
        "messages": [
            {"role": "user", "content": "hi"},
            {"role": "system", "content": "reminder"},
        ]
    });

    let logs = run_with_captured_logs("info", body).await;

    assert!(
        !logs.contains("coerced mid-conversation"),
        "the debug-level coercion line should not appear when filtered at info: {logs}"
    );
}
