use crate::app_state::AppState;
use crate::traits::Platform;
use axum::extract::State;
use axum::http::StatusCode;

pub async fn health<P: Platform>(State(_): State<AppState<P>>) -> StatusCode {
    StatusCode::OK
}
