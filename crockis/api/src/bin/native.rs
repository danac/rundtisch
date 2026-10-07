use std::sync::Arc;

use axum::Router;
use crockis::listen_addr;
use crockis::native_platform;
use crockis::static_dir;
use crockis::with_frontend;
use crockis::{build_router, load_environment};
use rundtisch::{AppState, SmtpEmailSender};
use tokio::signal;

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() {
    load_environment();

    let db = native_platform::connect()
        .await
        .expect("database connection");
    let email = SmtpEmailSender::from_env().expect("SMTP email configuration");
    let state = AppState {
        db,
        email: Arc::new(email),
    };
    let mut router = build_router(state);
    if let Some(dir) = static_dir() {
        println!("Serving frontend from {}", dir.display());
        router = with_frontend(router, &dir);
    }
    serve(router).await;
}

/// Bind the listen address and serve `router` until Ctrl+C or SIGTERM.
async fn serve(router: Router) {
    let addr = listen_addr();
    serve_at(router, &addr).await;
}

async fn serve_at(router: Router, addr: &str) {
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
