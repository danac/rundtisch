use worker::{Context, Env, HttpRequest};
use worker_macros::event;

#[event(fetch)]
async fn fetch(
    req: HttpRequest,
    env: Env,
    ctx: Context,
) -> worker::Result<axum::http::Response<axum::body::Body>> {
    backend::adapters::platform::cloudflare::handle_fetch(req, env, ctx).await
}
