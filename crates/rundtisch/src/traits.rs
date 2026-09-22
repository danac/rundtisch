pub mod clock;
pub mod db;
pub mod random;
pub mod secrets;

use std::sync::Arc;

use crate::adapters::clock::SystemClock;
use crate::adapters::random::DefaultRandom;
use clock::Clock;
use db::DatabaseExecutor;
use random::RandomSource;
use secrets::SecretStore;

/// Host platform an application is running on.
///
/// The platform owns long-lived resources (databases, secrets) that are cloned
/// into [`crate::AppState`] and held by the Axum router. Clock and random have
/// defaults so native platforms only implement [`Self::Database`] and
/// [`Self::SecretStore`].
pub trait Platform: 'static {
    /// Types must be `Send + Sync` because [`crate::AppState`] typically contains `Arc`s
    /// to them and [`crate::AppState`] must be `Send + Sync`.
    type Database: DatabaseExecutor + Send + Sync;
    type SecretStore: SecretStore + Send + Sync;

    fn database(&self) -> Arc<Self::Database>;
    fn secrets(&self) -> Arc<Self::SecretStore>;

    fn random(&self) -> Arc<dyn RandomSource> {
        Arc::new(DefaultRandom)
    }

    fn clock(&self) -> Arc<dyn Clock> {
        Arc::new(SystemClock)
    }
}
