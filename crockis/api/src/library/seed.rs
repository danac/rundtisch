use std::path::Path;
#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};

use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use super::catalog::{CatalogCollection, MOCK_CAPTURED_AT, mock_catalog};
use super::entities::{collection, picture};
use super::store::{self, checksum_blake3, data_dir, image_info, seed_enabled_from, storage_path};

const MAX_IMAGE_BYTES: usize = 32 * 1024 * 1024;

#[derive(Debug)]
pub enum SeedError {
    Db(sea_orm::DbErr),
    Io(std::io::Error),
    Http(String),
    Image(String),
    DataDir(String),
}

impl std::fmt::Display for SeedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SeedError::Db(err) => write!(f, "database: {err}"),
            SeedError::Io(err) => write!(f, "filesystem: {err}"),
            SeedError::Http(message) => write!(f, "download: {message}"),
            SeedError::Image(message) => write!(f, "image: {message}"),
            SeedError::DataDir(message) => write!(f, "data directory: {message}"),
        }
    }
}

impl std::error::Error for SeedError {}

impl From<sea_orm::DbErr> for SeedError {
    fn from(err: sea_orm::DbErr) -> Self {
        SeedError::Db(err)
    }
}

impl From<std::io::Error> for SeedError {
    fn from(err: std::io::Error) -> Self {
        SeedError::Io(err)
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

pub struct HttpSource {
    client: reqwest::Client,
}

impl HttpSource {
    pub fn new() -> Result<Self, SeedError> {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (compatible; CrockisSeed/0.1)")
            .timeout(std::time::Duration::from_secs(60))
            .redirect(reqwest::redirect::Policy::limited(8))
            .build()
            .map_err(|err| SeedError::Http(err.to_string()))?;
        Ok(Self { client })
    }
}

impl ImageSource for HttpSource {
    async fn fetch(&self, url: &str) -> Result<Vec<u8>, SeedError> {
        let response = self
            .client
            .get(url)
            .header(
                reqwest::header::ACCEPT,
                "image/jpeg,image/png,image/webp,image/gif,*/*;q=0.8",
            )
            .send()
            .await
            .map_err(|err| SeedError::Http(format!("{url}: {err}")))?;
        if let Some(len) = response.content_length() {
            if len > MAX_IMAGE_BYTES as u64 {
                return Err(SeedError::Http(format!(
                    "{url}: image is larger than {MAX_IMAGE_BYTES} bytes"
                )));
            }
        }
        let status = response.status();
        if !status.is_success() {
            return Err(SeedError::Http(format!("{url}: HTTP {status}")));
        }
        let bytes = response
            .bytes()
            .await
            .map_err(|err| SeedError::Http(format!("{url}: {err}")))?;
        if bytes.is_empty() {
            return Err(SeedError::Http(format!("{url}: empty response")));
        }
        if bytes.len() > MAX_IMAGE_BYTES {
            return Err(SeedError::Http(format!(
                "{url}: image is larger than {MAX_IMAGE_BYTES} bytes"
            )));
        }
        Ok(bytes.to_vec())
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

/// Download the mock library into `data_dir` and upsert matching rows.
///
/// Each collection's files live in `{data_dir}/{storage_folder}/`. A picture
/// that already has its storage file is left in place when the file's BLAKE3
/// digest matches `checksum_blake3`. A digest mismatch fails the seed. A
/// picture row whose file is missing is downloaded again into that same file
/// name. Text fields and the collection cover are refreshed from the catalog.
pub async fn seed_catalog<S: ImageSource>(
    db: &DatabaseConnection,
    data_dir: &Path,
    catalog: &[CatalogCollection],
    source: &S,
) -> Result<SeedReport, SeedError> {
    tokio::fs::create_dir_all(data_dir).await.map_err(|err| {
        SeedError::DataDir(format!("cannot create {}: {err}", data_dir.display()))
    })?;
    let mut report = SeedReport::default();
    for collection_catalog in catalog {
        let (collection_id, storage_folder) =
            upsert_collection(db, collection_catalog, &mut report).await?;
        let folder = data_dir.join(storage_folder.as_hyphenated().to_string());
        tokio::fs::create_dir_all(&folder).await.map_err(|err| {
            SeedError::DataDir(format!("cannot create {}: {err}", folder.display()))
        })?;
        for photo in collection_catalog.photos {
            upsert_picture(
                db,
                data_dir,
                source,
                collection_id,
                storage_folder,
                photo,
                &mut report,
            )
            .await?;
        }
        let cover = picture::Entity::find()
            .filter(picture::Column::PublicId.eq(collection_catalog.cover_public_id))
            .one(db)
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
            .one(db)
            .await?
            .ok_or_else(|| {
                SeedError::Db(sea_orm::DbErr::Custom("collection disappeared".into()))
            })?;
        let mut active: collection::ActiveModel = row.into();
        active.thumbnail_picture_id = Set(Some(cover.id));
        active.update(db).await?;
    }
    Ok(report)
}

async fn upsert_collection(
    db: &DatabaseConnection,
    catalog: &CatalogCollection,
    report: &mut SeedReport,
) -> Result<(i64, uuid::Uuid), SeedError> {
    if let Some(row) = collection::Entity::find()
        .filter(collection::Column::PublicId.eq(catalog.public_id))
        .one(db)
        .await?
    {
        if row.storage_folder != catalog.storage_folder {
            return Err(SeedError::DataDir(format!(
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
    db: &DatabaseConnection,
    data_dir: &Path,
    source: &S,
    collection_id: i64,
    storage_folder: uuid::Uuid,
    photo: &super::catalog::CatalogPhoto,
    report: &mut SeedReport,
) -> Result<(), SeedError> {
    if let Some(row) = picture::Entity::find()
        .filter(picture::Column::PublicId.eq(photo.public_id))
        .one(db)
        .await?
    {
        let path =
            storage_path(data_dir, storage_folder, &row.storage_filename).ok_or_else(|| {
                SeedError::DataDir(format!("unsafe storage filename {}", row.storage_filename))
            })?;
        let mut width = row.width;
        let mut height = row.height;
        let checksum = if file_ready(&path) {
            let bytes = tokio::fs::read(&path).await?;
            let checksum = checksum_blake3(&bytes);
            if row.checksum_blake3.as_slice() != checksum.as_slice() {
                return Err(SeedError::Image(format!(
                    "checksum mismatch for {}",
                    row.storage_filename
                )));
            }
            report.pictures_present += 1;
            checksum
        } else {
            let bytes = source.fetch(photo.source_url).await?;
            let info = image_info(&bytes, photo.original_filename).map_err(SeedError::Image)?;
            write_file(&path, &bytes).await?;
            width = info.width;
            height = info.height;
            report.files_written += 1;
            checksum_blake3(&bytes)
        };
        let extension = store::storage_extension(&row.storage_filename).to_owned();
        let mut active: picture::ActiveModel = row.into();
        active.captured_at = Set(MOCK_CAPTURED_AT);
        active.original_filename = Set(store::filename_with_extension(
            photo.original_filename,
            &extension,
        ));
        active.width = Set(width);
        active.height = Set(height);
        active.checksum_blake3 = Set(checksum.to_vec());
        active.update(db).await?;
        return Ok(());
    }

    let bytes = source.fetch(photo.source_url).await?;
    let info = image_info(&bytes, photo.original_filename).map_err(SeedError::Image)?;
    let checksum = checksum_blake3(&bytes);
    let storage_filename = format!("{}.{}", Uuid::new_v4(), info.extension);
    let path = storage_path(data_dir, storage_folder, &storage_filename)
        .ok_or_else(|| SeedError::DataDir(format!("unsafe storage filename {storage_filename}")))?;
    write_file(&path, &bytes).await?;
    picture::ActiveModel {
        public_id: Set(photo.public_id),
        collection_id: Set(collection_id),
        width: Set(info.width),
        height: Set(info.height),
        original_filename: Set(store::filename_with_extension(
            photo.original_filename,
            info.extension,
        )),
        captured_at: Set(MOCK_CAPTURED_AT),
        storage_filename: Set(storage_filename),
        checksum_blake3: Set(checksum.to_vec()),
        ..Default::default()
    }
    .insert(db)
    .await?;
    report.pictures_inserted += 1;
    report.files_written += 1;
    Ok(())
}

fn file_ready(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|meta| meta.is_file() && meta.len() > 0)
        .unwrap_or(false)
}

async fn write_file(path: &Path, bytes: &[u8]) -> Result<(), SeedError> {
    let tmp = path.with_extension("partial");
    tokio::fs::write(&tmp, bytes).await?;
    tokio::fs::rename(&tmp, path).await?;
    Ok(())
}

pub async fn seed_mock_library(
    db: &DatabaseConnection,
    data_dir: &Path,
) -> Result<SeedReport, SeedError> {
    let source = HttpSource::new()?;
    seed_catalog(db, data_dir, mock_catalog(), &source).await
}

pub async fn seed_if_enabled(db: &DatabaseConnection) -> Result<(), SeedError> {
    if !seed_enabled_from(std::env::var("CROCKIS_SEED").ok().as_deref()) {
        eprintln!("migrate: CROCKIS_SEED disabled, skipping library seed");
        return Ok(());
    }
    let dir = data_dir();
    eprintln!("migrate: seeding library into {}", dir.display());
    let report = seed_mock_library(db, &dir).await?;
    eprintln!(
        "migrate: library seed collections_inserted={} pictures_inserted={} files_written={} pictures_already_stored={}",
        report.collections_inserted,
        report.pictures_inserted,
        report.files_written,
        report.pictures_present
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrator::LibraryMigrator;
    use sea_orm::ConnectOptions;
    use sea_orm_migration::MigratorTrait;

    async fn memory_db() -> DatabaseConnection {
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
        LibraryMigrator::up(&db, None).await.unwrap();
        let dir = temp_dir();
        let source = BytesSource {
            bytes: store::PNG_1X1.to_vec(),
            hits: AtomicUsize::new(0),
        };
        let first = seed_catalog(&db, &dir, mock_catalog(), &source)
            .await
            .unwrap();
        assert_eq!(first.collections_inserted, 4);
        assert_eq!(first.pictures_inserted, 34);
        assert_eq!(first.files_written, 34);
        assert_eq!(source.hits.load(Ordering::SeqCst), 34);

        let second = seed_catalog(&db, &dir, mock_catalog(), &source)
            .await
            .unwrap();
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
        let path = storage_path(
            &dir,
            crate::library::catalog::SERENITY_SPA_STORAGE_FOLDER,
            &picture.storage_filename,
        )
        .unwrap();
        assert_eq!(
            picture.checksum_blake3.as_slice(),
            checksum_blake3(store::PNG_1X1).as_slice()
        );
        assert!(path.is_file());
        tokio::fs::write(&path, b"tampered").await.unwrap();
        let mismatch = seed_catalog(&db, &dir, mock_catalog(), &source)
            .await
            .unwrap_err();
        assert!(
            mismatch.to_string().contains("checksum mismatch"),
            "{mismatch}"
        );
        tokio::fs::remove_file(&path).await.unwrap();
        let third = seed_catalog(&db, &dir, mock_catalog(), &source)
            .await
            .unwrap();
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
        assert!(picture.storage_filename.ends_with(".png"));
        assert_eq!(
            collection.storage_folder,
            crate::library::catalog::SERENITY_SPA_STORAGE_FOLDER
        );
        let repaired = picture::Entity::find_by_id(picture.id)
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            repaired.checksum_blake3.as_slice(),
            checksum_blake3(store::PNG_1X1).as_slice()
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn http_source_downloads_from_a_local_server() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let app = axum::Router::new().route(
            "/tiny.png",
            axum::routing::get(|| async {
                (
                    [(axum::http::header::CONTENT_TYPE, "image/png")],
                    store::PNG_1X1.to_vec(),
                )
            }),
        );
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let source = HttpSource::new().unwrap();
        let bytes = source
            .fetch(&format!("http://127.0.0.1:{port}/tiny.png"))
            .await
            .unwrap();
        assert_eq!(bytes, store::PNG_1X1);
    }
}
