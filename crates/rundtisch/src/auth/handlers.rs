use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use crate::{AppState, Platform};
use serde_json::{Value, json};
use crate::auth::queries::user_list_query;
use crate::auth::models::User;
use crate::traits::db::{query_to_sql, DatabaseExecutor, Dialect, Error};

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        Json(json!({"error": self.to_string()})).into_response()
    }
}

// TODO: fix the interface: this should look like:
//let (sql, values) = {
//let query = user_list_query();
//query_to_sql(&query, state.database.dialect()).expect("failed to render SQL")
//};
//let result = state.database.fetch_all::<User>(sql, values).await;
pub async fn list_users<P: Platform>(
    State(state): State<AppState<P>>,
) -> impl IntoResponse {
    let query = user_list_query();
    let result = state.database.fetch_all::<User>(&query).await;
    match result {
        Ok(users) => Json(json!({"result": users})).into_response(),
        Err(e) => e.into_response()
    }
}
