pub mod db;
use std::sync::Arc;

// Platform must own its members because it is used to derive objects that stay long-lived
// in the Axum router
pub trait Platform: 'static {
    // Types must be Send + Sync because AppState typically contains Arcs to them and AppState must by Send+Sync
    type SecretStore: Send + Sync;

    fn secrets(&self) -> Arc<Self::SecretStore>;
}
