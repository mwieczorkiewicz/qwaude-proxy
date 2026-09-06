# Integration Test Instructions — qwaude-proxy

Standard test strategy: key boundary tests, cross-unit interaction (this is a single-crate, zero-Unit project, so "cross-unit" here means the integration between `Transform`, `ProxyServer`, and the mocked vLLM boundary).

## Test framework setup

- `wiremock` (dev-dependency) mocks the vLLM upstream for most tests (non-streaming responses, error conditions, auth header matching).
- A hand-rolled raw-socket mock server (`tests/common/mod.rs::spawn_chunked_upstream`) simulates an incrementally-arriving chunked/SSE response for the two tests that need real inter-chunk timing, which `wiremock`'s canned-response API cannot produce.
- `tower::ServiceExt::oneshot` drives the `axum::Router` in-process — no real TCP listener needed for these tests.

## How to run

```bash
cargo test --test integration
cargo test --test build_and_test_checks
```

Both are scoped test-binary targets (`tests/integration.rs`, `tests/build_and_test_checks.rs` — the latter added at this stage; see Build and Test Summary for why).

## Test inventory

| Test | Boundary verified |
|---|---|
| `non_streaming_response_round_trips_verbatim` | ProxyServer ↔ mocked vLLM, non-streaming pass-through |
| `authorization_header_is_forwarded_unchanged` | ProxyServer ↔ mocked vLLM, auth header pass-through |
| `large_message_array_is_forwarded_successfully` | ProxyServer ↔ Transform, large payload handling |
| `malformed_json_body_returns_400_invalid_request` | Client ↔ ProxyServer, input validation boundary |
| `upstream_connection_refused_returns_502` | ProxyServer ↔ mocked vLLM, connection-failure boundary |
| `upstream_timeout_returns_504` | ProxyServer ↔ mocked vLLM, timeout boundary |
| `oversized_body_returns_413` | Client ↔ ProxyServer, body-size-limit boundary |
| `streaming_response_is_forwarded_incrementally_not_buffered` | ProxyServer ↔ mocked vLLM, streaming pass-through boundary |
| `health_endpoint_returns_200` (added this stage) | Client ↔ ProxyServer, health boundary, independent of vLLM reachability |
| `concurrent_requests_are_handled_without_serialization` (added this stage) | ProxyServer's async runtime, concurrent-request boundary |

## Expected coverage targets

Every boundary the original task spec and the confirmed NFR requirements name explicitly (streaming vs. non-streaming, malformed input, upstream failure modes, auth pass-through, body-size limit, concurrency, health) has at least one dedicated integration test — 10 tests total across the two files, within the Standard strategy's "5-8 tests per component plus integration tests for key boundaries" volume guidance (the component-level unit-test counts are reported separately in `code-summary.md`).

## Test data management

- Request bodies are constructed inline via `serde_json::json!` macros or the shared `common::sample_request_body()`/`common::test_config()` helpers — no external fixture files.
- `common::test_config()` uses short timeouts (500ms connect / 2s total) and a small 1 MiB body limit specifically so timeout and oversized-body tests don't need to wait out or allocate at production-scale defaults.
