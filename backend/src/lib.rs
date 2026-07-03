use axum::{http::StatusCode, routing::get, Router};
use tower_service::Service;
use worker::*;

async fn health() -> StatusCode {
    StatusCode::OK
}

fn router() -> Router {
    Router::new().route("/api/health", get(health))
}

#[event(fetch)]
async fn fetch(
    req: HttpRequest,
    _env: Env,
    _ctx: Context,
) -> Result<axum::http::Response<axum::body::Body>> {
    Ok(router().call(req).await?)
}
