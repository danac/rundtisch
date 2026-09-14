use crate::platform::Platform;
use axum::Router;
use std::sync::Arc;
use tower_service::Service;
use worker::{Env, HttpRequest};

pub struct CloudflarePlatform {
    env: Env,
}

pub struct CloudflareSecrets;

impl Platform for CloudflarePlatform {
    type SecretStore = CloudflareSecrets;
    fn secrets(&self) -> Arc<Self::SecretStore> {
        Arc::new(CloudflareSecrets)
    }
}

impl CloudflarePlatform {
    pub fn new(env: Env) -> Self {
        Self { env }
    }

    pub fn env(&self) -> &Env {
        &self.env
    }
}

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
