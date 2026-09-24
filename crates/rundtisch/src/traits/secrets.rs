use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretError {
    NotFound,
    Backend(String),
}

impl fmt::Display for SecretError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SecretError::NotFound => write!(f, "secret not found"),
            SecretError::Backend(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for SecretError {}

pub trait SecretStore: Send + Sync {
    fn get(&self, name: &str) -> Result<String, SecretError>;
}
