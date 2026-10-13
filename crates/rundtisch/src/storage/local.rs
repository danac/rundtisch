use std::path::{Component, Path, PathBuf};
#[cfg(test)]
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

use super::error::FileStorageError;
use super::key::{ObjectKey, ObjectPrefix};
use super::port::{
    ByteRange, ByteStream, FileStat, FileStorage, ListedFile, ObjectBody, UploadOptions,
    UploadTarget, matches_prefix, slice_range, system_time_to_offset,
};
use super::{DeliverOptions, Delivery, DeliveryMeta, LocalFile};

/// Files on a local directory. Each write goes to `{key}.partial`, is synced, then renamed.
///
/// The root is `STORAGE_LOCAL_ROOT`, then `DATA_DIR`, then `/data`.
#[derive(Clone, Debug)]
pub struct LocalFileStorage {
    root: PathBuf,
}

impl LocalFileStorage {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn from_env() -> Result<Self, super::config::StorageConfigError> {
        Ok(Self::new(super::config::local_root_from_env()))
    }

    fn resolve(&self, key: &ObjectKey) -> Result<PathBuf, FileStorageError> {
        let mut path = self.root.clone();
        for segment in key.as_str().split('/') {
            if segment.is_empty() || segment == "." || segment == ".." {
                return Err(FileStorageError::InvalidKey(key.as_str().to_owned()));
            }
            path.push(segment);
        }
        Ok(path)
    }
}

struct TempGuard {
    path: PathBuf,
    armed: bool,
}

impl TempGuard {
    fn arm(path: PathBuf) -> Self {
        Self { path, armed: true }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for TempGuard {
    fn drop(&mut self) {
        if self.armed {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

fn temp_path(final_path: &Path) -> PathBuf {
    let mut name = final_path.file_name().unwrap_or_default().to_os_string();
    name.push(".partial");
    final_path.with_file_name(name)
}

fn relative_key(root: &Path, path: &Path) -> Option<ObjectKey> {
    let rel = path.strip_prefix(root).ok()?;
    let mut parts = Vec::new();
    for component in rel.components() {
        match component {
            Component::Normal(segment) => parts.push(segment.to_str()?.to_owned()),
            _ => return None,
        }
    }
    if parts.is_empty() {
        return None;
    }
    ObjectKey::parse(&parts.join("/")).ok()
}

#[async_trait::async_trait]
impl FileStorage for LocalFileStorage {
    async fn write_stream(
        &self,
        key: &ObjectKey,
        mut body: ByteStream,
        max_len: u64,
    ) -> Result<u64, FileStorageError> {
        let dest = self.resolve(key)?;
        if tokio::fs::try_exists(&dest)
            .await
            .map_err(FileStorageError::io)?
        {
            return Err(FileStorageError::AlreadyExists);
        }
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(FileStorageError::io)?;
        }
        let tmp = temp_path(&dest);
        let _ = tokio::fs::remove_file(&tmp).await;
        let mut file = tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
            .await
            .map_err(FileStorageError::io)?;
        let mut guard = TempGuard::arm(tmp);
        let mut written = 0u64;
        while let Some(chunk) = body.next().await {
            let chunk = chunk.map_err(FileStorageError::io)?;
            let next = written.saturating_add(chunk.len() as u64);
            if next > max_len {
                return Err(FileStorageError::TooLarge { limit: max_len });
            }
            file.write_all(&chunk).await.map_err(FileStorageError::io)?;
            written = next;
        }
        file.sync_all().await.map_err(FileStorageError::io)?;
        drop(file);
        if tokio::fs::try_exists(&dest)
            .await
            .map_err(FileStorageError::io)?
        {
            return Err(FileStorageError::AlreadyExists);
        }
        tokio::fs::rename(&guard.path, &dest)
            .await
            .map_err(FileStorageError::io)?;
        guard.disarm();
        Ok(written)
    }

    async fn read(
        &self,
        key: &ObjectKey,
        range: Option<ByteRange>,
    ) -> Result<ObjectBody, FileStorageError> {
        let path = self.resolve(key)?;
        let mut file = match tokio::fs::File::open(&path).await {
            Ok(file) => file,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Err(FileStorageError::NotFound);
            }
            Err(err) => return Err(FileStorageError::io(err)),
        };
        let total = file.metadata().await.map_err(FileStorageError::io)?.len();
        let (start, end) = slice_range(total, range)?;
        let len =
            usize::try_from(end - start).map_err(|_| FileStorageError::RangeNotSatisfiable)?;
        if start > 0 {
            file.seek(std::io::SeekFrom::Start(start))
                .await
                .map_err(FileStorageError::io)?;
        }
        let mut buf = vec![0u8; len];
        file.read_exact(&mut buf)
            .await
            .map_err(FileStorageError::io)?;
        Ok(ObjectBody {
            data: bytes::Bytes::from(buf),
            total_size: total,
        })
    }

    async fn stat(&self, key: &ObjectKey) -> Result<FileStat, FileStorageError> {
        let path = self.resolve(key)?;
        let meta = match tokio::fs::metadata(&path).await {
            Ok(meta) if meta.is_file() => meta,
            Ok(_) => return Err(FileStorageError::NotFound),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Err(FileStorageError::NotFound);
            }
            Err(err) => return Err(FileStorageError::io(err)),
        };
        Ok(FileStat {
            size: meta.len(),
            modified: meta.modified().ok().and_then(system_time_to_offset),
        })
    }

    async fn remove(&self, key: &ObjectKey) -> Result<(), FileStorageError> {
        let path = self.resolve(key)?;
        match tokio::fs::remove_file(&path).await {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(FileStorageError::io(err)),
        }
    }

    async fn list(
        &self,
        prefix: Option<&ObjectPrefix>,
    ) -> Result<Vec<ListedFile>, FileStorageError> {
        if !tokio::fs::try_exists(&self.root)
            .await
            .map_err(FileStorageError::io)?
        {
            return Ok(Vec::new());
        }
        let mut dirs = vec![self.root.clone()];
        let mut out = Vec::new();
        while let Some(dir) = dirs.pop() {
            let mut entries = tokio::fs::read_dir(&dir)
                .await
                .map_err(FileStorageError::io)?;
            while let Some(entry) = entries.next_entry().await.map_err(FileStorageError::io)? {
                let file_type = entry.file_type().await.map_err(FileStorageError::io)?;
                if file_type.is_dir() {
                    dirs.push(entry.path());
                    continue;
                }
                if !file_type.is_file() {
                    continue;
                }
                let Some(key) = relative_key(&self.root, &entry.path()) else {
                    continue;
                };
                if !matches_prefix(&key, prefix) {
                    continue;
                }
                let meta = entry.metadata().await.map_err(FileStorageError::io)?;
                out.push(ListedFile {
                    temporary: key.is_temporary(),
                    size: meta.len(),
                    modified: meta.modified().ok().and_then(system_time_to_offset),
                    key,
                });
            }
        }
        Ok(out)
    }

    async fn deliver(
        &self,
        key: &ObjectKey,
        opts: &DeliverOptions,
    ) -> Result<Delivery, FileStorageError> {
        let path = self.resolve(key)?;
        if !tokio::fs::try_exists(&path)
            .await
            .map_err(FileStorageError::io)?
        {
            return Err(FileStorageError::NotFound);
        }
        Ok(Delivery::File(LocalFile {
            path,
            meta: DeliveryMeta::from_options(opts),
        }))
    }

    fn prepare_upload(
        &self,
        _key: &ObjectKey,
        _opts: &UploadOptions,
    ) -> Result<UploadTarget, FileStorageError> {
        Ok(UploadTarget::ThisEndpoint)
    }

    fn describe(&self) -> String {
        format!("local:{}", self.root.display())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::port::assert_adapter_conformance;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "rundtisch-local-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[tokio::test]
    async fn local_adapter_conformance() {
        let dir = temp_dir();
        assert_adapter_conformance(Arc::new(LocalFileStorage::new(&dir))).await;
        let listed = LocalFileStorage::new(&dir).list(None).await.unwrap();
        assert!(
            listed
                .iter()
                .all(|file| file.key.as_str() != "photos/example.txt"),
            "conformance removes the file it wrote"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn partial_file_is_removed_when_the_write_fails() {
        let dir = temp_dir();
        let storage = LocalFileStorage::new(&dir);
        let key = ObjectKey::parse("folder/too-big.bin").unwrap();
        let err = storage
            .write_stream(&key, ByteStream::from_bytes(&b"0123456789"[..]), 4)
            .await
            .unwrap_err();
        assert_eq!(err, FileStorageError::TooLarge { limit: 4 });
        assert!(!dir.join("folder/too-big.bin").exists());
        assert!(!dir.join("folder/too-big.bin.partial").exists());
        std::fs::remove_dir_all(&dir).ok();
    }
}
