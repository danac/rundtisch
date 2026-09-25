use rundtisch::adapters::db::sqlite::SqliteExecutor;
use rundtisch::adapters::secrets::EnvSecretStore;
use rundtisch::traits::Platform;
use std::path::Path;
use std::sync::Arc;

pub struct NativePlatform {
    db: Arc<SqliteExecutor>,
    secrets: Arc<EnvSecretStore>,
}

impl Platform for NativePlatform {
    type Database = SqliteExecutor;
    type SecretStore = EnvSecretStore;

    fn database(&self) -> Arc<Self::Database> {
        self.db.clone()
    }

    fn secrets(&self) -> Arc<Self::SecretStore> {
        self.secrets.clone()
    }
}

impl NativePlatform {
    /// Open (or create) the SQLite database at `database_path` and panic on failure.
    pub async fn new(database_path: impl AsRef<Path>) -> Self {
        Self {
            db: Arc::new(SqliteExecutor::new(database_path).await),
            secrets: Arc::new(EnvSecretStore),
        }
    }
}
