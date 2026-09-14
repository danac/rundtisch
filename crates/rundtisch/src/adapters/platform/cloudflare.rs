use crate::traits::Platform;
use std::sync::Arc;
use worker::Env;

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
