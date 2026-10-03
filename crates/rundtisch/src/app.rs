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
#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub email: Arc<dyn EmailSender>,
}

impl AppState {
    pub fn secret(&self, name: &str) -> Result<String, SecretError> {
        std::env::var(name).map_err(|_| SecretError::NotFound)
    }
}
