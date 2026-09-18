pub mod db;

use std::sync::Arc;

use db::DatabaseExecutor;

/// Host platform an application is running on.
///
/// The platform owns long-lived resources (databases, and later secrets and
/// object storage) that are cloned into [`crate::AppState`] and held by the Axum router.
pub trait Platform: 'static {
    // Secrets will be wired later.
    // /// Types must be `Send + Sync` because [`crate::AppState`] typically contains `Arc`s
    // /// to them and [`crate::AppState`] must be `Send + Sync`.
    // type SecretStore: Send + Sync;
    //
    // fn secrets(&self) -> Arc<Self::SecretStore>;

    /// Types must be `Send + Sync` because [`crate::AppState`] typically contains `Arc`s
    /// to them and [`crate::AppState`] must be `Send + Sync`.
    type Database: DatabaseExecutor + Send + Sync;

    fn database(&self) -> Arc<Self::Database>;
}
