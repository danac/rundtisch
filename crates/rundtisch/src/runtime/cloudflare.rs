use crate::adapters::platform::cloudflare::CloudflarePlatform;
use axum::Router;
use tower_service::Service;
use worker::{Env, HttpRequest};

/// Construct the platform from the Worker `env`, build a router, and dispatch `req`.
///
/// `build` must return a router that already has state applied (`Router::with_state`).
pub async fn handle_fetch(
    req: HttpRequest,
    env: Env,
    build: impl FnOnce(&CloudflarePlatform) -> Router,
) -> worker::Result<axum::http::Response<axum::body::Body>> {
    let platform = CloudflarePlatform::new(env);
    let mut router = build(&platform);
    Ok(router.call(req).await?)
}
