use crate::app::AppState;
use crate::handlers;
use crate::traits::Platform;
use axum::Router;
use axum::routing::get;

pub fn build_router<P: Platform>(state: AppState<P>) -> Router {
    Router::new()
        .route("/api/health", get(handlers::health))
        .with_state(state)
}
