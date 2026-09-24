use crate::auth::config::{AUTH_HASH_PEPPER, AUTH_JWT_ACCESS_SECRET};
use crate::auth::error::AuthError;
use crate::auth::jwt::{AccessClaims, verify_access_token};
use crate::AppState;
use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};

pub struct BearerUser(pub AccessClaims);

impl FromRequestParts<AppState> for BearerUser {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| AuthError::InvalidToken.into_response())?;
        let token = header
            .strip_prefix("Bearer ")
            .ok_or_else(|| AuthError::InvalidToken.into_response())?;
        let secret = secret_bytes(state, AUTH_JWT_ACCESS_SECRET, 32)
            .map_err(IntoResponse::into_response)?;
        let claims = verify_access_token(token, &secret, time::OffsetDateTime::now_utc())
            .map_err(AuthError::from)
            .map_err(IntoResponse::into_response)?;
        Ok(BearerUser(claims))
    }
}

pub fn secret_bytes(state: &AppState, name: &str, min_len: usize) -> Result<Vec<u8>, AuthError> {
    let value = state.secret(name)?;
    let bytes = value.into_bytes();
    if bytes.len() < min_len {
        return Err(AuthError::Secrets);
    }
    if name == AUTH_HASH_PEPPER && bytes.len() != 32 {
        return Err(AuthError::Secrets);
    }
    Ok(bytes)
}
