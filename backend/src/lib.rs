mod app_state;
mod handlers;
mod routes;
mod traits;

use crate::app_state::AppState;
use crate::traits::Platform;
use std::sync::Arc;
use tower_service::Service;
use worker::*;

struct CloudFlareWorker;
struct CloudFlareSecrets;

impl Platform for CloudFlareWorker {
    type SecretStore = CloudFlareSecrets;
    fn secrets(&self) -> Arc<Self::SecretStore> {
        Arc::new(CloudFlareSecrets)
    }
}

#[event(fetch)]
async fn fetch(
    req: HttpRequest,
    _env: Env,
    _ctx: Context,
) -> Result<axum::http::Response<axum::body::Body>> {
    let platform = CloudFlareWorker {};
    let state = AppState::from_platform(&platform);

    Ok(routes::router(state).call(req).await?)
}
