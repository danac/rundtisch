use backend::app::AppState;
use backend::adapters::platform::native::NativePlatform;
use backend::routes::build_router;
use tokio::signal;

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() {
    let platform = NativePlatform::new();
    let state = AppState::from_platform(&platform);
    let router = build_router(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
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
