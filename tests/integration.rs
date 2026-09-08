//! Integration tests for the HTTP proxy, against a `wiremock`-mocked upstream
//! (or the hand-rolled chunked-response mock in `tests/common` for the
//! streaming test).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use std::time::{Duration, Instant};
use tower::ServiceExt;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn post_json(router: axum::Router, body: serde_json::Value) -> axum::response::Response {
    let request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&body).expect("serialize test body"),
        ))
        .expect("build test request");
    router.oneshot(request).await.expect("router is infallible")
}

async fn response_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("collect response body")
        .to_bytes();
    serde_json::from_slice(&bytes).expect("response body is valid json")
}

#[tokio::test]
async fn non_streaming_response_round_trips_verbatim() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "chatcmpl-test",
            "choices": [{"index": 0, "message": {"role": "assistant", "content": "hello"}}],
        })))
        .mount(&mock_server)
        .await;

    let router = common::test_router(mock_server.uri());
    let response = post_json(router, common::sample_request_body(false)).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["id"], "chatcmpl-test");
    assert_eq!(body["choices"][0]["message"]["content"], "hello");
}

#[tokio::test]
async fn authorization_header_is_forwarded_unchanged() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("authorization", "Bearer test-token-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))
        .mount(&mock_server)
        .await;

    let router = common::test_router(mock_server.uri());
    let request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-token-123")
        .body(Body::from(
            serde_json::to_vec(&common::sample_request_body(false)).expect("serialize"),
        ))
        .expect("build request");
    let response = router.oneshot(request).await.expect("router is infallible");
    // wiremock only matched the mock (and thus returned 200) if the
    // Authorization header arrived at the upstream unchanged.
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn large_message_array_is_forwarded_successfully() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))
        .mount(&mock_server)
        .await;

    let mut messages = vec![serde_json::json!({"role": "user", "content": "start"})];
    for i in 0..500 {
        messages.push(serde_json::json!({"role": "system", "content": format!("reminder {i}")}));
        messages.push(serde_json::json!({"role": "user", "content": format!("question {i}")}));
    }
    let body = serde_json::json!({"model": "qwen", "stream": false, "messages": messages});

    let router = common::test_router(mock_server.uri());
    let response = post_json(router, body).await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn malformed_json_body_returns_400_invalid_request() {
    let router = common::test_router("http://127.0.0.1:1".to_string()); // never dialed
    let request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .body(Body::from("not json"))
        .expect("build request");
    let response = router.oneshot(request).await.expect("router is infallible");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response_json(response).await;
    assert_eq!(body["error"]["type"], "invalid_request");
}

#[tokio::test]
async fn upstream_connection_refused_returns_502() {
    // Bind then immediately drop a listener to obtain a port nothing is
    // listening on, so the outbound connect fails deterministically.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");
    drop(listener);

    let router = common::test_router(format!("http://{addr}"));
    let response = post_json(router, common::sample_request_body(false)).await;
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    let body = response_json(response).await;
    assert_eq!(body["error"]["type"], "upstream_error");
}

#[tokio::test]
async fn upstream_timeout_returns_504() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"ok": true}))
                .set_delay(Duration::from_secs(3)), // longer than test_config's 2s total_timeout
        )
        .mount(&mock_server)
        .await;

    let router = common::test_router(mock_server.uri());
    let response = post_json(router, common::sample_request_body(false)).await;
    assert_eq!(response.status(), StatusCode::GATEWAY_TIMEOUT);
    let body = response_json(response).await;
    assert_eq!(body["error"]["type"], "upstream_error");
}

#[tokio::test]
async fn oversized_body_returns_413() {
    let router = common::test_router("http://127.0.0.1:1".to_string()); // never dialed
                                                                        // test_config's max_request_body_size is 1 MiB.
    let oversized_content = "x".repeat(2 * 1024 * 1024);
    let body = serde_json::json!({
        "model": "qwen",
        "messages": [{"role": "user", "content": oversized_content}],
    });
    let response = post_json(router, body).await;
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    let body = response_json(response).await;
    assert_eq!(body["error"]["type"], "invalid_request");
}

#[tokio::test]
async fn streaming_response_is_forwarded_incrementally_not_buffered() {
    let chunks = vec![
        "data: chunk-1\n\n",
        "data: chunk-2\n\n",
        "data: chunk-3\n\n",
        "data: [DONE]\n\n",
    ];
    let per_chunk_delay = Duration::from_millis(150);
    let expected_total = per_chunk_delay * (chunks.len() as u32);
    let upstream_url = common::spawn_chunked_upstream(chunks, per_chunk_delay).await;
    let router = common::test_router(upstream_url);

    let request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&common::sample_request_body(true)).expect("serialize"),
        ))
        .expect("build request");

    let start = Instant::now();
    let response = router.oneshot(request).await.expect("router is infallible");
    assert_eq!(response.status(), StatusCode::OK);

    let mut body_stream = response.into_body();
    let mut first_byte_at = None;
    while let Some(frame) = body_stream.frame().await {
        let frame = frame.expect("frame read ok");
        if frame.data_ref().is_some() && first_byte_at.is_none() {
            first_byte_at = Some(start.elapsed());
        }
    }
    let first_byte_at = first_byte_at.expect("stream produced at least one data frame");

    // The proxy must relay the first chunk well before the upstream has
    // finished sending the *last* one -- proof it isn't buffering the whole
    // response before forwarding any of it.
    assert!(
        first_byte_at < expected_total,
        "first byte arrived at {first_byte_at:?}, expected well under the \
         {expected_total:?} it takes the upstream to finish streaming"
    );
}

#[tokio::test]
async fn models_list_is_passthrough() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "object": "list",
            "data": [{"id": "qwen3.6-35b-a3b", "object": "model"}],
        })))
        .mount(&mock_server)
        .await;

    let router = common::test_router(mock_server.uri());
    let request = Request::builder()
        .method("GET")
        .uri("/v1/models")
        .header("authorization", "Bearer test-token")
        .body(Body::empty())
        .expect("build request");
    let response = router.oneshot(request).await.expect("router is infallible");
    assert_eq!(response.status(), StatusCode::OK);
    let json = response_json(response).await;
    assert_eq!(json["data"][0]["id"], "qwen3.6-35b-a3b");
}

#[tokio::test]
async fn embeddings_route_is_passthrough_without_coercion() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/embeddings"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "object": "list",
            "data": [{"object": "embedding", "embedding": [0.1, 0.2]}],
        })))
        .mount(&mock_server)
        .await;

    let router = common::test_router(mock_server.uri());
    let body = serde_json::json!({
        "model": "qwen3.6-35b-a3b",
        "input": "hello",
    });
    let request = Request::builder()
        .method("POST")
        .uri("/v1/embeddings")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).expect("serialize")))
        .expect("build request");
    let response = router.oneshot(request).await.expect("router is infallible");
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn anthropic_messages_route_coerces_and_forwards() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "msg_test",
            "type": "message",
            "role": "assistant",
            "content": [{"type": "text", "text": "hello"}],
        })))
        .mount(&mock_server)
        .await;

    let router = common::test_router(mock_server.uri());
    let body = serde_json::json!({
        "model": "qwen3.6-35b-a3b",
        "max_tokens": 16,
        "messages": [
            {"role": "user", "content": "hi"},
            {"role": "system", "content": "mid-turn reminder"},
            {"role": "user", "content": "hai"}
        ]
    });
    let request = Request::builder()
        .method("POST")
        .uri("/v1/messages")
        .header("content-type", "application/json")
        .header("anthropic-version", "2023-06-01")
        .body(Body::from(serde_json::to_vec(&body).expect("serialize")))
        .expect("build request");
    let response = router.oneshot(request).await.expect("router is infallible");
    assert_eq!(response.status(), StatusCode::OK);
}
