//! Prometheus metrics: names, recorder installation, and the metric this
//! module owns rendering for at `GET /metrics` (the route itself lives in
//! `server.rs`, next to the code that records these).

use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::sync::OnceLock;

/// Request volume by outcome, labeled by final HTTP status code (`status`).
pub const REQUESTS_TOTAL: &str = "proxy_requests_total";
/// End-to-end request handling latency (request received -> response sent).
pub const REQUEST_DURATION_SECONDS: &str = "proxy_request_duration_seconds";
/// Cumulative count of role-coerced messages, across all requests --
/// operational visibility into how often the original bug's trigger
/// condition actually fires.
pub const MESSAGES_COERCED_TOTAL: &str = "proxy_messages_coerced_total";
/// Upstream failure volume, labeled by cause (`kind`): `connect_timeout`,
/// `request_timeout`, or `non_2xx`, per
/// `nfr-design/observability-design.md` NFR5.3.
pub const UPSTREAM_ERRORS_TOTAL: &str = "proxy_upstream_errors_total";

/// The process-wide Prometheus recorder's handle, used to render
/// `GET /metrics`. Installed globally exactly once per process (`OnceLock`)
/// — the `metrics` facade only allows one global recorder per process, and
/// `AppState::new` is called once per test in the integration-test suite
/// (many `AppState`s sharing one process), not just once in production.
pub fn shared_handle() -> PrometheusHandle {
    static HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();
    HANDLE
        .get_or_init(|| {
            PrometheusBuilder::new()
                .install_recorder()
                .unwrap_or_else(|_| {
                    // A global recorder is already installed -- expected
                    // from the second `AppState` built in one process
                    // onward. Fall back to a local recorder's handle so
                    // /metrics still renders something instead of the
                    // route failing outright.
                    PrometheusBuilder::new().build_recorder().handle()
                })
        })
        .clone()
}
