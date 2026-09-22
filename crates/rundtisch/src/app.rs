use crate::auth::password::{Argon2idHasher, PasswordHasher};
use crate::traits::Platform;
use crate::traits::clock::Clock;
use crate::traits::random::RandomSource;
use crate::traits::secrets::{HASH_PEPPER, SecretStore};
use std::sync::Arc;

pub struct AppState<P: Platform> {
    pub database: Arc<P::Database>,
    pub secrets: Arc<P::SecretStore>,
    pub random: Arc<dyn RandomSource>,
    pub clock: Arc<dyn Clock>,
    pub password_hasher: Arc<dyn PasswordHasher>,
}

impl<P: Platform> Clone for AppState<P> {
    fn clone(&self) -> Self {
        AppState {
            database: self.database.clone(),
            secrets: self.secrets.clone(),
            random: self.random.clone(),
            clock: self.clock.clone(),
            password_hasher: self.password_hasher.clone(),
        }
    }
}

impl<P: Platform> AppState<P> {
    pub fn from_platform(platform: &P) -> AppState<P> {
        let secrets = platform.secrets();
        let random = platform.random();
        let password_hasher = hasher_from_secrets(&*secrets, random.clone());
        AppState {
            database: platform.database(),
            secrets,
            random,
            clock: platform.clock(),
            password_hasher,
        }
    }
}

fn hasher_from_secrets(
    secrets: &dyn SecretStore,
    random: Arc<dyn RandomSource>,
) -> Arc<dyn PasswordHasher> {
    match secrets.get(HASH_PEPPER) {
        Ok(pepper) if pepper.len() == 32 => {
            Arc::new(Argon2idHasher::new(random, pepper.into_bytes()))
        }
        _ => Arc::new(UnavailableHasher),
    }
}

struct UnavailableHasher;

impl PasswordHasher for UnavailableHasher {
    fn hash(&self, _password: &str) -> Result<String, crate::auth::password::PasswordHashError> {
        Err(crate::auth::password::PasswordHashError::Unavailable)
    }

    fn verify(
        &self,
        _password: &str,
        _password_hash: &str,
    ) -> Result<bool, crate::auth::password::PasswordHashError> {
        Err(crate::auth::password::PasswordHashError::Unavailable)
    }
}
