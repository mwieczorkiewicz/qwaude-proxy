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

    role_coercion_proxy::logging::init(&config.log_level);

    let listen_addr = config.listen_addr.clone();
    tracing::info!(
        listen_addr = %listen_addr,
        vllm_base_url = %config.vllm_base_url,
        "starting role-coercion-proxy"
    );

    let state = AppState::new(config);
    let app = build_router(state);

    let listener = match TcpListener::bind(&listen_addr).await {
        Ok(listener) => listener,
        Err(err) => {
            tracing::error!(error = %err, listen_addr = %listen_addr, "failed to bind listen address");
            return ExitCode::FAILURE;
        }
    };

    let result = axum::serve(listener, app)
        .with_graceful_shutdown(role_coercion_proxy::shutdown::signal())
        .await;

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            tracing::error!(error = %err, "server error");
            ExitCode::FAILURE
        }
    }
}
