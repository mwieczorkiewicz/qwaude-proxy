//! Shared integration-test setup: builds a router pointed at either a
//! `wiremock` mock upstream or a small hand-rolled raw-socket upstream (for
//! the streaming-backpressure test, which needs real inter-chunk timing that
//! wiremock's canned-response API cannot produce).
//!
//! Each file under `tests/` compiles as its own separate binary crate, so
//! not every helper here is used by every one of them; `dead_code` is
//! allowed for that reason rather than because anything here is unused in
//! the suite as a whole.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, dead_code)]

use qwaude_proxy::config::ProxyConfig;
use qwaude_proxy::server::{build_router, AppState};
use serde_json::json;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// A `ProxyConfig` pointed at `vllm_base_url`, with short timeouts (so a
/// timeout test doesn't need to wait out the 30s production default) and a
/// small body limit (so an oversized-body test doesn't need a multi-MiB
/// payload).
pub fn test_config(vllm_base_url: String) -> ProxyConfig {
    ProxyConfig {
        listen_addr: "127.0.0.1:0".to_string(),
        vllm_base_url,
        notice_prefix: "[System Notification] ".to_string(),
        log_level: "info".to_string(),
        max_request_body_size: 1024 * 1024, // 1 MiB
        connect_timeout: Duration::from_millis(500),
        total_timeout: Duration::from_secs(2),
        verbose_payload_logging: false,
    }
}

/// Build a fully wired router for `router.oneshot(request)`-style in-process
/// integration tests.
pub fn test_router(vllm_base_url: String) -> axum::Router {
    build_router(AppState::new(test_config(vllm_base_url)))
}

/// A minimal, valid chat-completion request body: a leading user message
/// plus one mid-conversation system message that must be coerced.
pub fn sample_request_body(stream: bool) -> serde_json::Value {
    json!({
        "model": "qwen",
        "stream": stream,
        "messages": [
            {"role": "user", "content": "hi"},
            {"role": "system", "content": "reminder"},
        ]
    })
}

/// Spawn a minimal HTTP/1.1 server that streams a chunked response, waiting
/// `delay_between_chunks` between each chunk it writes. Returns the mock
/// upstream's base URL (`http://127.0.0.1:<port>`).
///
/// `wiremock`'s `ResponseTemplate` always hands back one already-complete
/// body, so it cannot simulate a slow, incrementally-arriving stream — this
/// hand-rolled server is what lets the streaming-pass-through test observe
/// a real gap between the first and last bytes arriving.
pub async fn spawn_chunked_upstream(
    chunks: Vec<&'static str>,
    delay_between_chunks: Duration,
) -> String {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock upstream");
    let addr = listener.local_addr().expect("mock upstream local addr");

    tokio::spawn(async move {
        let Ok((mut socket, _)) = listener.accept().await else {
            return;
        };

        // Drain the request up to the blank line that ends the headers;
        // this proxy sends a headers-then-body request, no chunked request
        // body, so a fixed-size read loop is sufficient.
        let mut buf = [0u8; 8192];
        loop {
            match socket.read(&mut buf).await {
                Ok(0) => return,
                Err(_) => return,
                Ok(n) if buf[..n].windows(4).any(|w| w == b"\r\n\r\n") => break,
                Ok(_) => continue,
            }
        }

        let header =
            b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ntransfer-encoding: chunked\r\n\r\n";
        if socket.write_all(header).await.is_err() {
            return;
        }
        for chunk in chunks {
            let framed = format!("{:x}\r\n{chunk}\r\n", chunk.len());
            if socket.write_all(framed.as_bytes()).await.is_err() {
                return;
            }
            if socket.flush().await.is_err() {
                return;
            }
            tokio::time::sleep(delay_between_chunks).await;
        }
        let _ = socket.write_all(b"0\r\n\r\n").await;
    });

    format!("http://{addr}")
}
