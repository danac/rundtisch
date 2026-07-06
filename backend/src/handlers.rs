use crate::app_state::AppState;
use crate::traits::Platform;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use serde_json::{json, Value};

pub async fn health<P: Platform>(
    State(_): State<AppState<P>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    Json(json!({"status": "ok!","headers": headers.iter()
        .map(|(k, v)| (k.as_str().to_string(), json!(v.to_str().unwrap_or("").to_string()))).collect::<serde_json::Map<String, Value> >()}))
}
