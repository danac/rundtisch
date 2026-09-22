use crate::handlers::health;
use axum::Router;
use axum::routing::{get, patch};
use rundtisch::auth::handlers::{create_user, delete_user, list_users, update_user};
use rundtisch::{AppState, Platform};

pub fn build_router<P: Platform>(state: AppState<P>) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/auth/users", get(list_users).post(create_user))
        .route("/api/auth/users/{public_id}", patch(update_user).delete(delete_user))
        .with_state(state)
}
