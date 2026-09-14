use crate::handlers;
use axum::Router;
use axum::routing::get;
use rundtisch::{AppState, Platform};

pub fn build_router<P: Platform>(state: AppState<P>) -> Router {
    Router::new()
        .route("/api/health", get(handlers::health))
        .with_state(state)
}
