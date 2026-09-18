use rundtisch::adapters::db::d1::D1Executor;
use rundtisch::traits::Platform;
use std::sync::Arc;
use worker::Env;

pub struct CloudflarePlatform {
    env: Env,
    db: Arc<D1Executor>,
}

// Secrets will be wired later.
// pub struct CloudflareSecrets;

impl Platform for CloudflarePlatform {
    // type SecretStore = CloudflareSecrets;
    // fn secrets(&self) -> Arc<Self::SecretStore> {
    //     Arc::new(CloudflareSecrets)
    // }

    type Database = D1Executor;

    fn database(&self) -> Arc<Self::Database> {
        self.db.clone()
    }
}

impl CloudflarePlatform {
    /// Resolve the D1 binding named `database_binding` from `env` and panic if it is missing.
    pub fn new(env: Env, database_binding: &str) -> Self {
        let db = env
            .d1(database_binding)
            .unwrap_or_else(|err| panic!("D1 binding `{database_binding}`: {err}"));
        Self {
            env,
            db: Arc::new(D1Executor::new(db)),
        }
    }
}
