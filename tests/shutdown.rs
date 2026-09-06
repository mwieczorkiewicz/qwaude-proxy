//! Graceful-shutdown integration test (Step 9): a slow/streaming in-flight
//! request must complete before the server actually exits, once shutdown is
//! signaled.
//!
//! `shutdown::signal()` itself waits on real OS signals (SIGINT/SIGTERM),
//! which this test deliberately does not raise -- doing so would affect
//! every test in this process, since signal delivery isn't scoped to one
//! thread. Instead this test drives `axum::serve(...).with_graceful_shutdown`
//! with a `oneshot` channel it controls directly, which is exactly the same
//! mechanism `shutdown::signal()`'s future provides to `with_graceful_
//! shutdown` in production -- what's under test is axum's drain behavior
//! once *a* shutdown future resolves, not which future that is.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use axum::body::Body;
use axum::http::Request;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

#[tokio::test]
async fn in_flight_streaming_request_completes_before_shutdown_finishes() {
    // A slow upstream: 4 chunks, 150ms apart (~600ms total), so there's
    // plenty of time to trigger shutdown mid-stream and still observe the
    // response draining afterward.
    let chunks = vec!["chunk-1\n", "chunk-2\n", "chunk-3\n", "chunk-4\n"];
    let upstream_url = common::spawn_chunked_upstream(chunks, Duration::from_millis(150)).await;

    let router = common::test_router(upstream_url);
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind proxy listener");
    let proxy_addr = listener.local_addr().expect("proxy local addr");

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let server_handle = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            })
            .await
            .expect("server exits cleanly");
    });

    let client: Client<HttpConnector, Body> = Client::builder(TokioExecutor::new()).build_http();
    let uri: axum::http::Uri = format!("http://{proxy_addr}/v1/chat/completions")
        .parse()
        .expect("valid uri");
    let request = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&common::sample_request_body(true)).expect("serialize"),
        ))
        .expect("build request");

    let in_flight = tokio::spawn(async move {
        let response = client.request(request).await.expect("request succeeds");
        http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("collect body")
            .to_bytes()
    });

    // Let the request start (and the first chunk or two arrive) before
    // signaling shutdown, so it's genuinely in-flight when the signal fires.
    tokio::time::sleep(Duration::from_millis(200)).await;
    shutdown_tx.send(()).expect("send shutdown signal");

    // The in-flight streaming request must still finish -- graceful
    // shutdown drains it rather than aborting it.
    let body = tokio::time::timeout(Duration::from_secs(3), in_flight)
        .await
        .expect("in-flight request did not hang past shutdown")
        .expect("in-flight task did not panic");
    let body_text = String::from_utf8_lossy(&body);
    for expected in ["chunk-1", "chunk-2", "chunk-3", "chunk-4"] {
        assert!(
            body_text.contains(expected),
            "expected {expected:?} in the fully-drained response body: {body_text}"
        );
    }

    // The server itself must actually exit once the drain completes.
    tokio::time::timeout(Duration::from_secs(3), server_handle)
        .await
        .expect("server task did not exit after graceful shutdown")
        .expect("server task did not panic");
}
