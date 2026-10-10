use axum::Json;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use rundtisch::AppState;
use rundtisch::auth::extract::SessionUser;
use serde_json::json;

use uuid::Uuid;

use super::queries::{self, CollectionResponse, PhotoResponse, QueryError};
use super::store::{content_type_for_storage_name, data_dir, storage_path};

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
    queries::list_collections(&state.db)
        .await
        .map(Json)
        .map_err(|err| map_query(err, "Collection not found.", "Unable to load collections."))
}

pub async fn get_collection(
    _user: SessionUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<CollectionResponse>, LibraryError> {
    queries::collection_by_public_id(&state.db, id)
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
    queries::photos_by_collection(&state.db, id)
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

pub async fn photo_file(
    _user: SessionUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Response, LibraryError> {
    let picture = queries::picture_file(&state.db, id)
        .await
        .map_err(|err| map_query(err, "Photo not found.", "Unable to load this photo."))?;
    let path = storage_path(
        &data_dir(),
        picture.storage_folder,
        &picture.storage_filename,
    )
    .ok_or_else(|| {
        eprintln!(
            "library: rejected storage filename {}",
            picture.storage_filename
        );
        LibraryError::NotFound("Photo not found.")
    })?;
    let bytes = match tokio::fs::read(&path).await {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("library: missing file {}", path.display());
            return Err(LibraryError::NotFound("Photo not found."));
        }
        Err(err) => {
            eprintln!("library: read {}: {err}", path.display());
            return Err(LibraryError::Internal("Unable to load this photo."));
        }
    };
    Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            content_type_for_storage_name(&picture.storage_filename),
        )
        .header(header::CACHE_CONTROL, "private, max-age=86400")
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .header(header::CONTENT_LENGTH, bytes.len())
        .body(axum::body::Body::from(bytes))
        .map_err(|err| {
            eprintln!("library: response: {err}");
            LibraryError::Internal("Unable to load this photo.")
        })
}
