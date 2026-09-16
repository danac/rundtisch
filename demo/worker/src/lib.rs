use axum::Router;
use rundtisch::adapters::platform::cloudflare::CloudflarePlatform;
use rundtisch::AppState;
use rundtisch_demo::build_router;
use tower_service::Service;
use worker::{Context, Env, HttpRequest};
use worker_macros::event;

#[event(fetch)]
async fn fetch(
    req: HttpRequest,
    env: Env,
    _ctx: Context,
) -> worker::Result<axum::http::Response<axum::body::Body>> {
    handle_fetch(req, env, |platform| {
        build_router(AppState::from_platform(platform))
    })
    .await
}

/// Construct the platform from the Worker `env`, build a router, and dispatch `req`.
///
/// `build` must return a router that already has state applied (`Router::with_state`).
async fn handle_fetch(
    req: HttpRequest,
    env: Env,
    build: impl FnOnce(&CloudflarePlatform) -> Router,
) -> worker::Result<axum::http::Response<axum::body::Body>> {
    let platform = CloudflarePlatform::new(env);
    let mut router = build(&platform);
    Ok(router.call(req).await?)
}
