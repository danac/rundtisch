use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use bytes::Bytes;
use time::OffsetDateTime;

use super::error::FileStorageError;
use super::key::{ObjectKey, ObjectPrefix};
use super::port::{
    ByteRange, ByteStream, FileStat, FileStorage, ListedFile, ObjectBody, UploadOptions,
    UploadTarget, matches_prefix, slice_range,
};
use super::{ContentDelivery, DeliverOptions, Delivery, DeliveryMeta};

#[derive(Clone)]
struct MemFile {
    bytes: Bytes,
    modified: OffsetDateTime,
}

struct MemoryInner {
    files: HashMap<String, MemFile>,
    temps: HashMap<String, MemFile>,
    fail_removes: u32,
}

/// In-memory adapter for tests. Writes become visible only after the stream is consumed.
#[derive(Clone)]
pub struct MemoryFileStorage {
    inner: Arc<Mutex<MemoryInner>>,
}

impl Default for MemoryFileStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryFileStorage {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(MemoryInner {
                files: HashMap::new(),
                temps: HashMap::new(),
                fail_removes: 0,
            })),
        }
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, MemoryInner>, FileStorageError> {
        self.inner
            .lock()
            .map_err(|_| FileStorageError::io("memory storage lock poisoned"))
    }

    #[cfg(test)]
    pub fn insert_file(&self, key: &str, bytes: &[u8], modified: OffsetDateTime) {
        let key = ObjectKey::parse(key).expect("test key");
        self.lock().unwrap().files.insert(
            key.as_str().to_owned(),
            MemFile {
                bytes: Bytes::copy_from_slice(bytes),
                modified,
            },
        );
    }

    #[cfg(test)]
    pub fn insert_temp(&self, key: &str, bytes: &[u8], modified: OffsetDateTime) {
        assert!(key.ends_with(".partial"), "temp keys end in .partial");
        let key = ObjectKey::parse(key).expect("test temp key");
        self.lock().unwrap().temps.insert(
            key.as_str().to_owned(),
            MemFile {
                bytes: Bytes::copy_from_slice(bytes),
                modified,
            },
        );
    }

    #[cfg(test)]
    pub fn set_modified(&self, key: &str, modified: OffsetDateTime) {
        let mut guard = self.lock().unwrap();
        if let Some(file) = guard.files.get_mut(key) {
            file.modified = modified;
        }
    }

    #[cfg(test)]
    pub fn fail_next_remove(&self) {
        self.lock().unwrap().fail_removes = 1;
    }

    #[cfg(test)]
    pub fn contains(&self, key: &str) -> bool {
        self.lock().unwrap().files.contains_key(key)
    }
}

impl std::fmt::Debug for MemoryFileStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MemoryFileStorage")
    }
}

async fn collect(body: &mut ByteStream, max_len: u64) -> Result<Bytes, FileStorageError> {
    let mut buf = Vec::new();
    while let Some(chunk) = body.next().await {
        let chunk = chunk.map_err(FileStorageError::io)?;
        let next = buf.len() as u64 + chunk.len() as u64;
        if next > max_len {
            return Err(FileStorageError::TooLarge { limit: max_len });
        }
        buf.extend_from_slice(&chunk);
    }
    Ok(Bytes::from(buf))
}

#[async_trait::async_trait]
impl FileStorage for MemoryFileStorage {
    async fn write_stream(
        &self,
        key: &ObjectKey,
        mut body: ByteStream,
        max_len: u64,
    ) -> Result<u64, FileStorageError> {
        let bytes = collect(&mut body, max_len).await?;
        let mut guard = self.lock()?;
        if guard.files.contains_key(key.as_str()) {
            return Err(FileStorageError::AlreadyExists);
        }
        let len = bytes.len() as u64;
        guard.files.insert(
            key.as_str().to_owned(),
            MemFile {
                bytes,
                modified: OffsetDateTime::now_utc(),
            },
        );
        Ok(len)
    }

    async fn read(
        &self,
        key: &ObjectKey,
        range: Option<ByteRange>,
    ) -> Result<ObjectBody, FileStorageError> {
        let guard = self.lock()?;
        let file = guard
            .files
            .get(key.as_str())
            .ok_or(FileStorageError::NotFound)?;
        let total = file.bytes.len() as u64;
        let (start, end) = slice_range(total, range)?;
        let start_usize =
            usize::try_from(start).map_err(|_| FileStorageError::RangeNotSatisfiable)?;
        let end_usize = usize::try_from(end).map_err(|_| FileStorageError::RangeNotSatisfiable)?;
        Ok(ObjectBody {
            data: file.bytes.slice(start_usize..end_usize),
            total_size: total,
        })
    }

    async fn stat(&self, key: &ObjectKey) -> Result<FileStat, FileStorageError> {
        let guard = self.lock()?;
        let file = guard
            .files
            .get(key.as_str())
            .ok_or(FileStorageError::NotFound)?;
        Ok(FileStat {
            size: file.bytes.len() as u64,
            modified: Some(file.modified),
        })
    }

    async fn remove(&self, key: &ObjectKey) -> Result<(), FileStorageError> {
        let mut guard = self.lock()?;
        if guard.fail_removes > 0 {
            guard.fail_removes -= 1;
            return Err(FileStorageError::io("simulated remove failure"));
        }
        guard.files.remove(key.as_str());
        guard.temps.remove(key.as_str());
        Ok(())
    }

    async fn list(
        &self,
        prefix: Option<&ObjectPrefix>,
    ) -> Result<Vec<ListedFile>, FileStorageError> {
        let guard = self.lock()?;
        let mut out = Vec::new();
        for (key, file) in &guard.files {
            let key = ObjectKey::parse(key).map_err(FileStorageError::InvalidKey)?;
            if matches_prefix(&key, prefix) {
                out.push(ListedFile {
                    temporary: key.is_temporary(),
                    key,
                    size: file.bytes.len() as u64,
                    modified: Some(file.modified),
                });
            }
        }
        for (key, file) in &guard.temps {
            let key = ObjectKey::parse(key).map_err(FileStorageError::InvalidKey)?;
            if matches_prefix(&key, prefix) {
                out.push(ListedFile {
                    temporary: true,
                    key,
                    size: file.bytes.len() as u64,
                    modified: Some(file.modified),
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
        let body = self.read(key, None).await?;
        Ok(Delivery::Content(ContentDelivery {
            bytes: body.data,
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
        "memory".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::port::assert_adapter_conformance;

    #[tokio::test]
    async fn memory_adapter_conformance() {
        assert_adapter_conformance(Arc::new(MemoryFileStorage::new())).await;
    }
}
