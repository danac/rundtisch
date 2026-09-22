use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RandomError {
    Unavailable,
    Backend(String),
}

impl fmt::Display for RandomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RandomError::Unavailable => write!(f, "random source unavailable"),
            RandomError::Backend(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for RandomError {}

/// Cryptographically secure entropy. Synchronous on native and Workers.
pub trait RandomSource: Send + Sync {
    fn fill_bytes(&self, dest: &mut [u8]) -> Result<(), RandomError>;
}

impl dyn RandomSource + Send + Sync {
    pub fn bytes(&self, n: usize) -> Result<Vec<u8>, RandomError> {
        let mut dest = vec![0u8; n];
        self.fill_bytes(&mut dest)?;
        Ok(dest)
    }
}
