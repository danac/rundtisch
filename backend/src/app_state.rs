use crate::traits::Platform;
use std::sync::Arc;

pub struct AppState<P: Platform> {
    pub secrets: Arc<P::SecretStore>,
}

impl<P: Platform> Clone for AppState<P> {
    fn clone(&self) -> Self {
        AppState{
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
