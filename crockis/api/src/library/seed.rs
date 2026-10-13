#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};

use rundtisch::storage::{
    ByteStream, NewObject, ObjectPrefix, ObjectStore, StoreError, Verified,
    extension_for_content_type,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait};

use super::catalog::{CatalogCollection, MOCK_CAPTURED_AT, mock_catalog};
use super::entities::{collection, picture};
#[cfg(test)]
use super::inspect;
use super::inspect::{filename_with_extension, image_info};

const MAX_IMAGE_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Debug)]
pub enum SeedError {
    Db(sea_orm::DbErr),
    #[cfg_attr(not(feature = "seed"), allow(dead_code))]
    Http(String),
    Image(String),
    Storage(String),
}

impl std::fmt::Display for SeedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SeedError::Db(err) => write!(f, "database: {err}"),
            SeedError::Http(message) => write!(f, "download: {message}"),
            SeedError::Image(message) => write!(f, "image: {message}"),
            SeedError::Storage(message) => write!(f, "storage: {message}"),
        }
    }
}

impl std::error::Error for SeedError {}

impl From<sea_orm::DbErr> for SeedError {
    fn from(err: sea_orm::DbErr) -> Self {
        SeedError::Db(err)
    }
}

impl From<StoreError> for SeedError {
    fn from(err: StoreError) -> Self {
        match err {
            StoreError::Db(err) => SeedError::Db(err),
            StoreError::ChecksumMismatch => SeedError::Image("checksum mismatch".into()),
            other => SeedError::Storage(other.to_string()),
        }
    }
}

#[derive(Debug, Default)]
pub struct SeedReport {
    pub collections_inserted: usize,
    pub pictures_inserted: usize,
    pub pictures_present: usize,
    pub files_written: usize,
}

pub trait ImageSource: Send + Sync {
    fn fetch(
        &self,
        url: &str,
    ) -> impl std::future::Future<Output = Result<Vec<u8>, SeedError>> + Send;
}

/// HTTP catalog client. Only the `crockis-seed` binary enables this.
#[cfg(feature = "seed")]
pub struct HttpSource {
    inner: rundtisch::storage::HttpUrlFetcher,
}

#[cfg(feature = "seed")]
impl HttpSource {
    pub fn new() -> Result<Self, SeedError> {
        let inner = rundtisch::storage::HttpUrlFetcher::new()
            .map_err(|err| SeedError::Http(err.to_string()))?;
        Ok(Self { inner })
    }
}

#[cfg(feature = "seed")]
impl ImageSource for HttpSource {
    async fn fetch(&self, url: &str) -> Result<Vec<u8>, SeedError> {
        self.inner
            .fetch(url, MAX_IMAGE_BYTES)
            .await
            .map(|bytes| bytes.to_vec())
            .map_err(|err| SeedError::Http(err.to_string()))
    }
}

#[cfg(test)]
pub struct BytesSource {
    pub bytes: Vec<u8>,
    pub hits: AtomicUsize,
}

#[cfg(test)]
impl ImageSource for BytesSource {
    async fn fetch(&self, _url: &str) -> Result<Vec<u8>, SeedError> {
        self.hits.fetch_add(1, Ordering::SeqCst);
        Ok(self.bytes.clone())
    }
}

/// Download the mock library into the object store and upsert matching rows.
///
/// The collection `storage_folder` is the object key prefix. A picture whose
/// file matches its BLAKE3 digest is left in place. A digest mismatch fails
/// the seed. A missing file is downloaded again and restored only when the
/// bytes hash to the committed digest. Text fields and the collection cover
/// are refreshed from the catalog.
pub async fn seed_catalog<S: ImageSource>(
    store: &ObjectStore,
    catalog: &[CatalogCollection],
    source: &S,
) -> Result<SeedReport, SeedError> {
    let mut report = SeedReport::default();
    for collection_catalog in catalog {
        let (collection_id, storage_folder) =
            upsert_collection(store, collection_catalog, &mut report).await?;
        let prefix = ObjectPrefix::parse(&storage_folder.as_hyphenated().to_string())
            .map_err(SeedError::Storage)?;
        for photo in collection_catalog.photos {
            upsert_picture(store, source, collection_id, &prefix, photo, &mut report).await?;
        }
        let cover = picture::Entity::find()
            .filter(picture::Column::PublicId.eq(collection_catalog.cover_public_id))
            .one(store.db())
            .await?
            .ok_or_else(|| {
                SeedError::Image(format!(
                    "cover {} was not inserted",
                    collection_catalog.cover_public_id
                ))
            })?;
        if cover.collection_id != collection_id {
            return Err(SeedError::Image(format!(
                "cover {} belongs to another collection",
                collection_catalog.cover_public_id
            )));
        }
        let row = collection::Entity::find_by_id(collection_id)
            .one(store.db())
            .await?
            .ok_or_else(|| {
                SeedError::Db(sea_orm::DbErr::Custom("collection disappeared".into()))
            })?;
        let mut active: collection::ActiveModel = row.into();
        active.thumbnail_picture_id = Set(Some(cover.id));
        active.update(store.db()).await?;
    }
    Ok(report)
}

async fn upsert_collection(
    store: &ObjectStore,
    catalog: &CatalogCollection,
    report: &mut SeedReport,
) -> Result<(i64, uuid::Uuid), SeedError> {
    let db = store.db();
    if let Some(row) = collection::Entity::find()
        .filter(collection::Column::PublicId.eq(catalog.public_id))
        .one(db)
        .await?
    {
        if row.storage_folder != catalog.storage_folder {
            return Err(SeedError::Storage(format!(
                "collection {} storage folder {} does not match catalog {}",
                catalog.public_id, row.storage_folder, catalog.storage_folder
            )));
        }
        let storage_folder = row.storage_folder;
        let mut active: collection::ActiveModel = row.into();
        active.name = Set(catalog.name.to_owned());
        active.description = Set(catalog.description.to_owned());
        let updated = active.update(db).await?;
        return Ok((updated.id, storage_folder));
    }
    let inserted = collection::ActiveModel {
        public_id: Set(catalog.public_id),
        storage_folder: Set(catalog.storage_folder),
        name: Set(catalog.name.to_owned()),
        description: Set(catalog.description.to_owned()),
        created_at: Set(catalog.created_at),
        thumbnail_picture_id: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await?;
    report.collections_inserted += 1;
    Ok((inserted.id, inserted.storage_folder))
}

async fn upsert_picture<S: ImageSource>(
    store: &ObjectStore,
    source: &S,
    collection_id: i64,
    prefix: &ObjectPrefix,
    photo: &super::catalog::CatalogPhoto,
    report: &mut SeedReport,
) -> Result<(), SeedError> {
    if let Some(row) = picture::Entity::find()
        .filter(picture::Column::PublicId.eq(photo.public_id))
        .one(store.db())
        .await?
    {
        let object = store.get(row.object_id).await?;
        let mut width = row.width;
        let mut height = row.height;
        let extension = match store.verify(&object).await? {
            Verified::Match => {
                report.pictures_present += 1;
                display_extension(photo.original_filename, &object.content_type)
            }
            Verified::Mismatch { .. } => {
                return Err(SeedError::Image(format!(
                    "checksum mismatch for {}",
                    photo.public_id
                )));
            }
            Verified::Missing => {
                let bytes = source.fetch(photo.source_url).await?;
                let info = image_info(&bytes, photo.original_filename).map_err(SeedError::Image)?;
                store
                    .restore(&object, ByteStream::from_bytes(bytes))
                    .await?;
                width = info.width;
                height = info.height;
                report.files_written += 1;
                info.extension.to_owned()
            }
        };
        let filename = filename_with_extension(photo.original_filename, &extension);
        let txn = store.db().begin().await?;
        store
            .set_original_filename(&txn, object.id, Some(&filename))
            .await?;
        let mut active: picture::ActiveModel = row.into();
        active.captured_at = Set(MOCK_CAPTURED_AT);
        active.width = Set(width);
        active.height = Set(height);
        active.update(&txn).await?;
        txn.commit().await?;
        return Ok(());
    }

    let bytes = source.fetch(photo.source_url).await?;
    let info = image_info(&bytes, photo.original_filename).map_err(SeedError::Image)?;
    let filename = filename_with_extension(photo.original_filename, info.extension);
    let staged = store
        .stage(
            NewObject {
                prefix: Some(prefix),
                content_type: content_type_for_extension(info.extension),
                original_filename: Some(&filename),
                max_len: MAX_IMAGE_BYTES,
            },
            ByteStream::from_bytes(bytes),
        )
        .await?;
    let txn = store.db().begin().await?;
    let committed = store.commit(&txn, staged).await?;
    picture::ActiveModel {
        public_id: Set(photo.public_id),
        collection_id: Set(collection_id),
        width: Set(info.width),
        height: Set(info.height),
        captured_at: Set(MOCK_CAPTURED_AT),
        object_id: Set(committed.id),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    txn.commit().await?;
    report.pictures_inserted += 1;
    report.files_written += 1;
    Ok(())
}

fn content_type_for_extension(extension: &str) -> &'static str {
    match extension {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "heic" => "image/heic",
        "heif" => "image/heif",
        _ => "application/octet-stream",
    }
}

/// Keep a catalog `.jpeg` suffix. Every other type follows the stored content type.
fn display_extension(original_filename: &str, content_type: &str) -> String {
    if content_type == "image/jpeg" && original_filename.to_ascii_lowercase().ends_with(".jpeg") {
        return "jpeg".to_owned();
    }
    extension_for_content_type(content_type)
        .unwrap_or("bin")
        .to_owned()
}

#[cfg(feature = "seed")]
pub async fn seed_mock_library(store: &ObjectStore) -> Result<SeedReport, SeedError> {
    let source = HttpSource::new()?;
    seed_catalog(store, mock_catalog(), &source).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrate;
    use rundtisch::storage::entity::object;
    use rundtisch::storage::{Blake3Digest, LocalFileStorage};
    use sea_orm::ConnectOptions;
    use std::sync::Arc;

    async fn memory_db() -> sea_orm::DatabaseConnection {
        let mut opts = ConnectOptions::new("sqlite::memory:");
        opts.max_connections(1);
        sea_orm::Database::connect(opts).await.expect("sqlite")
    }

    fn temp_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "crockis-seed-{}-{}",
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
    async fn seed_is_idempotent_and_repairs_a_missing_file() {
        let db = memory_db().await;
        migrate(&db).await.unwrap();
        let dir = temp_dir();
        let store = ObjectStore::new(db.clone(), Arc::new(LocalFileStorage::new(&dir)));
        let source = BytesSource {
            bytes: inspect::PNG_1X1.to_vec(),
            hits: AtomicUsize::new(0),
        };
        let first = seed_catalog(&store, mock_catalog(), &source).await.unwrap();
        assert_eq!(first.collections_inserted, 4);
        assert_eq!(first.pictures_inserted, 34);
        assert_eq!(first.files_written, 34);
        assert_eq!(source.hits.load(Ordering::SeqCst), 34);

        let second = seed_catalog(&store, mock_catalog(), &source).await.unwrap();
        assert_eq!(second.collections_inserted, 0);
        assert_eq!(second.pictures_inserted, 0);
        assert_eq!(second.files_written, 0);
        assert_eq!(second.pictures_present, 34);
        assert_eq!(source.hits.load(Ordering::SeqCst), 34);

        let picture = picture::Entity::find()
            .filter(picture::Column::PublicId.eq(crate::library::catalog::SERENITY_SPA_FIRST_ID))
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        let stored = object::Entity::find_by_id(picture.object_id)
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        let folder = crate::library::catalog::SERENITY_SPA_STORAGE_FOLDER
            .as_hyphenated()
            .to_string();
        assert!(stored.storage_key.starts_with(&format!("{folder}/")));
        assert!(stored.storage_key.ends_with(".png"));
        assert_eq!(stored.content_type, "image/png");
        assert_eq!(stored.original_filename.as_deref(), Some("Stones.png"));
        assert_eq!(
            stored.blake3.as_deref(),
            Some(Blake3Digest::hash(inspect::PNG_1X1).as_bytes().as_slice())
        );
        let path = dir.join(&stored.storage_key);
        assert!(path.is_file());
        tokio::fs::write(&path, b"tampered").await.unwrap();
        let mismatch = seed_catalog(&store, mock_catalog(), &source)
            .await
            .unwrap_err();
        assert!(
            mismatch.to_string().contains("checksum mismatch"),
            "{mismatch}"
        );
        tokio::fs::remove_file(&path).await.unwrap();
        let third = seed_catalog(&store, mock_catalog(), &source).await.unwrap();
        assert_eq!(third.files_written, 1);
        assert_eq!(third.pictures_inserted, 0);
        assert!(path.is_file());
        assert_eq!(source.hits.load(Ordering::SeqCst), 35);

        let collection = collection::Entity::find()
            .filter(collection::Column::PublicId.eq(crate::library::catalog::SERENITY_SPA_ID))
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        let cover = picture::Entity::find_by_id(collection.thumbnail_picture_id.unwrap())
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            cover.public_id,
            crate::library::catalog::SERENITY_SPA_COVER_ID
        );
        assert_eq!(picture.width, 1);
        assert_eq!(picture.height, 1);
        assert_eq!(
            collection.storage_folder,
            crate::library::catalog::SERENITY_SPA_STORAGE_FOLDER
        );
        let repaired = object::Entity::find_by_id(picture.object_id)
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            repaired.blake3.as_deref(),
            Some(Blake3Digest::hash(inspect::PNG_1X1).as_bytes().as_slice())
        );
        assert_eq!(
            store
                .verify(&store.get(picture.object_id).await.unwrap())
                .await
                .unwrap(),
            Verified::Match
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
