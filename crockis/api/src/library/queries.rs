use std::collections::HashMap;

use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::Serialize;
use time::format_description::well_known::Rfc3339;
use uuid::Uuid;

use rundtisch::storage::{ObjectInfo, ObjectStore, StoreError};

use super::entities::{collection, picture};

#[derive(Debug)]
pub enum QueryError {
    NotFound,
    Db(sea_orm::DbErr),
}

impl From<sea_orm::DbErr> for QueryError {
    fn from(err: sea_orm::DbErr) -> Self {
        QueryError::Db(err)
    }
}

impl From<StoreError> for QueryError {
    fn from(err: StoreError) -> Self {
        match err {
            StoreError::Db(err) => QueryError::Db(err),
            other => QueryError::Db(sea_orm::DbErr::Custom(other.to_string())),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PhotoResponse {
    pub id: String,
    pub collection_id: String,
    pub src: String,
    pub width: i32,
    pub height: i32,
    pub taken_at: String,
    pub filename: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub cover: Option<PhotoResponse>,
    pub photo_count: i64,
}

pub async fn list_collections(store: &ObjectStore) -> Result<Vec<CollectionResponse>, QueryError> {
    let db = store.db();
    let collections = collection::Entity::find()
        .order_by_asc(collection::Column::CreatedAt)
        .order_by_asc(collection::Column::Id)
        .all(db)
        .await?;
    let pictures = picture::Entity::find()
        .order_by_asc(picture::Column::Id)
        .all(db)
        .await?;
    let mut by_collection: HashMap<i64, Vec<picture::Model>> = HashMap::new();
    for picture in pictures {
        by_collection
            .entry(picture.collection_id)
            .or_default()
            .push(picture);
    }
    let objects = filenames(store, &by_collection).await?;
    let mut out = Vec::with_capacity(collections.len());
    for collection in collections {
        let photos = by_collection.remove(&collection.id).unwrap_or_default();
        out.push(collection_response(collection, photos, &objects)?);
    }
    Ok(out)
}

pub async fn collection_by_public_id(
    store: &ObjectStore,
    public_id: Uuid,
) -> Result<CollectionResponse, QueryError> {
    let db = store.db();
    let collection = collection::Entity::find()
        .filter(collection::Column::PublicId.eq(public_id))
        .one(db)
        .await?
        .ok_or(QueryError::NotFound)?;
    let photos = picture::Entity::find()
        .filter(picture::Column::CollectionId.eq(collection.id))
        .order_by_asc(picture::Column::Id)
        .all(db)
        .await?;
    let objects = load_objects(store, &photos).await?;
    collection_response(collection, photos, &objects)
}

pub async fn photos_by_collection(
    store: &ObjectStore,
    public_id: Uuid,
) -> Result<Vec<PhotoResponse>, QueryError> {
    let db = store.db();
    let collection = collection::Entity::find()
        .filter(collection::Column::PublicId.eq(public_id))
        .one(db)
        .await?
        .ok_or(QueryError::NotFound)?;
    let photos = picture::Entity::find()
        .filter(picture::Column::CollectionId.eq(collection.id))
        .order_by_asc(picture::Column::Id)
        .all(db)
        .await?;
    let objects = load_objects(store, &photos).await?;
    photos
        .into_iter()
        .map(|photo| photo_response(photo, collection.public_id, &objects))
        .collect()
}

pub struct PictureObject {
    pub object_id: i64,
}

pub async fn picture_by_public_id<C: ConnectionTrait>(
    db: &C,
    public_id: Uuid,
) -> Result<PictureObject, QueryError> {
    let picture = picture::Entity::find()
        .filter(picture::Column::PublicId.eq(public_id))
        .one(db)
        .await?
        .ok_or(QueryError::NotFound)?;
    Ok(PictureObject {
        object_id: picture.object_id,
    })
}

async fn filenames(
    store: &ObjectStore,
    by_collection: &HashMap<i64, Vec<picture::Model>>,
) -> Result<HashMap<i64, ObjectInfo>, QueryError> {
    let ids: Vec<i64> = by_collection
        .values()
        .flatten()
        .map(|photo| photo.object_id)
        .collect();
    Ok(store.get_many(&ids).await?)
}

async fn load_objects(
    store: &ObjectStore,
    photos: &[picture::Model],
) -> Result<HashMap<i64, ObjectInfo>, QueryError> {
    let ids: Vec<i64> = photos.iter().map(|photo| photo.object_id).collect();
    Ok(store.get_many(&ids).await?)
}

fn collection_response(
    collection: collection::Model,
    photos: Vec<picture::Model>,
    objects: &HashMap<i64, ObjectInfo>,
) -> Result<CollectionResponse, QueryError> {
    let cover = cover_photo(&collection, &photos)
        .map(|photo| photo_response(photo.clone(), collection.public_id, objects))
        .transpose()?;
    Ok(CollectionResponse {
        id: collection.public_id.to_string(),
        name: collection.name,
        description: collection.description,
        cover,
        photo_count: photos.len() as i64,
    })
}

fn cover_photo<'a>(
    collection: &collection::Model,
    photos: &'a [picture::Model],
) -> Option<&'a picture::Model> {
    if let Some(thumbnail_id) = collection.thumbnail_picture_id {
        if let Some(photo) = photos.iter().find(|photo| photo.id == thumbnail_id) {
            return Some(photo);
        }
    }
    photos.first()
}

fn photo_response(
    photo: picture::Model,
    collection_public_id: Uuid,
    objects: &HashMap<i64, ObjectInfo>,
) -> Result<PhotoResponse, QueryError> {
    let taken_at = photo.captured_at.format(&Rfc3339).map_err(|err| {
        QueryError::Db(sea_orm::DbErr::Custom(format!(
            "capture date for {}: {err}",
            photo.public_id
        )))
    })?;
    let filename = objects
        .get(&photo.object_id)
        .and_then(|object| object.original_filename.clone())
        .ok_or_else(|| {
            QueryError::Db(sea_orm::DbErr::Custom(format!(
                "picture {} has no stored filename",
                photo.public_id
            )))
        })?;
    Ok(PhotoResponse {
        src: format!("/api/photos/{}/file", photo.public_id),
        id: photo.public_id.to_string(),
        collection_id: collection_public_id.to_string(),
        width: photo.width,
        height: photo.height,
        taken_at,
        filename,
    })
}
