//! Metrics endpoint verification (Step 10): `GET /metrics` renders the four
//! metrics named in `nfr-design/observability-design.md` NFR5.3, after a few
//! requests have been made through the router.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn get_metrics_text(router: axum::Router) -> String {
    let request = Request::builder()
        .method("GET")
        .uri("/metrics")
        .body(Body::empty())
        .expect("build metrics request");
    let response = router.oneshot(request).await.expect("router is infallible");
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("collect metrics body")
        .to_bytes();
    String::from_utf8(bytes.to_vec()).expect("metrics body is utf-8")
}

#[tokio::test]
async fn metrics_endpoint_reports_all_four_named_metrics_after_traffic() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))
        .mount(&mock_server)
        .await;

    let router = common::test_router(mock_server.uri());

    // One coercing request, so proxy_messages_coerced_total has something
    // to report.
    let request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&common::sample_request_body(false)).expect("serialize"),
        ))
        .expect("build request");
    let response = router
        .clone()
        .oneshot(request)
        .await
        .expect("router is infallible");
    assert_eq!(response.status(), StatusCode::OK);

    let metrics_text = get_metrics_text(router).await;

    for expected_metric in [
        "proxy_requests_total",
        "proxy_request_duration_seconds",
        "proxy_messages_coerced_total",
    ] {
        assert!(
            metrics_text.contains(expected_metric),
            "expected {expected_metric:?} in /metrics output:\n{metrics_text}"
        );
    }
    // proxy_requests_total must carry the status label, per NFR5.3's table.
    assert!(
        metrics_text.contains("proxy_requests_total{status=\"200\"}"),
        "expected a status-labeled proxy_requests_total sample:\n{metrics_text}"
    );
}

#[tokio::test]
async fn upstream_errors_total_is_recorded_with_a_kind_label_on_connect_failure() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");
    drop(listener); // nothing listens here -- the outbound connect fails

    let router = common::test_router(format!("http://{addr}"));
    let request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&common::sample_request_body(false)).expect("serialize"),
        ))
        .expect("build request");
    let response = router
        .clone()
        .oneshot(request)
        .await
        .expect("router is infallible");
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);

    let metrics_text = get_metrics_text(router).await;
    assert!(
        metrics_text.contains("proxy_upstream_errors_total{kind=\"connect_timeout\"}"),
        "expected a connect_timeout-labeled upstream error sample:\n{metrics_text}"
    );
}
