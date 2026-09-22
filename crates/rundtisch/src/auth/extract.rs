use crate::auth::error::AuthError;
use crate::auth::jwt::{AccessClaims, verify_access_token};
use crate::traits::secrets::{JWT_ACCESS_SECRET, SecretStore};
use crate::{AppState, Platform};
use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};

pub struct BearerUser(pub AccessClaims);

impl<P: Platform> FromRequestParts<AppState<P>> for BearerUser {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState<P>,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| AuthError::InvalidToken.into_response())?;
        let token = header
            .strip_prefix("Bearer ")
            .ok_or_else(|| AuthError::InvalidToken.into_response())?;
        let secret = secret_bytes(&*state.secrets, JWT_ACCESS_SECRET, 32)
            .map_err(IntoResponse::into_response)?;
        let claims = verify_access_token(token, &secret, &*state.clock)
            .map_err(AuthError::from)
            .map_err(IntoResponse::into_response)?;
        Ok(BearerUser(claims))
    }
}

pub fn secret_bytes(
    store: &dyn SecretStore,
    name: &str,
    min_len: usize,
) -> Result<Vec<u8>, AuthError> {
    let value = store.get(name)?;
    let bytes = value.into_bytes();
    if bytes.len() < min_len {
        return Err(AuthError::Secrets);
    }
    if name == crate::traits::secrets::HASH_PEPPER && bytes.len() != 32 {
        return Err(AuthError::Secrets);
    }
    Ok(bytes)
}
