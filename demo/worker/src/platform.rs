use rundtisch::adapters::db::d1::D1Executor;
use rundtisch::adapters::secrets::WorkerSecretStore;
use rundtisch::traits::Platform;
use std::sync::Arc;
use worker::Env;

pub struct CloudflarePlatform {
    db: Arc<D1Executor>,
    secrets: Arc<WorkerSecretStore>,
}

impl Platform for CloudflarePlatform {
    type Database = D1Executor;
    type SecretStore = WorkerSecretStore;

    fn database(&self) -> Arc<Self::Database> {
        self.db.clone()
    }

    fn secrets(&self) -> Arc<Self::SecretStore> {
        self.secrets.clone()
    }
}

impl CloudflarePlatform {
    /// Resolve the D1 binding named `database_binding` from `env` and panic if it is missing.
    pub fn new(env: Env, database_binding: &str) -> Self {
        let db = env
            .d1(database_binding)
            .unwrap_or_else(|err| panic!("D1 binding `{database_binding}`: {err}"));
        Self {
            secrets: Arc::new(WorkerSecretStore::new(env)),
            db: Arc::new(D1Executor::new(db)),
        }
    }
}
