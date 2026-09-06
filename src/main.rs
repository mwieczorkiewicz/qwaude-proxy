//! `role-coercion-proxy` — an HTTP proxy sitting in front of a vLLM chat-completions
//! endpoint that rewrites mid-conversation `role: "system"` messages so vLLM's chat
//! template does not reject them.

use role_coercion_proxy::config::ProxyConfig;
use role_coercion_proxy::server::{build_router, AppState};
use std::process::ExitCode;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> ExitCode {
    let config = match ProxyConfig::from_env() {
        Ok(config) => config,
        Err(err) => {
            eprintln!("configuration error: {err}");
            return ExitCode::FAILURE;
        }
    };

    let listen_addr = config.listen_addr.clone();
    let state = AppState::new(config);
    let app = build_router(state);

    let listener = match TcpListener::bind(&listen_addr).await {
        Ok(listener) => listener,
        Err(err) => {
            eprintln!("failed to bind {listen_addr}: {err}");
            return ExitCode::FAILURE;
        }
    };

    // Graceful-shutdown wiring lands in Step 9 (code-generation-plan.md);
    // this is the plain `axum::serve` stub the plan calls for at Step 7.
    match axum::serve(listener, app).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("server error: {err}");
            ExitCode::FAILURE
        }
    }
}
