pub mod db;

use std::sync::Arc;

/// Host platform an application is running on.
///
/// The platform owns long-lived resources (secrets, and later databases and
/// object storage) that are cloned into [`crate::AppState`] and held by the Axum router.
pub trait Platform: 'static {
    /// Types must be `Send + Sync` because [`crate::AppState`] typically contains `Arc`s
    /// to them and [`crate::AppState`] must be `Send + Sync`.
    type SecretStore: Send + Sync;

    fn secrets(&self) -> Arc<Self::SecretStore>;
}
