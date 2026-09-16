use crate::traits::Platform;
use std::sync::Arc;

pub struct AppState<P: Platform> {
    // pub secrets: Arc<P::SecretStore>,
    pub database: Arc<P::Database>,
}

impl<P: Platform> Clone for AppState<P> {
    fn clone(&self) -> Self {
        AppState {
            database: self.database.clone(),
        }
    }
}

impl<P: Platform> AppState<P> {
    pub fn from_platform(platform: &P) -> AppState<P> {
        AppState {
            database: platform.database(),
        }
    }
}
