//! Coverage added at the Build and Test stage to close two gaps the target
//! inventory found were claimed (NFR5.2, NFR1.4) but had no dedicated
//! automated check: the health endpoint, and concurrent-request handling.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use std::time::{Duration, Instant};
use tower::ServiceExt;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// NFR5.2: `GET /health` returns 200 with no upstream dependency check.
#[tokio::test]
async fn health_endpoint_returns_200() {
    // Point at an address nothing is listening on -- health must not depend
    // on vLLM being reachable.
    let router = common::test_router("http://127.0.0.1:1".to_string());
    let request = Request::builder()
        .method("GET")
        .uri("/health")
        .body(Body::empty())
        .expect("build health request");
    let response = router.oneshot(request).await.expect("router is infallible");
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("collect health body")
        .to_bytes();
    assert!(
        !bytes.is_empty(),
        "health response body should not be empty"
    );
}

/// NFR1.4: at least tens of concurrent requests, without one slow request
/// blocking another. Fires 20 concurrent requests against a mock upstream
/// with an artificial per-request delay and asserts the total wall-clock
/// time is far below what fully serial handling would take -- proof the
/// async runtime is actually running them concurrently, not queuing them.
#[tokio::test]
async fn concurrent_requests_are_handled_without_serialization() {
    const CONCURRENCY: usize = 20;
    const PER_REQUEST_DELAY: Duration = Duration::from_millis(200);

    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"ok": true}))
                .set_delay(PER_REQUEST_DELAY),
        )
        .mount(&mock_server)
        .await;

    let router = common::test_router(mock_server.uri());

    let start = Instant::now();
    let mut handles = Vec::with_capacity(CONCURRENCY);
    for _ in 0..CONCURRENCY {
        let router = router.clone();
        handles.push(tokio::spawn(async move {
            let request = Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&common::sample_request_body(false))
                        .expect("serialize test body"),
                ))
                .expect("build request");
            router.oneshot(request).await.expect("router is infallible")
        }));
    }

    for handle in handles {
        let response = handle.await.expect("request task did not panic");
        assert_eq!(response.status(), StatusCode::OK);
    }

    let elapsed = start.elapsed();
    // Fully serial handling of CONCURRENCY requests would take at least
    // CONCURRENCY * PER_REQUEST_DELAY. Concurrent handling should complete
    // in roughly one delay period plus overhead -- assert well under half
    // the serial-case duration as a generous, non-flaky bound.
    let serial_case = PER_REQUEST_DELAY * CONCURRENCY as u32;
    assert!(
        elapsed < serial_case / 2,
        "20 concurrent requests took {elapsed:?}; expected well under {:?} \
         (half the fully-serial bound of {serial_case:?}), which would indicate \
         requests are blocking one another instead of running concurrently",
        serial_case / 2
    );
}
