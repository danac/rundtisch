use crate::traits::Platform;

use std::sync::Arc;
use worker::Env;

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
