use crate::auth::jwt::TokenError;
use crate::auth::password::{PasswordError, PasswordHashError};
use crate::traits::db::Error as DbError;
use crate::traits::random::RandomError;
use crate::traits::secrets::SecretError;
use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug)]
pub enum AuthError {
    InvalidCredentials,
    EmailNotVerified,
    InvalidToken,
    TokenExpired,
    InvalidPassword,
    TypeMismatch,
    Secrets,
    Db(DbError),
    Random(RandomError),
    Password(PasswordHashError),
    Token(TokenError),
}

impl From<DbError> for AuthError {
    fn from(value: DbError) -> Self {
        AuthError::Db(value)
    }
}

impl From<RandomError> for AuthError {
    fn from(value: RandomError) -> Self {
        AuthError::Random(value)
    }
}

impl From<PasswordHashError> for AuthError {
    fn from(value: PasswordHashError) -> Self {
        AuthError::Password(value)
    }
}

impl From<TokenError> for AuthError {
    fn from(value: TokenError) -> Self {
        match value {
            TokenError::Expired => AuthError::TokenExpired,
            TokenError::Invalid => AuthError::InvalidToken,
            other => AuthError::Token(other),
        }
    }
}

impl From<SecretError> for AuthError {
    fn from(_: SecretError) -> Self {
        AuthError::Secrets
    }
}

impl From<PasswordError> for AuthError {
    fn from(_: PasswordError) -> Self {
        AuthError::InvalidPassword
    }
}

impl AuthError {
    fn status_and_code(&self) -> (StatusCode, &'static str) {
        match self {
            AuthError::InvalidCredentials => (StatusCode::UNAUTHORIZED, "invalid_credentials"),
            AuthError::EmailNotVerified => (StatusCode::FORBIDDEN, "email_not_verified"),
            AuthError::InvalidToken => (StatusCode::UNAUTHORIZED, "invalid_token"),
            AuthError::TokenExpired => (StatusCode::UNAUTHORIZED, "token_expired"),
            AuthError::InvalidPassword => (StatusCode::BAD_REQUEST, "invalid_password"),
            AuthError::TypeMismatch => (StatusCode::BAD_REQUEST, "type mismatch"),
            AuthError::Secrets
            | AuthError::Random(_)
            | AuthError::Password(_)
            | AuthError::Token(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
            AuthError::Db(DbError::NotFound) => (StatusCode::NOT_FOUND, "not found"),
            AuthError::Db(DbError::Conflict) => (StatusCode::CONFLICT, "conflict"),
            AuthError::Db(DbError::TypeMismatch) => (StatusCode::BAD_REQUEST, "type mismatch"),
            AuthError::Db(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
        }
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, code) = self.status_and_code();
        (status, Json(json!({"error": code}))).into_response()
    }
}
