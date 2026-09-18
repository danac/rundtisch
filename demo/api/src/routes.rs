use crate::handlers::health;
use axum::Router;
use axum::routing::get;
use rundtisch::{AppState, Platform};
use rundtisch::auth::handlers::list_users;

pub fn build_router<P: Platform>(state: AppState<P>) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/auth/users", get(list_users))
        .with_state(state)
}
