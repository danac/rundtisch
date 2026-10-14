use std::fmt;
use std::sync::Arc;

use sea_orm::DatabaseConnection;

use crate::email::EmailSender;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretError {
    NotFound,
}

impl fmt::Display for SecretError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SecretError::NotFound => write!(f, "secret not found"),
        }
    }
}

impl std::error::Error for SecretError {}

/// Application state shared by Axum handlers.
///
/// The connection is opened by the process entry point. This crate does not
/// choose SQLite, MySQL, or Postgres. The process also supplies the email
/// transport (`email`) used for recovery links and similar notices.
///
/// With the `storage` feature, `objects` is an [`crate::storage::ObjectStore`].
/// [`AppState::new`] installs a disabled backend so the demo can keep building
/// without a filesystem. Call [`AppState::with_storage`] to attach a real one.
#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub email: Arc<dyn EmailSender>,
    #[cfg(feature = "storage")]
    pub objects: crate::storage::ObjectStore,
}

impl AppState {
    pub fn new(db: DatabaseConnection, email: Arc<dyn EmailSender>) -> Self {
        Self {
            #[cfg(feature = "storage")]
            objects: crate::storage::ObjectStore::disabled(db.clone()),
            db,
            email,
        }
    }

    /// Replace the disabled object store with `storage`.
    #[cfg(feature = "storage")]
    pub fn with_storage(mut self, storage: Arc<dyn crate::storage::FileStorage>) -> Self {
        self.objects = crate::storage::ObjectStore::new(self.db.clone(), storage);
        self
    }

    pub fn secret(&self, name: &str) -> Result<String, SecretError> {
        std::env::var(name).map_err(|_| SecretError::NotFound)
    }
}
