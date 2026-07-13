use crate::traits::Platform;
use std::sync::Arc;

#[derive(Default)]
pub struct NativePlatform;

pub struct EnvironmentVariableSecrets;

impl Platform for NativePlatform {
    type SecretStore = EnvironmentVariableSecrets;
    fn secrets(&self) -> Arc<Self::SecretStore> {
        Arc::new(EnvironmentVariableSecrets)
    }
}

impl NativePlatform {
    pub fn new() -> Self {
        Default::default()
    }
}
