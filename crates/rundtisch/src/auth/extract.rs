use crate::auth::config::{AUTH_HASH_PEPPER, AUTH_JWT_ACCESS_SECRET};
use crate::auth::error::AuthError;
use crate::auth::jwt::{AccessClaims, verify_access_token};
use crate::auth::models::Role;
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

/// Logged-in user whose access token role is `Admin`.
pub struct AdminUser(pub AccessClaims);

impl FromRequestParts<AppState> for AdminUser {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let BearerUser(claims) = BearerUser::from_request_parts(parts, state).await?;
        if claims.role != Role::Admin {
            return Err(AuthError::Forbidden.into_response());
        }
        Ok(AdminUser(claims))
    }
}

pub fn secret_bytes(state: &AppState, name: &str, min_len: usize) -> Result<Vec<u8>, AuthError> {
    let value = match state.secret(name) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("auth secret {name} is not set");
            return Err(err.into());
        }
    };
    let bytes = value.into_bytes();
    if bytes.len() < min_len {
        eprintln!("auth secret {name} is shorter than {min_len} bytes");
        return Err(AuthError::Secrets);
    }
    if name == AUTH_HASH_PEPPER && bytes.len() != 32 {
        eprintln!("auth secret {name} must be exactly 32 bytes");
        return Err(AuthError::Secrets);
    }
    Ok(bytes)
}
