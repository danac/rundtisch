use std::path::PathBuf;
use std::sync::Arc;

use super::error::FileStorageError;
use super::port::FileStorage;

/// `STORAGE_LOCAL_ROOT`, then `DATA_DIR`, then `/data`.
pub fn local_root_from_env() -> PathBuf {
    for name in ["STORAGE_LOCAL_ROOT", "DATA_DIR"] {
        if let Ok(value) = std::env::var(name) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return PathBuf::from(trimmed);
            }
        }
    }
    PathBuf::from("/data")
}

/// Environment error for [`storage_from_env`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageConfigError {
    message: String,
}

impl StorageConfigError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for StorageConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for StorageConfigError {}

impl From<StorageConfigError> for FileStorageError {
    fn from(err: StorageConfigError) -> Self {
        FileStorageError::io(err)
    }
}

/// `STORAGE_BACKEND=local` (default) or `disabled`. `s3` is a later phase.
///
/// The local root is [`local_root_from_env`].
pub fn storage_from_env() -> Result<Arc<dyn FileStorage>, StorageConfigError> {
    let backend = std::env::var("STORAGE_BACKEND").unwrap_or_default();
    match backend.trim() {
        "" | "local" => open_local(),
        "disabled" => Ok(Arc::new(super::disabled::DisabledFileStorage)),
        #[cfg(feature = "storage-memory")]
        "memory" => Ok(Arc::new(super::memory::MemoryFileStorage::new())),
        "s3" => Err(StorageConfigError::new(
            "STORAGE_BACKEND=s3 is not available yet",
        )),
        other => Err(StorageConfigError::new(format!(
            "unknown STORAGE_BACKEND {other:?}"
        ))),
    }
}

fn open_local() -> Result<Arc<dyn FileStorage>, StorageConfigError> {
    #[cfg(feature = "storage-local")]
    {
        Ok(Arc::new(super::local::LocalFileStorage::from_env()?))
    }
    #[cfg(not(feature = "storage-local"))]
    {
        Err(StorageConfigError::new(
            "STORAGE_BACKEND=local requires the storage-local feature",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn root_prefers_storage_local_root_then_data_dir() {
        let _guard = ENV_LOCK.lock().unwrap();
        let previous_root = std::env::var("STORAGE_LOCAL_ROOT").ok();
        let previous_data = std::env::var("DATA_DIR").ok();
        unsafe {
            std::env::set_var("STORAGE_LOCAL_ROOT", " /var/objects ");
            std::env::set_var("DATA_DIR", "/var/data");
        }
        assert_eq!(local_root_from_env(), PathBuf::from("/var/objects"));
        unsafe {
            std::env::remove_var("STORAGE_LOCAL_ROOT");
        }
        assert_eq!(local_root_from_env(), PathBuf::from("/var/data"));
        unsafe {
            std::env::set_var("DATA_DIR", "  ");
        }
        assert_eq!(local_root_from_env(), PathBuf::from("/data"));
        restore("STORAGE_LOCAL_ROOT", previous_root);
        restore("DATA_DIR", previous_data);
    }

    fn restore(name: &str, value: Option<String>) {
        unsafe {
            match value {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
    }

    #[test]
    fn unknown_backend_is_rejected() {
        let _guard = ENV_LOCK.lock().unwrap();
        let previous = std::env::var("STORAGE_BACKEND").ok();
        unsafe {
            std::env::set_var("STORAGE_BACKEND", "s3");
        }
        let err = match storage_from_env() {
            Err(err) => err,
            Ok(_) => panic!("s3 backend should be rejected"),
        };
        assert!(err.to_string().contains("not available"));
        unsafe {
            std::env::set_var("STORAGE_BACKEND", "nope");
        }
        let err = match storage_from_env() {
            Err(err) => err,
            Ok(_) => panic!("unknown backend should be rejected"),
        };
        assert!(err.to_string().contains("unknown"));
        restore("STORAGE_BACKEND", previous);
    }
}
