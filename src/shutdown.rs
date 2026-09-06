//! Graceful shutdown: resolves on SIGINT (Ctrl+C) or, on Unix, SIGTERM.

use tokio::signal;

/// Passed to `axum::serve(...).with_graceful_shutdown(...)`: axum stops
/// accepting new connections once this future resolves and waits for
/// in-flight requests — including long-lived streaming ones — to finish
/// before the server actually exits.
pub async fn signal() {
    let ctrl_c = async {
        if let Err(err) = signal::ctrl_c().await {
            tracing::error!(error = %err, "failed to install Ctrl+C handler");
            // Nothing sensible to do but wait forever rather than treating a
            // failed handler install as an immediate shutdown signal.
            std::future::pending::<()>().await;
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match signal::unix::signal(signal::unix::SignalKind::terminate()) {
            Ok(mut term) => {
                term.recv().await;
            }
            Err(err) => {
                tracing::error!(error = %err, "failed to install SIGTERM handler");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {
            tracing::info!("received SIGINT, starting graceful shutdown");
        }
        () = terminate => {
            tracing::info!("received SIGTERM, starting graceful shutdown");
        }
    }
}
