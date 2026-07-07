use crate::traits::Platform;
use crate::{routes, AppState};

use std::sync::Arc;
use tower_service::Service;
use worker::{Context, Env, HttpRequest};
use worker_macros::event;

pub struct CloudFlareWorker {
    _env: Env,
}
pub struct CloudFlareSecrets;

impl Platform for CloudFlareWorker {
    type SecretStore = CloudFlareSecrets;
    fn secrets(&self) -> Arc<Self::SecretStore> {
        Arc::new(CloudFlareSecrets)
    }
}

impl CloudFlareWorker {
    pub fn new(env: Env) -> Self {
        Self { _env: env }
    }
}

#[event(fetch)]
async fn fetch(
    req: HttpRequest,
    env: Env,
    _ctx: Context,
) -> worker::Result<axum::http::Response<axum::body::Body>> {
    let platform = CloudFlareWorker::new(env);
    let state = AppState::from_platform(&platform);

    Ok(routes::build_router(state).call(req).await?)
}
