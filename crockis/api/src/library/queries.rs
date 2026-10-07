use std::collections::HashMap;

use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::Serialize;
use time::format_description::well_known::Rfc3339;

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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PhotoResponse {
    pub id: String,
    pub collection_id: String,
    pub src: String,
    pub width: i32,
    pub height: i32,
    pub alt: String,
    pub title: String,
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

pub async fn list_collections<C: ConnectionTrait>(
    db: &C,
) -> Result<Vec<CollectionResponse>, QueryError> {
    let collections = collection::Entity::find()
        .order_by_asc(collection::Column::SortOrder)
        .order_by_asc(collection::Column::Id)
        .all(db)
        .await?;
    let pictures = picture::Entity::find()
        .order_by_asc(picture::Column::SortOrder)
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
    let mut out = Vec::with_capacity(collections.len());
    for collection in collections {
        let photos = by_collection.remove(&collection.id).unwrap_or_default();
        out.push(collection_response(collection, photos)?);
    }
    Ok(out)
}

pub async fn collection_by_public_id<C: ConnectionTrait>(
    db: &C,
    public_id: &str,
) -> Result<CollectionResponse, QueryError> {
    let collection = collection::Entity::find()
        .filter(collection::Column::PublicId.eq(public_id))
        .one(db)
        .await?
        .ok_or(QueryError::NotFound)?;
    let photos = picture::Entity::find()
        .filter(picture::Column::CollectionId.eq(collection.id))
        .order_by_asc(picture::Column::SortOrder)
        .order_by_asc(picture::Column::Id)
        .all(db)
        .await?;
    collection_response(collection, photos)
}

pub async fn photos_by_collection<C: ConnectionTrait>(
    db: &C,
    public_id: &str,
) -> Result<Vec<PhotoResponse>, QueryError> {
    let collection = collection::Entity::find()
        .filter(collection::Column::PublicId.eq(public_id))
        .one(db)
        .await?
        .ok_or(QueryError::NotFound)?;
    let photos = picture::Entity::find()
        .filter(picture::Column::CollectionId.eq(collection.id))
        .order_by_asc(picture::Column::SortOrder)
        .order_by_asc(picture::Column::Id)
        .all(db)
        .await?;
    photos
        .into_iter()
        .map(|photo| photo_response(photo, &collection.public_id))
        .collect()
}

pub async fn picture_file<C: ConnectionTrait>(
    db: &C,
    public_id: &str,
) -> Result<picture::Model, QueryError> {
    picture::Entity::find()
        .filter(picture::Column::PublicId.eq(public_id))
        .one(db)
        .await?
        .ok_or(QueryError::NotFound)
}

fn collection_response(
    collection: collection::Model,
    photos: Vec<picture::Model>,
) -> Result<CollectionResponse, QueryError> {
    let cover = cover_photo(&collection, &photos)
        .map(|photo| photo_response(photo.clone(), &collection.public_id))
        .transpose()?;
    Ok(CollectionResponse {
        id: collection.public_id,
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
    collection_public_id: &str,
) -> Result<PhotoResponse, QueryError> {
    let taken_at = photo.captured_at.format(&Rfc3339).map_err(|err| {
        QueryError::Db(sea_orm::DbErr::Custom(format!(
            "capture date for {}: {err}",
            photo.public_id
        )))
    })?;
    Ok(PhotoResponse {
        src: format!("/api/photos/{}/file", photo.public_id),
        id: photo.public_id,
        collection_id: collection_public_id.to_owned(),
        width: photo.width,
        height: photo.height,
        alt: photo.alt,
        title: photo.title,
        taken_at,
        filename: photo.original_filename,
    })
}
