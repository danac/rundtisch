use crate::platform::Platform;
use axum::Router;
use std::sync::Arc;
use tokio::signal;

#[derive(Default)]
pub struct NativePlatform;

pub struct EnvironmentVariableSecrets;

impl Platform for NativePlatform {
    type SecretStore = EnvironmentVariableSecrets;
    fn secrets(&self) -> Arc<Self::SecretStore> {
        Arc::new(EnvironmentVariableSecrets)
    }
}

impl NativePlatform {
    pub fn new() -> Self {
        Self
    }
}

/// Bind `0.0.0.0:8080` and serve `router` until Ctrl+C or SIGTERM.
///
/// `router` must already have state applied (Axum `Router::with_state`).
pub async fn serve(router: Router) {
    serve_at(router, "0.0.0.0:8080").await;
}

pub async fn serve_at(router: Router, addr: &str) {
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!(
        "Starting server, listening on {} (http://localhost:{})...",
        listener.local_addr().unwrap(),
        listener.local_addr().unwrap().port()
    );
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
    println!("Server stopped");
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
