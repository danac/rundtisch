use crate::auth::models::User;
use crate::auth::queries::user_list_query;
use crate::traits::db::{DatabaseExecutor, Error};
use crate::{AppState, Platform};
use axum::Json;
use axum::extract::State;
use axum::response::IntoResponse;
use serde_json::json;

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        Json(json!({"error": self.to_string()})).into_response()
    }
}

pub async fn list_users<P: Platform>(State(state): State<AppState<P>>) -> impl IntoResponse {
    let query = user_list_query();
    let result = state.database.fetch_all::<User>(&query).await;
    match result {
        Ok(users) => Json(json!({"result": users})).into_response(),
        Err(e) => e.into_response(),
    }
}
