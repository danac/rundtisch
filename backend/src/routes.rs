use crate::app_state::AppState;
use crate::handlers;
use crate::traits::Platform;
use axum::routing::get;
use axum::Router;
use std::sync::Arc;

pub fn router<P: Platform>(state: AppState<P>) -> Router {
    Router::new()
        .route("/api/health", get(handlers::health))
        .with_state(state)
}
