use crate::auth::config::{AUTH_HASH_PEPPER, SESSION_COOKIE};
use crate::auth::error::AuthError;
use crate::auth::models::User;
use crate::auth::services::authenticate_token;
use crate::auth::session::cookie_value;
use crate::AppState;
use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};

pub struct SessionUser(pub User);

impl FromRequestParts<AppState> for SessionUser {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = presented_token(&parts.headers).ok_or_else(|| {
            AuthError::InvalidToken.into_response()
        })?;
        let pepper = secret_bytes(state, AUTH_HASH_PEPPER, 32).map_err(IntoResponse::into_response)?;
        let (user, _session_id) = authenticate_token(&state.db, &pepper, &token)
            .await
            .map_err(IntoResponse::into_response)?;
        Ok(SessionUser(user))
    }
}

pub fn presented_token(headers: &axum::http::HeaderMap) -> Option<String> {
    if let Some(header) = headers.get(AUTHORIZATION).and_then(|value| value.to_str().ok()) {
        if let Some(token) = header.strip_prefix("Bearer ") {
            let token = token.trim();
            if !token.is_empty() {
                return Some(token.to_string());
            }
        }
    }
    cookie_value(headers, SESSION_COOKIE)
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
