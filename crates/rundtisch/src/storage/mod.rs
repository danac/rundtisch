//! Object store: a lib-owned `storage_objects` table, content hashes, and a thin file port.
//!
//! Applications never see storage keys. They pass an optional prefix, stage bytes,
//! and commit the pending row inside their own transaction. Reads return committed
//! rows only. [`ObjectStore::cleanup`] is for a later cron executable, not for
//! request handlers.

mod config;
mod delivery;
mod digest;
mod disabled;
mod error;
mod id;
mod key;
mod port;
mod store;

pub mod entity;
pub mod migrations;

#[cfg(feature = "storage-fetch")]
mod fetch;
#[cfg(feature = "storage-http")]
mod http;
#[cfg(feature = "storage-local")]
mod local;
#[cfg(any(test, feature = "storage-memory"))]
mod memory;

pub use config::{StorageConfigError, local_root_from_env, storage_from_env};
pub use delivery::{CachePolicy, DeliverOptions, Delivery, Disposition};
pub use digest::Blake3Digest;
pub use disabled::DisabledFileStorage;
pub use error::{FileStorageError, StoreError};
pub use key::{ObjectKey, ObjectPrefix, extension_for_content_type};
pub use migrations::{MIGRATION_TABLE, Migrator, migrations};
pub use port::{
    ByteRange, ByteStream, FileStat, FileStorage, ListedFile, ObjectBody, UploadOptions,
    UploadTarget,
};
pub use store::{
    CleanupOptions, CleanupReport, FileRemoval, NewObject, ObjectInfo, ObjectStore, StagedObject,
    StoreLimits, Verified,
};

#[cfg(feature = "storage-fetch")]
pub use fetch::{FetchError, HttpUrlFetcher};
#[cfg(feature = "storage-local")]
pub use local::LocalFileStorage;
#[cfg(any(test, feature = "storage-memory"))]
pub use memory::MemoryFileStorage;

// Re-exported for sibling modules (`super::Delivery`, `super::LocalFile`, …).
#[cfg(any(test, feature = "storage-memory"))]
pub(crate) use delivery::ContentDelivery;
pub(crate) use delivery::{DeliveryMeta, LocalFile};
