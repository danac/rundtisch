use backend::adapters::platform::cloudflare::CloudFlareWorker;
use backend::app::AppState;
use backend::routes::build_router;
use tower_service::Service;
use worker::{Context, Env, HttpRequest};
use worker_macros::event;

#[event(fetch)]
async fn fetch(
    req: HttpRequest,
    env: Env,
    _ctx: Context,
) -> worker::Result<axum::http::Response<axum::body::Body>> {
    let platform = CloudFlareWorker::new(env);
    let state = AppState::from_platform(&platform);

    Ok(build_router(state).call(req).await?)
}
