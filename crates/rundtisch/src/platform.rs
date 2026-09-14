use std::sync::Arc;

/// Host platform an application is running on.
///
/// The platform owns long-lived resources (secrets, and later databases and
/// object storage) that are cloned into [`AppState`] and held by the Axum router.
pub trait Platform: 'static {
    /// Types must be `Send + Sync` because [`AppState`] typically contains `Arc`s
    /// to them and [`AppState`] must be `Send + Sync`.
    type SecretStore: Send + Sync;

    fn secrets(&self) -> Arc<Self::SecretStore>;
}

pub struct AppState<P: Platform> {
    pub secrets: Arc<P::SecretStore>,
}

impl<P: Platform> Clone for AppState<P> {
    fn clone(&self) -> Self {
        AppState {
            secrets: self.secrets.clone(),
        }
    }
}

impl<P: Platform> AppState<P> {
    pub fn from_platform(platform: &P) -> AppState<P> {
        AppState {
            secrets: platform.secrets(),
        }
    }
}
