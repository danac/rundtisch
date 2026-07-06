use std::sync::Arc;

pub trait Platform: Send + Sync + 'static {
    type SecretStore: Send + Sync;

    fn secrets(&self) -> Arc<Self::SecretStore>;
}

pub trait SecretStore {
    fn get_secret_by_name(&self, name: String) -> Option<String>;
}
