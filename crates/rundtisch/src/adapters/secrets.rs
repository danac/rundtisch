use crate::traits::secrets::{SecretError, SecretStore};
use std::collections::HashMap;

/// `std::env::var` — native deployments.
#[derive(Debug, Default, Clone, Copy)]
pub struct EnvSecretStore;

impl SecretStore for EnvSecretStore {
    fn get(&self, name: &str) -> Result<String, SecretError> {
        std::env::var(name).map_err(|_| SecretError::NotFound)
    }
}

/// In-memory map for tests.
#[derive(Debug, Clone, Default)]
pub struct MapSecretStore {
    values: HashMap<String, String>,
}

impl MapSecretStore {
    pub fn new(values: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>) -> Self {
        Self {
            values: values
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        }
    }
}

impl SecretStore for MapSecretStore {
    fn get(&self, name: &str) -> Result<String, SecretError> {
        self.values.get(name).cloned().ok_or(SecretError::NotFound)
    }
}

/// Wrangler / dashboard secrets via `worker::Env`.
#[cfg(feature = "d1")]
#[derive(Clone)]
pub struct WorkerSecretStore {
    env: worker::Env,
}

#[cfg(feature = "d1")]
impl WorkerSecretStore {
    pub fn new(env: worker::Env) -> Self {
        Self { env }
    }
}

#[cfg(feature = "d1")]
impl SecretStore for WorkerSecretStore {
    fn get(&self, name: &str) -> Result<String, SecretError> {
        self.env
            .secret(name)
            .map(|secret| secret.to_string())
            .map_err(|_| SecretError::NotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::secrets::HASH_PEPPER;

    #[test]
    fn map_store_returns_values() {
        let store = MapSecretStore::new([(HASH_PEPPER, "pepper-value")]);
        assert_eq!(store.get(HASH_PEPPER).unwrap(), "pepper-value");
        assert_eq!(store.get("missing").unwrap_err(), SecretError::NotFound);
    }
}
