use crate::traits::random::RandomSource;
use argon2::password_hash::{PasswordHasher as _, PasswordVerifier};
use argon2::{Algorithm, Argon2, Params, Version};
use std::sync::{Arc, OnceLock};
use zeroize::Zeroize;

pub const MIN_PASSWORD_LEN: usize = 15;
pub const MAX_PASSWORD_LEN: usize = 256;

/// Passwords of at least [`MIN_PASSWORD_LEN`] that still must be rejected.
const COMMON_PASSWORDS: &[&str] = &[
    "123456789012345",
    "1234567890123456",
    "12345678901234567",
    "passwordpassword",
    "passwordpassword1",
    "qwertyuiopasdfg",
    "qwertyuiopasdfgh",
    "iloveyouiloveyou",
    "letmeinletmein1",
    "adminadminadmin",
    "welcomewelcome1",
    "footballfootball",
    "monkeymonkey123",
    "dragondragon123",
    "baseballbaseball",
    "abc123abc123abc",
    "111111111111111",
    "000000000000000",
    "aaaaaaaaaaaaaaa",
    "password1234567",
    "password12345678",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PasswordError {
    TooShort,
    TooLong,
    Common,
}

impl std::fmt::Display for PasswordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PasswordError::TooShort | PasswordError::TooLong | PasswordError::Common => {
                write!(f, "invalid_password")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PasswordHashError {
    Unavailable,
    Backend(String),
}

impl std::fmt::Display for PasswordHashError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PasswordHashError::Unavailable => write!(f, "password hasher unavailable"),
            PasswordHashError::Backend(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for PasswordHashError {}

pub trait PasswordHasher: Send + Sync {
    fn hash(&self, password: &str) -> Result<String, PasswordHashError>;
    fn verify(&self, password: &str, password_hash: &str) -> Result<bool, PasswordHashError>;
}

/// OWASP Argon2id `m=19456,t=2,p=1`, unique 16-byte salt, keyed with `AUTH_HASH_PEPPER`.
pub struct Argon2idHasher {
    random: Arc<dyn RandomSource>,
    pepper: Vec<u8>,
}

impl Argon2idHasher {
    pub fn new(random: Arc<dyn RandomSource>, pepper: Vec<u8>) -> Self {
        Self { random, pepper }
    }

    fn argon(&self) -> Result<Argon2<'_>, PasswordHashError> {
        let params = Params::new(19456, 2, 1, None)
            .map_err(|err| PasswordHashError::Backend(err.to_string()))?;
        Argon2::new_with_secret(&self.pepper, Algorithm::Argon2id, Version::V0x13, params)
            .map_err(|err| PasswordHashError::Backend(err.to_string()))
    }
}

impl PasswordHasher for Argon2idHasher {
    fn hash(&self, password: &str) -> Result<String, PasswordHashError> {
        let mut salt_bytes = [0u8; 16];
        self.random
            .fill_bytes(&mut salt_bytes)
            .map_err(|err| PasswordHashError::Backend(err.to_string()))?;
        let mut password_bytes = password.as_bytes().to_vec();
        let hash = self
            .argon()?
            .hash_password_with_salt(&password_bytes, &salt_bytes)
            .map_err(|err| PasswordHashError::Backend(err.to_string()))?
            .to_string();
        password_bytes.zeroize();
        Ok(hash)
    }

    fn verify(&self, password: &str, password_hash: &str) -> Result<bool, PasswordHashError> {
        match self
            .argon()?
            .verify_password(password.as_bytes(), password_hash)
        {
            Ok(()) => Ok(true),
            Err(argon2::password_hash::Error::PasswordInvalid) => Ok(false),
            Err(err) => Err(PasswordHashError::Backend(err.to_string())),
        }
    }
}

static DUMMY_PHC: OnceLock<String> = OnceLock::new();

/// Constant-time-ish unknown-user path: verify against a dummy PHC hashed with the
/// same pepper (computed once per isolate).
pub fn dummy_verify(hasher: &dyn PasswordHasher, password: &str) {
    let dummy = DUMMY_PHC.get_or_init(|| {
        hasher
            .hash("timing-dummy-not-a-real-password")
            .unwrap_or_else(|_| "$argon2id$v=19$m=19456,t=2,p=1$dummy".into())
    });
    let _ = hasher.verify(password, dummy);
}

pub fn check_password_policy(password: &str) -> Result<(), PasswordError> {
    let chars = password.chars().count();
    if chars < MIN_PASSWORD_LEN {
        return Err(PasswordError::TooShort);
    }
    if chars > MAX_PASSWORD_LEN {
        return Err(PasswordError::TooLong);
    }
    let lowered = password.to_lowercase();
    if COMMON_PASSWORDS
        .iter()
        .any(|common| lowered == *common || password == *common)
    {
        return Err(PasswordError::Common);
    }
    Ok(())
}

/// Fast hasher for tests. Never use in production.
#[derive(Debug, Default, Clone, Copy)]
pub struct TestPasswordHasher;

impl PasswordHasher for TestPasswordHasher {
    fn hash(&self, password: &str) -> Result<String, PasswordHashError> {
        Ok(format!("test:{password}"))
    }

    fn verify(&self, password: &str, password_hash: &str) -> Result<bool, PasswordHashError> {
        Ok(password_hash == format!("test:{password}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::random::{OsRandom, ReplayRandom};

    #[test]
    fn policy_rejects_short_long_and_common() {
        assert_eq!(check_password_policy("short"), Err(PasswordError::TooShort));
        assert_eq!(
            check_password_policy(&"x".repeat(257)),
            Err(PasswordError::TooLong)
        );
        assert_eq!(
            check_password_policy("passwordpassword"),
            Err(PasswordError::Common)
        );
        assert!(check_password_policy("unique-passphrase-ok").is_ok());
    }

    #[test]
    fn test_hasher_round_trip() {
        let hasher = TestPasswordHasher;
        let hash = hasher.hash("unique-passphrase-ok").unwrap();
        assert!(hasher.verify("unique-passphrase-ok", &hash).unwrap());
        assert!(!hasher.verify("other-passphrase-ok1", &hash).unwrap());
    }

    #[test]
    fn argon2id_hashes_with_unique_salts() {
        let hasher = Argon2idHasher::new(
            Arc::new(OsRandom),
            b"cccccccccccccccccccccccccccccccc".to_vec(),
        );
        let a = hasher.hash("unique-passphrase-ok").unwrap();
        let b = hasher.hash("unique-passphrase-ok").unwrap();
        assert!(a.starts_with("$argon2id$"));
        assert_ne!(a, b);
        assert!(hasher.verify("unique-passphrase-ok", &a).unwrap());
        assert!(!hasher.verify("other-passphrase-ok1", &a).unwrap());
    }

    #[test]
    fn argon2id_uses_supplied_salt_bytes() {
        let hasher = Argon2idHasher::new(
            Arc::new(ReplayRandom::new(vec![7u8; 16])),
            b"cccccccccccccccccccccccccccccccc".to_vec(),
        );
        let a = hasher.hash("unique-passphrase-ok").unwrap();
        let hasher2 = Argon2idHasher::new(
            Arc::new(ReplayRandom::new(vec![7u8; 16])),
            b"cccccccccccccccccccccccccccccccc".to_vec(),
        );
        let b = hasher2.hash("unique-passphrase-ok").unwrap();
        assert_eq!(a, b);
    }
}
