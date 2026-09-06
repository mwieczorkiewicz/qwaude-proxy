//! Library crate for `qwaude-proxy`.
//!
//! Splitting the binary into a thin `main.rs` plus this library lets integration
//! tests and benchmarks construct the axum `Router` and call `transform` directly,
//! in-process, without spawning the compiled binary.

pub mod config;
pub mod error;
pub mod logging;
pub mod metrics;
pub mod server;
pub mod shutdown;
pub mod transform;
