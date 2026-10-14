use std::time::Duration;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use rundtisch::AppState;
use rundtisch::auth::extract::SessionUser;
use rundtisch::storage::{CachePolicy, DeliverOptions, Disposition, StoreError};
use serde::Deserialize;
use serde_json::json;

use uuid::Uuid;

use super::queries::{self, CollectionResponse, PhotoResponse, QueryError};

pub enum LibraryError {
    NotFound(&'static str),
    Internal(&'static str),
}

impl IntoResponse for LibraryError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            LibraryError::NotFound(message) => (StatusCode::NOT_FOUND, message),
            LibraryError::Internal(message) => (StatusCode::INTERNAL_SERVER_ERROR, message),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}

fn map_query(err: QueryError, not_found: &'static str, internal: &'static str) -> LibraryError {
    match err {
        QueryError::NotFound => LibraryError::NotFound(not_found),
        QueryError::Db(err) => {
            eprintln!("library: {err}");
            LibraryError::Internal(internal)
        }
    }
}

pub async fn list_collections(
    _user: SessionUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<CollectionResponse>>, LibraryError> {
    queries::list_collections(&state.objects)
        .await
        .map(Json)
        .map_err(|err| map_query(err, "Collection not found.", "Unable to load collections."))
}

pub async fn get_collection(
    _user: SessionUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<CollectionResponse>, LibraryError> {
    queries::collection_by_public_id(&state.objects, id)
        .await
        .map(Json)
        .map_err(|err| {
            map_query(
                err,
                "Collection not found.",
                "Unable to load this collection.",
            )
        })
}

pub async fn list_photos(
    _user: SessionUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<PhotoResponse>>, LibraryError> {
    queries::photos_by_collection(&state.objects, id)
        .await
        .map(Json)
        .map_err(|err| {
            map_query(
                err,
                "Collection not found.",
                "Unable to load this collection.",
            )
        })
}

#[derive(Debug, Deserialize)]
pub struct FileQuery {
    #[serde(default)]
    download: bool,
}

pub async fn photo_file(
    _user: SessionUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(query): Query<FileQuery>,
    method: Method,
    headers: HeaderMap,
) -> Result<Response, LibraryError> {
    let picture = queries::picture_by_public_id(&state.db, id)
        .await
        .map_err(|err| map_query(err, "Photo not found.", "Unable to load this photo."))?;
    let object = state
        .objects
        .get(picture.object_id)
        .await
        .map_err(map_store)?;
    let disposition = if query.download {
        Disposition::attachment_from(&object)
    } else {
        Disposition::Inline {
            filename: object.original_filename.clone(),
        }
    };
    let delivery = state
        .objects
        .deliver(
            &object,
            DeliverOptions::new(
                Some(disposition),
                CachePolicy::private(Duration::from_secs(86_400)),
            ),
        )
        .await
        .map_err(map_store)?;
    Ok(delivery.respond(&method, &headers).await)
}

fn map_store(err: StoreError) -> LibraryError {
    match err {
        StoreError::NotFound => LibraryError::NotFound("Photo not found."),
        other => {
            eprintln!("library: {other}");
            LibraryError::Internal("Unable to load this photo.")
        }
    }
}
