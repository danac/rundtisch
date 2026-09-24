use crate::handlers::health;
use axum::Router;
use axum::routing::{get, patch, post};
use rundtisch::auth::handlers::{
    activate, create_user, delete_user, list_users, login, logout, me, refresh, register,
    update_user,
};
use rundtisch::{AppState, Platform};

pub fn build_router<P: Platform>(state: AppState<P>) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/auth/users", get(list_users).post(create_user))
        .route(
            "/api/auth/users/{public_id}",
            patch(update_user).delete(delete_user),
        )
        .route("/api/auth/register", post(register))
        .route("/api/auth/activate", post(activate))
        .route("/api/auth/login", post(login))
        .route("/api/auth/refresh", post(refresh))
        .route("/api/auth/logout", post(logout))
        .route("/api/auth/me", get(me))
        .with_state(state)
}
