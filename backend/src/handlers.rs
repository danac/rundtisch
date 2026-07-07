use crate::app_state::AppState;
use crate::traits::Platform;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::{json, Value};

pub async fn health<P: Platform>(
    State(_): State<AppState<P>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let headers_map = headers
        .iter()
        .map(|(k, v)| {
            (
                k.as_str().to_owned(),
                Value::String(String::from_utf8_lossy(v.as_bytes()).into_owned()),
            )
        })
        .collect::<Vec<_>>();
    Json(json!({"status": "ok","headers": headers_map}))
}
