use std::collections::{HashMap, HashSet};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;

use bytes::Bytes;
use futures_core::Stream;
use sea_orm::ActiveValue::Set;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, TransactionTrait,
    sea_query::Expr,
};
use sha2::Digest;
use time::OffsetDateTime;
use uuid::Uuid;

use super::digest::Blake3Digest;
use super::entity::object::{self, ObjectStatus};
use super::error::{FileStorageError, StoreError};
use super::id::new_uuid_v4;
use super::key::{ObjectKey, ObjectPrefix, generate_key};
use super::port::{ByteRange, ByteStream, FileStorage, ObjectBody};
use super::{DeliverOptions, Delivery};

const DEFAULT_MAX_BYTES: u64 = 512 * 1024 * 1024;

/// Hard cap applied on top of each call's `max_len`.
#[derive(Clone, Copy, Debug)]
pub struct StoreLimits {
    pub max_bytes: u64,
}

impl Default for StoreLimits {
    fn default() -> Self {
        Self {
            max_bytes: DEFAULT_MAX_BYTES,
        }
    }
}

/// What the application may choose when staging an object. The key is not one of these.
pub struct NewObject<'a> {
    pub prefix: Option<&'a ObjectPrefix>,
    pub content_type: &'a str,
    pub original_filename: Option<&'a str>,
    pub max_len: u64,
}

/// A committed row. Safe to hand to application code: it has no storage key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectInfo {
    pub id: i64,
    pub public_id: Uuid,
    pub size: u64,
    pub content_type: String,
    pub blake3: Blake3Digest,
    pub sha256: [u8; 32],
    pub original_filename: Option<String>,
    pub created_at: OffsetDateTime,
    pub committed_at: OffsetDateTime,
}

/// Handle returned by [`ObjectStore::stage`]. Pass it to [`ObjectStore::commit`]
/// inside the application's transaction.
#[derive(Debug)]
pub struct StagedObject {
    id: i64,
    public_id: Uuid,
    size: u64,
    content_type: String,
    blake3: Blake3Digest,
    sha256: [u8; 32],
    original_filename: Option<String>,
    created_at: OffsetDateTime,
}

impl StagedObject {
    pub fn id(&self) -> i64 {
        self.id
    }

    pub fn public_id(&self) -> Uuid {
        self.public_id
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn content_type(&self) -> &str {
        &self.content_type
    }

    pub fn blake3(&self) -> Blake3Digest {
        self.blake3
    }

    pub fn sha256(&self) -> [u8; 32] {
        self.sha256
    }

    pub fn original_filename(&self) -> Option<&str> {
        self.original_filename.as_deref()
    }

    pub fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }
}

/// Result of comparing stored bytes with the committed BLAKE3 digest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verified {
    Match,
    Mismatch { actual: Blake3Digest },
    Missing,
}

/// Deletes the file after the caller's transaction commits.
///
/// Dropping this without [`FileRemoval::run`] leaves an orphan for [`ObjectStore::cleanup`].
#[must_use = "run FileRemoval after the transaction commits, or the file is left for cleanup"]
pub struct FileRemoval {
    storage: Arc<dyn FileStorage>,
    key: ObjectKey,
}

impl FileRemoval {
    pub async fn run(self) -> Result<(), StoreError> {
        self.storage
            .remove(&self.key)
            .await
            .map_err(StoreError::from)
    }
}

#[derive(Clone, Debug)]
pub struct CleanupOptions {
    pub pending_older_than: Duration,
    pub orphan_grace: Duration,
    pub dry_run: bool,
    pub max_deletions: Option<usize>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CleanupReport {
    pub stale_pending_removed: usize,
    pub orphan_files_removed: usize,
    pub temp_files_removed: usize,
    pub errors: Vec<String>,
}

/// Database row plus a [`FileStorage`] adapter.
///
/// `stage` inserts a pending row and writes the file. `commit` marks that row
/// committed inside the caller's transaction, next to the application's own row.
/// Reads only return committed rows.
#[derive(Clone)]
pub struct ObjectStore {
    db: sea_orm::DatabaseConnection,
    storage: Arc<dyn FileStorage>,
    limits: StoreLimits,
}

impl ObjectStore {
    pub fn new(db: sea_orm::DatabaseConnection, storage: Arc<dyn FileStorage>) -> Self {
        Self {
            db,
            storage,
            limits: StoreLimits::default(),
        }
    }

    pub fn disabled(db: sea_orm::DatabaseConnection) -> Self {
        Self::new(db, Arc::new(super::disabled::DisabledFileStorage))
    }

    pub fn db(&self) -> &sea_orm::DatabaseConnection {
        &self.db
    }

    /// Insert a pending row, stream the body to storage while hashing, then record the digests.
    ///
    /// The pending row is committed before the file write so a crash leaves either a pending
    /// row or an orphan file. [`Self::cleanup`] removes both. Reads do not see the row until
    /// [`Self::commit`].
    pub async fn stage(
        &self,
        new: NewObject<'_>,
        body: ByteStream,
    ) -> Result<StagedObject, StoreError> {
        let content_type = validate_content_type(new.content_type)?;
        let original_filename = validate_filename(new.original_filename)?;
        let limit = effective_limit(self.limits.max_bytes, new.max_len);
        let key = generate_key(new.prefix, &content_type);
        let now = OffsetDateTime::now_utc();
        let public_id = new_uuid_v4();

        let txn = self.db.begin().await?;
        let inserted = object::ActiveModel {
            public_id: Set(public_id),
            storage_key: Set(key.as_str().to_owned()),
            content_type: Set(content_type.clone()),
            original_filename: Set(original_filename.clone()),
            status: Set(ObjectStatus::Pending),
            created_at: Set(now),
            size: Set(None),
            blake3: Set(None),
            sha256: Set(None),
            committed_at: Set(None),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
        txn.commit().await?;

        let progress = Arc::new(Mutex::new(HashProgress::new(limit)));
        let stream = ByteStream::from_stream(HashingStream {
            inner: body,
            progress: Arc::clone(&progress),
        });
        let write = self.storage.write_stream(&key, stream, limit).await;
        let too_large = progress
            .lock()
            .map(|guard| guard.too_large)
            .unwrap_or(false);
        let written = match write {
            Ok(written) => written,
            Err(FileStorageError::TooLarge { limit }) => {
                return Err(StoreError::TooLarge { limit });
            }
            Err(_err) if too_large => return Err(StoreError::TooLarge { limit }),
            Err(err) => return Err(StoreError::from(err)),
        };

        let (size, blake3, sha256) = finish_hash(progress, written)?;
        let size_column = i64::try_from(size).map_err(|_| {
            StoreError::InvalidInput("object size does not fit in a signed 64-bit integer".into())
        })?;
        let txn = self.db.begin().await?;
        let model = object::Entity::find_by_id(inserted.id)
            .one(&txn)
            .await?
            .ok_or(StoreError::NotFound)?;
        let mut active: object::ActiveModel = model.into();
        active.size = Set(Some(size_column));
        active.blake3 = Set(Some(blake3.as_bytes().to_vec()));
        active.sha256 = Set(Some(sha256.to_vec()));
        active.update(&txn).await?;
        txn.commit().await?;

        Ok(StagedObject {
            id: inserted.id,
            public_id,
            size,
            content_type,
            blake3,
            sha256,
            original_filename,
            created_at: now,
        })
    }

    /// Mark `staged` committed inside `conn`, which is the application's transaction.
    pub async fn commit<C: ConnectionTrait>(
        &self,
        conn: &C,
        staged: StagedObject,
    ) -> Result<ObjectInfo, StoreError> {
        let now = OffsetDateTime::now_utc();
        let result = object::Entity::update_many()
            .col_expr(object::Column::Status, Expr::value("committed"))
            .col_expr(object::Column::CommittedAt, Expr::value(now))
            .filter(object::Column::Id.eq(staged.id))
            .filter(object::Column::Status.eq(ObjectStatus::Pending))
            .filter(object::Column::Blake3.is_not_null())
            .exec(conn)
            .await?;
        if result.rows_affected != 1 {
            return Err(self.commit_rejected(conn, staged.id).await);
        }
        let model = object::Entity::find_by_id(staged.id)
            .one(conn)
            .await?
            .ok_or(StoreError::NotFound)?;
        committed_info(model)
    }

    async fn commit_rejected<C: ConnectionTrait>(&self, conn: &C, id: i64) -> StoreError {
        match object::Entity::find_by_id(id).one(conn).await {
            Ok(Some(row)) if row.status != ObjectStatus::Pending => StoreError::NotPending,
            Ok(Some(row)) if row.blake3.is_none() || row.sha256.is_none() => {
                StoreError::InvalidInput("pending object has no digest".into())
            }
            Ok(Some(_)) => StoreError::NotPending,
            Ok(None) => StoreError::NotFound,
            Err(err) => StoreError::Db(err),
        }
    }

    /// Stage and commit with no application row of our own.
    pub async fn put(
        &self,
        new: NewObject<'_>,
        body: ByteStream,
    ) -> Result<ObjectInfo, StoreError> {
        let staged = self.stage(new, body).await?;
        let txn = self.db.begin().await?;
        let info = self.commit(&txn, staged).await?;
        txn.commit().await?;
        Ok(info)
    }

    pub async fn put_bytes(
        &self,
        new: NewObject<'_>,
        bytes: impl Into<Bytes>,
    ) -> Result<ObjectInfo, StoreError> {
        self.put(new, ByteStream::from_bytes(bytes)).await
    }

    pub async fn get(&self, id: i64) -> Result<ObjectInfo, StoreError> {
        let model = object::Entity::find_by_id(id)
            .filter(object::Column::Status.eq(ObjectStatus::Committed))
            .one(&self.db)
            .await?
            .ok_or(StoreError::NotFound)?;
        committed_info(model)
    }

    pub async fn get_by_public_id(&self, public_id: Uuid) -> Result<ObjectInfo, StoreError> {
        let model = object::Entity::find()
            .filter(object::Column::PublicId.eq(public_id))
            .filter(object::Column::Status.eq(ObjectStatus::Committed))
            .one(&self.db)
            .await?
            .ok_or(StoreError::NotFound)?;
        committed_info(model)
    }

    pub async fn get_many(&self, ids: &[i64]) -> Result<HashMap<i64, ObjectInfo>, StoreError> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let rows = object::Entity::find()
            .filter(object::Column::Id.is_in(ids.iter().copied()))
            .filter(object::Column::Status.eq(ObjectStatus::Committed))
            .all(&self.db)
            .await?;
        let mut out = HashMap::with_capacity(rows.len());
        for row in rows {
            let info = committed_info(row)?;
            out.insert(info.id, info);
        }
        Ok(out)
    }

    pub async fn deliver(
        &self,
        obj: &ObjectInfo,
        opts: DeliverOptions,
    ) -> Result<Delivery, StoreError> {
        let key = self.key_of(obj.id).await?;
        let opts = DeliverOptions {
            content_type: obj.content_type.clone(),
            etag: Some(obj.blake3.etag()),
            last_modified: opts.last_modified.or(Some(obj.committed_at)),
            disposition: opts.disposition,
            cache: opts.cache,
        };
        self.storage
            .deliver(&key, &opts)
            .await
            .map_err(StoreError::from)
    }

    pub async fn read(
        &self,
        obj: &ObjectInfo,
        range: Option<ByteRange>,
    ) -> Result<ObjectBody, StoreError> {
        let key = self.key_of(obj.id).await?;
        self.storage
            .read(&key, range)
            .await
            .map_err(StoreError::from)
    }

    pub async fn verify(&self, obj: &ObjectInfo) -> Result<Verified, StoreError> {
        let key = self.key_of(obj.id).await?;
        match self.storage.read(&key, None).await {
            Err(FileStorageError::NotFound) => Ok(Verified::Missing),
            Err(err) => Err(StoreError::from(err)),
            Ok(body) => {
                let actual = Blake3Digest::hash(&body.data);
                if actual == obj.blake3 && body.data.len() as u64 == obj.size {
                    Ok(Verified::Match)
                } else {
                    Ok(Verified::Mismatch { actual })
                }
            }
        }
    }

    /// Rewrite a missing file when `body` hashes to the committed BLAKE3 digest.
    pub async fn restore(&self, obj: &ObjectInfo, body: ByteStream) -> Result<(), StoreError> {
        let key = self.key_of(obj.id).await?;
        match self.storage.stat(&key).await {
            Ok(_) => return Err(StoreError::AlreadyExists),
            Err(FileStorageError::NotFound) => {}
            Err(err) => return Err(StoreError::from(err)),
        }
        let limit = obj.size.saturating_add(1).max(1);
        let hashed = hash_buffered(body, limit).await?;
        if hashed.too_large || hashed.blake3 != obj.blake3 || hashed.len != obj.size {
            return Err(StoreError::ChecksumMismatch);
        }
        self.storage
            .write_stream(&key, ByteStream::from_bytes(hashed.bytes), obj.size.max(1))
            .await
            .map_err(StoreError::from)?;
        Ok(())
    }

    /// Delete the row inside `conn`. The caller runs [`FileRemoval::run`] after commit.
    pub async fn delete<C: ConnectionTrait>(
        &self,
        conn: &C,
        id: i64,
    ) -> Result<FileRemoval, StoreError> {
        let model = object::Entity::find_by_id(id)
            .one(conn)
            .await?
            .ok_or(StoreError::NotFound)?;
        let key = ObjectKey::parse(&model.storage_key)
            .map_err(|err| StoreError::InvalidInput(format!("stored key is invalid: {err}")))?;
        let result = object::Entity::delete_by_id(id).exec(conn).await?;
        if result.rows_affected == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(FileRemoval {
            storage: Arc::clone(&self.storage),
            key,
        })
    }

    pub async fn set_original_filename<C: ConnectionTrait>(
        &self,
        conn: &C,
        id: i64,
        filename: Option<&str>,
    ) -> Result<(), StoreError> {
        let filename = validate_filename(filename)?;
        let model = object::Entity::find_by_id(id)
            .filter(object::Column::Status.eq(ObjectStatus::Committed))
            .one(conn)
            .await?
            .ok_or(StoreError::NotFound)?;
        let mut active: object::ActiveModel = model.into();
        active.original_filename = Set(filename);
        active.update(conn).await?;
        Ok(())
    }

    /// Sweep stale pending rows and orphan files. Not for request handlers.
    pub async fn cleanup(&self, opts: CleanupOptions) -> Result<CleanupReport, StoreError> {
        let mut report = CleanupReport::default();
        let mut budget = opts.max_deletions;
        self.sweep_pending(&opts, &mut report, &mut budget).await?;
        self.sweep_orphans(&opts, &mut report, &mut budget).await;
        Ok(report)
    }

    async fn sweep_pending(
        &self,
        opts: &CleanupOptions,
        report: &mut CleanupReport,
        budget: &mut Option<usize>,
    ) -> Result<(), StoreError> {
        let older_than = time_duration(opts.pending_older_than)?;
        let cutoff = OffsetDateTime::now_utc() - older_than;
        let rows = object::Entity::find()
            .filter(object::Column::Status.eq(ObjectStatus::Pending))
            .filter(object::Column::CreatedAt.lt(cutoff))
            .all(&self.db)
            .await?;
        for row in rows {
            if budget.is_some_and(|left| left == 0) {
                break;
            }
            let key = match ObjectKey::parse(&row.storage_key) {
                Ok(key) => key,
                Err(err) => {
                    report.errors.push(format!("{}: {err}", row.storage_key));
                    continue;
                }
            };
            if !opts.dry_run {
                if let Err(err) = self.storage.remove(&key).await {
                    report
                        .errors
                        .push(format!("pending {}: {err}", key.as_str()));
                    continue;
                }
                if let Err(err) = object::Entity::delete_by_id(row.id).exec(&self.db).await {
                    report.errors.push(format!("pending row {}: {err}", row.id));
                    continue;
                }
            }
            report.stale_pending_removed += 1;
            spend(budget);
        }
        Ok(())
    }

    async fn sweep_orphans(
        &self,
        opts: &CleanupOptions,
        report: &mut CleanupReport,
        budget: &mut Option<usize>,
    ) {
        let referenced = match self.all_keys().await {
            Ok(keys) => keys,
            Err(err) => {
                report.errors.push(err.to_string());
                return;
            }
        };
        let listed = match self.storage.list(None).await {
            Ok(listed) => listed,
            Err(err) => {
                report.errors.push(err.to_string());
                return;
            }
        };
        let grace = match time_duration(opts.orphan_grace) {
            Ok(grace) => grace,
            Err(err) => {
                report.errors.push(err.to_string());
                return;
            }
        };
        let cutoff = OffsetDateTime::now_utc() - grace;
        for file in listed {
            if budget.is_some_and(|left| left == 0) {
                break;
            }
            if referenced.contains(file.key.as_str()) {
                continue;
            }
            let old_enough = match file.modified {
                Some(modified) => modified < cutoff,
                None => true,
            };
            if !old_enough {
                continue;
            }
            let temporary = file.temporary || file.key.is_temporary();
            if !opts.dry_run {
                if let Err(err) = self.storage.remove(&file.key).await {
                    report
                        .errors
                        .push(format!("orphan {}: {err}", file.key.as_str()));
                    continue;
                }
            }
            if temporary {
                report.temp_files_removed += 1;
            } else {
                report.orphan_files_removed += 1;
            }
            spend(budget);
        }
    }

    async fn all_keys(&self) -> Result<HashSet<String>, StoreError> {
        let rows = object::Entity::find().all(&self.db).await?;
        Ok(rows.into_iter().map(|row| row.storage_key).collect())
    }

    async fn key_of(&self, id: i64) -> Result<ObjectKey, StoreError> {
        let model = object::Entity::find_by_id(id)
            .filter(object::Column::Status.eq(ObjectStatus::Committed))
            .one(&self.db)
            .await?
            .ok_or(StoreError::NotFound)?;
        ObjectKey::parse(&model.storage_key)
            .map_err(|err| StoreError::InvalidInput(format!("stored key is invalid: {err}")))
    }
}

fn spend(budget: &mut Option<usize>) {
    if let Some(left) = budget.as_mut() {
        *left = left.saturating_sub(1);
    }
}

fn time_duration(duration: Duration) -> Result<time::Duration, StoreError> {
    time::Duration::try_from(duration)
        .map_err(|_| StoreError::InvalidInput("duration is too large".into()))
}

fn effective_limit(store_max: u64, call_max: u64) -> u64 {
    store_max.min(call_max)
}

fn validate_content_type(value: &str) -> Result<String, StoreError> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 255
        || value.chars().any(|c| c.is_control() || c.is_whitespace())
    {
        return Err(StoreError::InvalidInput(
            "content type must be one header token".into(),
        ));
    }
    Ok(value.to_owned())
}

fn validate_filename(value: Option<&str>) -> Result<Option<String>, StoreError> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_empty() {
        return Ok(None);
    }
    if value.len() > 255
        || value
            .chars()
            .any(|c| c.is_control() || c == '/' || c == '\\')
    {
        return Err(StoreError::InvalidInput(
            "original filename is empty or unsafe".into(),
        ));
    }
    Ok(Some(value.to_owned()))
}

fn committed_info(model: object::Model) -> Result<ObjectInfo, StoreError> {
    if model.status != ObjectStatus::Committed {
        return Err(StoreError::NotFound);
    }
    let size = model
        .size
        .ok_or_else(|| StoreError::InvalidInput("committed object has no size".into()))?;
    let size = u64::try_from(size)
        .map_err(|_| StoreError::InvalidInput("object size is negative".into()))?;
    let blake3 = Blake3Digest::from_slice(
        model
            .blake3
            .as_deref()
            .ok_or_else(|| StoreError::InvalidInput("committed object has no blake3".into()))?,
    )?;
    let sha_bytes = model
        .sha256
        .as_deref()
        .ok_or_else(|| StoreError::InvalidInput("committed object has no sha256".into()))?;
    let sha256: [u8; 32] = sha_bytes
        .try_into()
        .map_err(|_| StoreError::InvalidInput("sha256 digest must be 32 bytes".into()))?;
    let committed_at = model
        .committed_at
        .ok_or_else(|| StoreError::InvalidInput("committed object has no committed_at".into()))?;
    Ok(ObjectInfo {
        id: model.id,
        public_id: model.public_id,
        size,
        content_type: model.content_type,
        blake3,
        sha256,
        original_filename: model.original_filename,
        created_at: model.created_at,
        committed_at,
    })
}

struct HashProgress {
    blake3: blake3::Hasher,
    sha256: sha2::Sha256,
    len: u64,
    limit: u64,
    too_large: bool,
}

impl HashProgress {
    fn new(limit: u64) -> Self {
        Self {
            blake3: blake3::Hasher::new(),
            sha256: sha2::Sha256::new(),
            len: 0,
            limit,
            too_large: false,
        }
    }
}

struct HashingStream {
    inner: ByteStream,
    progress: Arc<Mutex<HashProgress>>,
}

impl Stream for HashingStream {
    type Item = Result<Bytes, std::io::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        match Pin::new(&mut this.inner).poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(Some(Err(err))) => Poll::Ready(Some(Err(err))),
            Poll::Ready(Some(Ok(chunk))) => {
                let mut progress = match this.progress.lock() {
                    Ok(progress) => progress,
                    Err(_) => {
                        return Poll::Ready(Some(Err(std::io::Error::other(
                            "object hasher lock poisoned",
                        ))));
                    }
                };
                let next = progress.len.saturating_add(chunk.len() as u64);
                if next > progress.limit {
                    progress.too_large = true;
                    return Poll::Ready(Some(Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "object exceeds max length",
                    ))));
                }
                progress.blake3.update(&chunk);
                progress.sha256.update(&chunk);
                progress.len = next;
                Poll::Ready(Some(Ok(chunk)))
            }
        }
    }
}

fn finish_hash(
    progress: Arc<Mutex<HashProgress>>,
    written: u64,
) -> Result<(u64, Blake3Digest, [u8; 32]), StoreError> {
    let mut progress = progress
        .lock()
        .map_err(|_| StoreError::InvalidInput("object hasher lock poisoned".into()))?;
    if progress.too_large {
        return Err(StoreError::TooLarge {
            limit: progress.limit,
        });
    }
    if progress.len != written {
        return Err(StoreError::InvalidInput(
            "hashed length does not match the bytes stored".into(),
        ));
    }
    let blake3 = Blake3Digest::from_bytes(*progress.blake3.finalize().as_bytes());
    let sha = std::mem::replace(&mut progress.sha256, sha2::Sha256::new());
    let sha256 = sha.finalize().into();
    Ok((progress.len, blake3, sha256))
}

struct BufferedHash {
    bytes: Bytes,
    len: u64,
    blake3: Blake3Digest,
    too_large: bool,
}

async fn hash_buffered(mut body: ByteStream, limit: u64) -> Result<BufferedHash, StoreError> {
    let mut buf = Vec::new();
    let mut hasher = blake3::Hasher::new();
    let mut too_large = false;
    while let Some(chunk) = body.next().await {
        let chunk = chunk.map_err(FileStorageError::io)?;
        if buf.len() as u64 + chunk.len() as u64 > limit {
            too_large = true;
            break;
        }
        hasher.update(&chunk);
        buf.extend_from_slice(&chunk);
    }
    Ok(BufferedHash {
        len: buf.len() as u64,
        blake3: Blake3Digest::from_bytes(*hasher.finalize().as_bytes()),
        bytes: Bytes::from(buf),
        too_large,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::migrations::Migrator;
    use crate::storage::port::FileStorage;
    use sea_orm::ConnectOptions;
    use sea_orm_migration::MigratorTrait;

    async fn memory_db() -> sea_orm::DatabaseConnection {
        let mut opts = ConnectOptions::new("sqlite::memory:");
        opts.max_connections(1);
        sea_orm::Database::connect(opts).await.expect("sqlite")
    }

    async fn setup() -> (
        sea_orm::DatabaseConnection,
        ObjectStore,
        Arc<crate::storage::memory::MemoryFileStorage>,
    ) {
        let db = memory_db().await;
        Migrator::up(&db, None).await.unwrap();
        let storage = Arc::new(crate::storage::memory::MemoryFileStorage::new());
        let store = ObjectStore::new(db.clone(), storage.clone());
        (db, store, storage)
    }

    fn png_object() -> NewObject<'static> {
        NewObject {
            prefix: None,
            content_type: "image/png",
            original_filename: Some("Naïve.png"),
            max_len: 1024,
        }
    }

    #[tokio::test]
    async fn stage_hides_the_row_until_commit_and_records_both_digests() {
        let (db, store, storage) = setup().await;
        let staged = store
            .stage(png_object(), ByteStream::from_bytes(&b"hello"[..]))
            .await
            .unwrap();
        assert!(matches!(
            store.get(staged.id()).await,
            Err(StoreError::NotFound)
        ));
        let pending = object::Entity::find_by_id(staged.id())
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(pending.status, ObjectStatus::Pending);
        assert!(pending.blake3.is_some());
        assert!(pending.storage_key.ends_with(".png"));
        assert!(!pending.storage_key.contains(&pending.public_id.to_string()));
        assert_ne!(pending.public_id, Uuid::nil());
        assert_eq!(pending.public_id.get_version(), Some(uuid::Version::Random));
        assert!(storage.contains(&pending.storage_key));

        let txn = db.begin().await.unwrap();
        let info = store.commit(&txn, staged).await.unwrap();
        txn.rollback().await.unwrap();
        assert!(matches!(
            store.get(info.id).await,
            Err(StoreError::NotFound)
        ));

        let staged = store
            .stage(png_object(), ByteStream::from_bytes(&b"hello"[..]))
            .await
            .unwrap();
        let txn = db.begin().await.unwrap();
        let info = store.commit(&txn, staged).await.unwrap();
        txn.commit().await.unwrap();
        assert_eq!(info.blake3, Blake3Digest::hash(b"hello"));
        assert_eq!(info.sha256, crate::storage::digest::sha256(b"hello"));
        assert_eq!(info.size, 5);
        assert_eq!(info.original_filename.as_deref(), Some("Naïve.png"));
        assert_eq!(store.get(info.id).await.unwrap().public_id, info.public_id);
        assert_eq!(
            store.get_by_public_id(info.public_id).await.unwrap().id,
            info.id
        );
    }

    #[tokio::test]
    async fn too_large_leaves_a_pending_row_and_no_file() {
        let (db, store, storage) = setup().await;
        let err = store
            .stage(
                NewObject {
                    prefix: None,
                    content_type: "text/plain",
                    original_filename: None,
                    max_len: 4,
                },
                ByteStream::from_bytes(&b"abcdef"[..]),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, StoreError::TooLarge { limit: 4 }));
        let rows = object::Entity::find().all(&db).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].status, ObjectStatus::Pending);
        assert!(rows[0].blake3.is_none());
        assert!(!storage.contains(&rows[0].storage_key));
        assert!(matches!(
            store.get(rows[0].id).await,
            Err(StoreError::NotFound)
        ));
    }

    #[tokio::test]
    async fn prefix_is_part_of_the_key_and_public_id_is_not() {
        let (_db, store, storage) = setup().await;
        let prefix = ObjectPrefix::parse("018f5c20-0000-7000-8000-000000000001").unwrap();
        let info = store
            .put_bytes(
                NewObject {
                    prefix: Some(&prefix),
                    content_type: "image/jpeg",
                    original_filename: Some("spa.jpg"),
                    max_len: 64,
                },
                &b"jpeg"[..],
            )
            .await
            .unwrap();
        let listed = storage.list(None).await.unwrap();
        assert_eq!(listed.len(), 1);
        assert!(
            listed[0]
                .key
                .as_str()
                .starts_with(&format!("{}/", prefix.as_str()))
        );
        assert!(listed[0].key.as_str().ends_with(".jpg"));
        assert!(!listed[0].key.as_str().contains(&info.public_id.to_string()));
    }

    #[tokio::test]
    async fn delete_removes_the_row_first_and_the_file_when_run() {
        let (db, store, storage) = setup().await;
        let info = store.put_bytes(png_object(), &b"hello"[..]).await.unwrap();
        let key = storage.list(None).await.unwrap()[0].key.as_str().to_owned();
        let txn = db.begin().await.unwrap();
        let removal = store.delete(&txn, info.id).await.unwrap();
        assert!(
            storage.contains(&key),
            "file stays until the caller runs removal"
        );
        txn.commit().await.unwrap();
        assert!(matches!(
            store.get(info.id).await,
            Err(StoreError::NotFound)
        ));
        assert!(storage.contains(&key));
        removal.run().await.unwrap();
        assert!(!storage.contains(&key));
    }

    #[tokio::test]
    async fn failed_file_removal_leaves_an_orphan_for_cleanup() {
        let (db, store, storage) = setup().await;
        let info = store.put_bytes(png_object(), &b"hello"[..]).await.unwrap();
        let key = storage.list(None).await.unwrap()[0].key.as_str().to_owned();
        let txn = db.begin().await.unwrap();
        let removal = store.delete(&txn, info.id).await.unwrap();
        txn.commit().await.unwrap();
        storage.fail_next_remove();
        let err = removal.run().await.unwrap_err();
        assert!(err.to_string().contains("simulated"));
        assert!(storage.contains(&key));
        storage.set_modified(&key, OffsetDateTime::now_utc() - time::Duration::hours(5));
        let report = store
            .cleanup(CleanupOptions {
                pending_older_than: Duration::from_secs(24 * 3600),
                orphan_grace: Duration::from_secs(3600),
                dry_run: false,
                max_deletions: None,
            })
            .await
            .unwrap();
        assert_eq!(report.orphan_files_removed, 1);
        assert!(!storage.contains(&key));
        assert!(report.errors.is_empty());
    }

    #[tokio::test]
    async fn verify_and_restore_round_trip() {
        let (_db, store, storage) = setup().await;
        let info = store.put_bytes(png_object(), &b"hello"[..]).await.unwrap();
        assert_eq!(store.verify(&info).await.unwrap(), Verified::Match);
        let key = storage.list(None).await.unwrap()[0].key.as_str().to_owned();
        storage.insert_file(&key, b"tampered", OffsetDateTime::now_utc());
        match store.verify(&info).await.unwrap() {
            Verified::Mismatch { actual } => assert_ne!(actual, info.blake3),
            other => panic!("expected mismatch, got {other:?}"),
        }
        storage
            .remove(&ObjectKey::parse(&key).unwrap())
            .await
            .unwrap();
        assert_eq!(store.verify(&info).await.unwrap(), Verified::Missing);
        let err = store
            .restore(&info, ByteStream::from_bytes(&b"nope"[..]))
            .await
            .unwrap_err();
        assert!(matches!(err, StoreError::ChecksumMismatch));
        assert_eq!(store.verify(&info).await.unwrap(), Verified::Missing);
        store
            .restore(&info, ByteStream::from_bytes(&b"hello"[..]))
            .await
            .unwrap();
        assert_eq!(store.verify(&info).await.unwrap(), Verified::Match);
        let err = store
            .restore(&info, ByteStream::from_bytes(&b"hello"[..]))
            .await
            .unwrap_err();
        assert!(matches!(err, StoreError::AlreadyExists));
    }

    #[tokio::test]
    async fn cleanup_removes_stale_pending_and_keeps_referenced_files() {
        let (db, store, storage) = setup().await;
        let kept = store.put_bytes(png_object(), &b"keep"[..]).await.unwrap();
        let kept_key = storage.list(None).await.unwrap()[0].key.as_str().to_owned();
        storage.set_modified(
            &kept_key,
            OffsetDateTime::now_utc() - time::Duration::hours(48),
        );

        let pending = store
            .stage(png_object(), ByteStream::from_bytes(&b"old"[..]))
            .await
            .unwrap();
        let pending_row = object::Entity::find_by_id(pending.id())
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        let mut active: object::ActiveModel = pending_row.into();
        active.created_at = Set(OffsetDateTime::now_utc() - time::Duration::hours(48));
        active.update(&db).await.unwrap();
        let pending_key = object::Entity::find_by_id(pending.id())
            .one(&db)
            .await
            .unwrap()
            .unwrap()
            .storage_key;

        let fresh = store
            .stage(png_object(), ByteStream::from_bytes(&b"new"[..]))
            .await
            .unwrap();

        storage.insert_file(
            "orphan.bin",
            b"orphan",
            OffsetDateTime::now_utc() - time::Duration::hours(5),
        );
        storage.insert_file("fresh.bin", b"fresh", OffsetDateTime::now_utc());
        storage.insert_temp(
            "notes.txt.partial",
            b"tmp",
            OffsetDateTime::now_utc() - time::Duration::hours(5),
        );
        storage.insert_temp("live.txt.partial", b"tmp", OffsetDateTime::now_utc());

        let dry = store
            .cleanup(CleanupOptions {
                pending_older_than: Duration::from_secs(3600),
                orphan_grace: Duration::from_secs(3600),
                dry_run: true,
                max_deletions: None,
            })
            .await
            .unwrap();
        assert_eq!(dry.stale_pending_removed, 1);
        assert_eq!(dry.orphan_files_removed, 1);
        assert_eq!(dry.temp_files_removed, 1);
        assert!(storage.contains(&pending_key));
        assert!(storage.contains("orphan.bin"));

        let report = store
            .cleanup(CleanupOptions {
                pending_older_than: Duration::from_secs(3600),
                orphan_grace: Duration::from_secs(3600),
                dry_run: false,
                max_deletions: None,
            })
            .await
            .unwrap();
        assert_eq!(report.stale_pending_removed, 1);
        assert_eq!(report.orphan_files_removed, 1);
        assert_eq!(report.temp_files_removed, 1);
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        assert!(!storage.contains(&pending_key));
        assert!(
            object::Entity::find_by_id(pending.id())
                .one(&db)
                .await
                .unwrap()
                .is_none()
        );
        assert!(storage.contains(&kept_key));
        assert_eq!(store.verify(&kept).await.unwrap(), Verified::Match);
        assert!(
            storage.contains(
                &object::Entity::find_by_id(fresh.id())
                    .one(&db)
                    .await
                    .unwrap()
                    .unwrap()
                    .storage_key
            )
        );
        assert!(storage.contains("fresh.bin"));
        assert!(!storage.contains("orphan.bin"));
        assert!(!storage.contains("notes.txt.partial"));
        let temps = storage.list(None).await.unwrap();
        assert!(
            temps
                .iter()
                .any(|file| file.key.as_str() == "live.txt.partial")
        );
    }

    #[tokio::test]
    async fn cleanup_respects_max_deletions() {
        let (_db, store, storage) = setup().await;
        let old = OffsetDateTime::now_utc() - time::Duration::hours(5);
        storage.insert_file("a.bin", b"a", old);
        storage.insert_file("b.bin", b"b", old);
        let report = store
            .cleanup(CleanupOptions {
                pending_older_than: Duration::from_secs(3600),
                orphan_grace: Duration::from_secs(3600),
                dry_run: false,
                max_deletions: Some(1),
            })
            .await
            .unwrap();
        assert_eq!(report.orphan_files_removed, 1);
        let left = storage.list(None).await.unwrap();
        assert_eq!(left.len(), 1);
    }

    #[cfg(feature = "storage-local")]
    #[tokio::test]
    async fn cleanup_on_local_files_keeps_referenced_bytes() {
        let db = memory_db().await;
        Migrator::up(&db, None).await.unwrap();
        let root = std::env::temp_dir().join(format!(
            "rundtisch-sweep-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let storage = Arc::new(crate::storage::local::LocalFileStorage::new(&root));
        let store = ObjectStore::new(db.clone(), storage.clone());
        let info = store
            .put_bytes(png_object(), &b"keep-local"[..])
            .await
            .unwrap();
        let listed = storage.list(None).await.unwrap();
        let kept = root.join(listed[0].key.as_str());
        set_old(&kept);

        let pending = store
            .stage(png_object(), ByteStream::from_bytes(&b"pending"[..]))
            .await
            .unwrap();
        let row = object::Entity::find_by_id(pending.id())
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        let pending_path = root.join(&row.storage_key);
        let mut active: object::ActiveModel = row.into();
        active.created_at = Set(OffsetDateTime::now_utc() - time::Duration::hours(48));
        active.update(&db).await.unwrap();

        let orphan = root.join("orphan.bin");
        std::fs::write(&orphan, b"orphan").unwrap();
        set_old(&orphan);
        let temp = root.join("crash.bin.partial");
        std::fs::write(&temp, b"partial").unwrap();
        set_old(&temp);
        let fresh = root.join("fresh.bin.partial");
        std::fs::write(&fresh, b"partial").unwrap();

        let report = store
            .cleanup(CleanupOptions {
                pending_older_than: Duration::from_secs(3600),
                orphan_grace: Duration::from_secs(3600),
                dry_run: false,
                max_deletions: None,
            })
            .await
            .unwrap();
        assert_eq!(report.stale_pending_removed, 1);
        assert_eq!(report.orphan_files_removed, 1);
        assert_eq!(report.temp_files_removed, 1);
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        assert!(kept.is_file());
        assert_eq!(store.verify(&info).await.unwrap(), Verified::Match);
        assert!(!pending_path.exists());
        assert!(!orphan.exists());
        assert!(!temp.exists());
        assert!(fresh.is_file());
        std::fs::remove_dir_all(&root).ok();
    }

    #[cfg(feature = "storage-local")]
    fn set_old(path: &std::path::Path) {
        let old = std::time::SystemTime::now() - Duration::from_secs(48 * 3600);
        std::fs::File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(old)
            .unwrap();
    }

    #[tokio::test]
    async fn commit_refuses_a_pending_row_without_a_digest() {
        let (db, store, _) = setup().await;
        let staged = store
            .stage(png_object(), ByteStream::from_bytes(&b"hello"[..]))
            .await
            .unwrap();
        let model = object::Entity::find_by_id(staged.id())
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        let mut active: object::ActiveModel = model.into();
        active.blake3 = Set(None);
        active.update(&db).await.unwrap();
        let txn = db.begin().await.unwrap();
        let err = store.commit(&txn, staged).await.unwrap_err();
        assert!(matches!(err, StoreError::InvalidInput(_)), "{err}");
    }
}
