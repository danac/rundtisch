use super::error::FileStorageError;
use super::key::{ObjectKey, ObjectPrefix};
use super::port::{
    ByteRange, ByteStream, FileStat, FileStorage, ListedFile, ObjectBody, UploadOptions,
    UploadTarget,
};
use super::{DeliverOptions, Delivery};

/// Adapter that refuses every call. [`crate::AppState::new`] installs it so a
/// binary can enable the `storage` feature without configuring a backend.
#[derive(Clone, Debug, Default)]
pub struct DisabledFileStorage;

#[async_trait::async_trait]
impl FileStorage for DisabledFileStorage {
    async fn write_stream(
        &self,
        _key: &ObjectKey,
        _body: ByteStream,
        _max_len: u64,
    ) -> Result<u64, FileStorageError> {
        Err(FileStorageError::Disabled)
    }

    async fn read(
        &self,
        _key: &ObjectKey,
        _range: Option<ByteRange>,
    ) -> Result<ObjectBody, FileStorageError> {
        Err(FileStorageError::Disabled)
    }

    async fn stat(&self, _key: &ObjectKey) -> Result<FileStat, FileStorageError> {
        Err(FileStorageError::Disabled)
    }

    async fn remove(&self, _key: &ObjectKey) -> Result<(), FileStorageError> {
        Err(FileStorageError::Disabled)
    }

    async fn list(
        &self,
        _prefix: Option<&ObjectPrefix>,
    ) -> Result<Vec<ListedFile>, FileStorageError> {
        Err(FileStorageError::Disabled)
    }

    async fn deliver(
        &self,
        _key: &ObjectKey,
        _opts: &DeliverOptions,
    ) -> Result<Delivery, FileStorageError> {
        Err(FileStorageError::Disabled)
    }

    fn prepare_upload(
        &self,
        _key: &ObjectKey,
        _opts: &UploadOptions,
    ) -> Result<UploadTarget, FileStorageError> {
        Err(FileStorageError::Disabled)
    }

    fn describe(&self) -> String {
        "disabled".to_owned()
    }
}
