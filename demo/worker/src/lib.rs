use rundtisch::runtime::cloudflare::handle_fetch;
use rundtisch_demo::build_router;
use worker::{Context, Env, HttpRequest};
use worker_macros::event;

#[event(fetch)]
async fn fetch(
    req: HttpRequest,
    env: Env,
    _ctx: Context,
) -> worker::Result<axum::http::Response<axum::body::Body>> {
    handle_fetch(req, env, |platform| {
        build_router(rundtisch::AppState::from_platform(platform))
    })
    .await
}
