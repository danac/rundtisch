use crate::adapters::db::sqlite::SqliteExecutor;
use crate::traits::Platform;
use std::path::Path;
use std::sync::Arc;

pub struct NativePlatform {
    db: Arc<SqliteExecutor>,
}

// Secrets will be wired later.
// pub struct EnvironmentVariableSecrets;

impl Platform for NativePlatform {
    // type SecretStore = EnvironmentVariableSecrets;
    // fn secrets(&self) -> Arc<Self::SecretStore> {
    //     Arc::new(EnvironmentVariableSecrets)
    // }

    type Database = SqliteExecutor;

    fn database(&self) -> Arc<Self::Database> {
        self.db.clone()
    }
}

impl NativePlatform {
    /// Open (or create) the SQLite database at `database_path` and panic on failure.
    pub async fn new(database_path: impl AsRef<Path>) -> Self {
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(database_path)
            .create_if_missing(true);
        let pool = sqlx::SqlitePool::connect_with(options)
            .await
            .expect("failed to open SQLite database");
        Self {
            db: Arc::new(SqliteExecutor::new(pool)),
        }
    }
}
