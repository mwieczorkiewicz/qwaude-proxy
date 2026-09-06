//! Structured logging setup: a `tracing_subscriber` filtered by `LOG_LEVEL`.
//!
//! This module owns process startup only (`init`, called once from
//! `main()`). Per-request logging itself — the `debug!`/`error!` calls, and
//! the `#[tracing::instrument]` span — lives in `server.rs`, next to the
//! code whose behavior they describe.

use tracing_subscriber::EnvFilter;

/// Initialize the process-global tracing subscriber from `log_level` (the
/// resolved `LOG_LEVEL` config value, e.g. `"info"`, `"debug"`). Falls back
/// to `"info"` if `log_level` isn't a valid `EnvFilter` directive, rather
/// than failing startup over a logging misconfiguration.
pub fn init(log_level: &str) {
    let filter = EnvFilter::try_new(log_level).unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .init();
}
