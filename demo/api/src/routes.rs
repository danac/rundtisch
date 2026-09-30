use crate::handlers::health;
use axum::Router;
use axum::routing::{delete, get, post, put};
use rundtisch::auth::handlers::{
    clear_password, list_passkeys, login, logout, logout_all, me, passkey_delete, passkey_login,
    passkey_login_options, passkey_register, passkey_register_options, register_passkey,
    register_passkey_options, register_password, register_with_token, request_reset, reset_passkey,
    reset_passkey_options, reset_password, set_password, step_up_login, step_up_passkey_login,
    step_up_passkey_login_options,
};
use rundtisch::AppState;

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route(
            "/api/auth/register_with_token",
            post(register_with_token),
        )
        .route("/api/auth/register/password", post(register_password))
        .route(
            "/api/auth/register/passkey/options",
            post(register_passkey_options),
        )
        .route("/api/auth/register/passkey", post(register_passkey))
        .route("/api/auth/login", post(login))
        .route(
            "/api/auth/passkeys/login/options",
            post(passkey_login_options),
        )
        .route("/api/auth/passkeys/login", post(passkey_login))
        .route("/api/auth/step-up/login", post(step_up_login))
        .route(
            "/api/auth/step-up/passkeys/login/options",
            post(step_up_passkey_login_options),
        )
        .route("/api/auth/step-up/passkeys/login", post(step_up_passkey_login))
        .route("/api/auth/logout", post(logout))
        .route("/api/auth/logout_all", post(logout_all))
        .route("/api/auth/me", get(me))
        .route("/api/auth/password", put(set_password).delete(clear_password))
        .route("/api/auth/request_reset", post(request_reset))
        .route("/api/auth/reset", post(reset_password))
        .route(
            "/api/auth/reset/passkey/options",
            post(reset_passkey_options),
        )
        .route("/api/auth/reset/passkey", post(reset_passkey))
        .route("/api/auth/passkeys", get(list_passkeys))
        .route(
            "/api/auth/passkeys/register/options",
            post(passkey_register_options),
        )
        .route("/api/auth/passkeys/register", post(passkey_register))
        .route(
            "/api/auth/passkeys/{public_id}",
            delete(passkey_delete),
        )
        .with_state(state)
}
