use std::fmt;

/// Failure from a [`super::FileStorage`] adapter. Adapters do not see the database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileStorageError {
    NotFound,
    AlreadyExists,
    TooLarge { limit: u64 },
    RangeNotSatisfiable,
    InvalidKey(String),
    Disabled,
    Io(String),
}

impl fmt::Display for FileStorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileStorageError::NotFound => write!(f, "stored file not found"),
            FileStorageError::AlreadyExists => write!(f, "stored file already exists"),
            FileStorageError::TooLarge { limit } => {
                write!(f, "stored file exceeds {limit} bytes")
            }
            FileStorageError::RangeNotSatisfiable => write!(f, "byte range is not satisfiable"),
            FileStorageError::InvalidKey(key) => write!(f, "invalid storage key {key:?}"),
            FileStorageError::Disabled => write!(f, "object storage is disabled"),
            FileStorageError::Io(message) => write!(f, "object storage: {message}"),
        }
    }
}

impl std::error::Error for FileStorageError {}

impl FileStorageError {
    pub(crate) fn io(err: impl fmt::Display) -> Self {
        FileStorageError::Io(err.to_string())
    }
}

/// Failure from [`super::ObjectStore`]. The database stays authoritative.
#[derive(Debug)]
pub enum StoreError {
    NotFound,
    /// `commit` saw a row that is missing, already committed, or not staged.
    NotPending,
    AlreadyExists,
    TooLarge {
        limit: u64,
    },
    /// `restore` refused bytes whose BLAKE3 does not match the committed row.
    ChecksumMismatch,
    InvalidPrefix(String),
    InvalidInput(String),
    Storage(FileStorageError),
    Db(sea_orm::DbErr),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::NotFound => write!(f, "object not found"),
            StoreError::NotPending => write!(f, "object is not a pending staged upload"),
            StoreError::AlreadyExists => write!(f, "object file already exists"),
            StoreError::TooLarge { limit } => write!(f, "object exceeds {limit} bytes"),
            StoreError::ChecksumMismatch => write!(f, "object checksum does not match"),
            StoreError::InvalidPrefix(message) => write!(f, "invalid object prefix: {message}"),
            StoreError::InvalidInput(message) => write!(f, "invalid object: {message}"),
            StoreError::Storage(err) => write!(f, "{err}"),
            StoreError::Db(err) => write!(f, "database: {err}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<FileStorageError> for StoreError {
    fn from(err: FileStorageError) -> Self {
        match err {
            FileStorageError::NotFound => StoreError::NotFound,
            FileStorageError::AlreadyExists => StoreError::AlreadyExists,
            FileStorageError::TooLarge { limit } => StoreError::TooLarge { limit },
            other => StoreError::Storage(other),
        }
    }
}

impl From<sea_orm::DbErr> for StoreError {
    fn from(err: sea_orm::DbErr) -> Self {
        StoreError::Db(err)
    }
}
