use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures_core::Stream;
use time::OffsetDateTime;

use super::Delivery;
use super::error::FileStorageError;
use super::key::{ObjectKey, ObjectPrefix};

/// Inclusive byte range. `end: None` means through the last byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ByteRange {
    pub start: u64,
    pub end: Option<u64>,
}

/// Bytes read from an adapter. `data` is the requested slice; `total_size` is the whole object.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectBody {
    pub data: Bytes,
    pub total_size: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileStat {
    pub size: u64,
    pub modified: Option<OffsetDateTime>,
}

/// One file the orphan sweep can see, including a stale `*.partial` temp file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListedFile {
    pub key: ObjectKey,
    pub size: u64,
    pub modified: Option<OffsetDateTime>,
    pub temporary: bool,
}

/// Where a later client upload should send bytes. Local and memory stay on this server.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UploadTarget {
    ThisEndpoint,
}

#[derive(Clone, Debug)]
pub struct UploadOptions {
    pub content_type: String,
    pub content_length: Option<u64>,
    pub sha256: Option<[u8; 32]>,
}

/// A body the store can hash while an adapter writes it.
pub struct ByteStream {
    inner: Pin<Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Send>>,
}

impl ByteStream {
    pub fn from_bytes(bytes: impl Into<Bytes>) -> Self {
        Self::from_stream(OnceBytes {
            bytes: Some(bytes.into()),
        })
    }

    pub fn from_stream<S>(stream: S) -> Self
    where
        S: Stream<Item = Result<Bytes, std::io::Error>> + Send + 'static,
    {
        Self {
            inner: Box::pin(stream),
        }
    }

    pub async fn next(&mut self) -> Option<Result<Bytes, std::io::Error>> {
        std::future::poll_fn(|cx| Pin::new(&mut *self).poll_next(cx)).await
    }
}

impl Stream for ByteStream {
    type Item = Result<Bytes, std::io::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut().inner.as_mut().poll_next(cx)
    }
}

struct OnceBytes {
    bytes: Option<Bytes>,
}

impl Stream for OnceBytes {
    type Item = Result<Bytes, std::io::Error>;

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(self.get_mut().bytes.take().map(Ok))
    }
}

/// Thin storage port. Adapters know nothing about the database or about hashing.
#[async_trait::async_trait]
pub trait FileStorage: Send + Sync {
    /// Stream into `key` atomically. Fails when the key already exists.
    async fn write_stream(
        &self,
        key: &ObjectKey,
        body: ByteStream,
        max_len: u64,
    ) -> Result<u64, FileStorageError>;

    async fn read(
        &self,
        key: &ObjectKey,
        range: Option<ByteRange>,
    ) -> Result<ObjectBody, FileStorageError>;

    async fn stat(&self, key: &ObjectKey) -> Result<FileStat, FileStorageError>;

    /// Missing files are success. The database row is already gone by the time this runs.
    async fn remove(&self, key: &ObjectKey) -> Result<(), FileStorageError>;

    /// Every stored key, plus stale temp files, for the orphan sweep.
    async fn list(
        &self,
        prefix: Option<&ObjectPrefix>,
    ) -> Result<Vec<ListedFile>, FileStorageError>;

    /// Local files become [`Delivery::File`]. Memory becomes [`Delivery::Content`].
    async fn deliver(
        &self,
        key: &ObjectKey,
        opts: &super::DeliverOptions,
    ) -> Result<Delivery, FileStorageError>;

    /// Client upload target. Local and memory are this server; S3 is a later phase.
    fn prepare_upload(
        &self,
        key: &ObjectKey,
        opts: &UploadOptions,
    ) -> Result<UploadTarget, FileStorageError>;

    fn describe(&self) -> String;
}

pub(crate) fn matches_prefix(key: &ObjectKey, prefix: Option<&ObjectPrefix>) -> bool {
    match prefix {
        None => true,
        Some(prefix) => {
            let raw = prefix.as_str();
            key.as_str() == raw || key.as_str().starts_with(&format!("{raw}/"))
        }
    }
}

/// Resolve an inclusive range against `total`. An empty object has no satisfiable range
/// except a full read, which yields an empty body.
pub(crate) fn slice_range(
    total: u64,
    range: Option<ByteRange>,
) -> Result<(u64, u64), FileStorageError> {
    let Some(range) = range else {
        return Ok((0, total));
    };
    if range.start > total || (total == 0 && range.start > 0) {
        return Err(FileStorageError::RangeNotSatisfiable);
    }
    let end_exclusive = match range.end {
        Some(end) if end < range.start => return Err(FileStorageError::RangeNotSatisfiable),
        Some(end) => end.saturating_add(1).min(total),
        None => total,
    };
    if range.start > end_exclusive || (range.start == total && total > 0) {
        return Err(FileStorageError::RangeNotSatisfiable);
    }
    Ok((range.start, end_exclusive))
}

pub(crate) fn system_time_to_offset(time: std::time::SystemTime) -> Option<OffsetDateTime> {
    let dur = time
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .ok()?;
    OffsetDateTime::from_unix_timestamp_nanos(dur.as_nanos() as i128).ok()
}

#[cfg(test)]
pub(crate) async fn assert_adapter_conformance(storage: std::sync::Arc<dyn FileStorage>) {
    let key = ObjectKey::parse("photos/example.txt").unwrap();
    let body = b"hello storage";
    let written = storage
        .write_stream(&key, ByteStream::from_bytes(&body[..]), 1024)
        .await
        .unwrap();
    assert_eq!(written, body.len() as u64);

    let again = storage
        .write_stream(&key, ByteStream::from_bytes(&body[..]), 1024)
        .await
        .unwrap_err();
    assert_eq!(again, FileStorageError::AlreadyExists);

    let stat = storage.stat(&key).await.unwrap();
    assert_eq!(stat.size, body.len() as u64);

    let full = storage.read(&key, None).await.unwrap();
    assert_eq!(&full.data[..], &body[..]);
    assert_eq!(full.total_size, body.len() as u64);

    let slice = storage
        .read(
            &key,
            Some(ByteRange {
                start: 1,
                end: Some(3),
            }),
        )
        .await
        .unwrap();
    assert_eq!(&slice.data[..], b"ell");

    let unsatisfiable = storage
        .read(
            &key,
            Some(ByteRange {
                start: 100,
                end: None,
            }),
        )
        .await
        .unwrap_err();
    assert_eq!(unsatisfiable, FileStorageError::RangeNotSatisfiable);

    let too_big = ObjectKey::parse("photos/big.bin").unwrap();
    let err = storage
        .write_stream(&too_big, ByteStream::from_bytes(&b"abcdef"[..]), 4)
        .await
        .unwrap_err();
    assert_eq!(err, FileStorageError::TooLarge { limit: 4 });
    assert_eq!(
        storage.stat(&too_big).await.unwrap_err(),
        FileStorageError::NotFound
    );

    let listed = storage
        .list(Some(&ObjectPrefix::parse("photos").unwrap()))
        .await
        .unwrap();
    assert!(listed.iter().any(|file| file.key == key && !file.temporary));
    assert!(
        storage
            .list(Some(&ObjectPrefix::parse("other").unwrap()))
            .await
            .unwrap()
            .is_empty()
    );

    let upload = storage
        .prepare_upload(
            &key,
            &UploadOptions {
                content_type: "text/plain".into(),
                content_length: Some(body.len() as u64),
                sha256: None,
            },
        )
        .unwrap();
    assert_eq!(upload, UploadTarget::ThisEndpoint);
    assert!(!storage.describe().is_empty());

    storage.remove(&key).await.unwrap();
    storage.remove(&key).await.unwrap();
    assert_eq!(
        storage.stat(&key).await.unwrap_err(),
        FileStorageError::NotFound
    );
}
