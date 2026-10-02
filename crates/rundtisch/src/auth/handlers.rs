use crate::AppState;
use crate::auth::config::{
    AUTH_HASH_PEPPER, AUTH_WEBAUTHN_RP_ORIGIN, DEFAULT_WEBAUTHN_RP_ORIGIN, RECOVERY_TTL,
    STEP_UP_HEADER,
};
use crate::auth::error::AuthError;
use crate::auth::extract::presented_token;
use crate::auth::extract::{SessionUser, secret_bytes};
use crate::auth::models::{AccountView, SessionGrant};
use crate::auth::password::{Argon2idHasher, PasswordHasher, check_password_policy};
use crate::auth::services::{
    authenticate_step_up, clear_session_password, complete_password_recovery,
    complete_password_registration, create_recovery_token, delete_passkey, finish_invite_passkey,
    finish_passkey_login, finish_recovery_passkey, finish_session_passkey,
    finish_step_up_passkey_login, list_passkey_info, list_session_info, login_with_password,
    logout_all_for_user, logout_current, mint_cli_session, request_recovery,
    revoke_session_for_user, set_session_password, start_invite_passkey, start_passkey_login,
    start_recovery_passkey, start_session_passkey, start_step_up_passkey_login,
    step_up_with_password,
};
use crate::auth::session::{
    cap_user_agent, clear_session_cookie_header, session_cookie_header, token_hash,
};
use crate::auth::webauthn::PasskeyCeremony;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode, header};
use axum::response::IntoResponse;
use email_address::EmailAddress;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;
use zeroize::Zeroize;

#[cfg(test)]
use crate::auth::password::TestPasswordHasher;

#[derive(Debug, Deserialize)]
pub struct RegisterPasswordBody {
    pub token: String,
    pub password: String,
    pub alias: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct InvitePasskeyOptionsBody {
    pub token: String,
    pub alias: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FlowCredentialBody {
    pub flow_id: String,
    pub credential: Value,
}

#[derive(Debug, Deserialize)]
pub struct LoginBody {
    pub email: EmailAddress,
    pub password: String,
}

/// Usernameless passkey login. An email would let the options response
/// reveal whether that account has a credential.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PasskeyLoginOptionsBody {}

#[derive(Debug, Deserialize)]
pub struct RequestResetBody {
    pub email: EmailAddress,
}

#[derive(Debug, Deserialize)]
pub struct ResetPasswordBody {
    pub token: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct RecoveryTokenBody {
    pub token: String,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PasskeyOptionsBody {
    #[serde(default)]
    pub label: Option<String>,
}

fn hasher(state: &AppState) -> Result<Box<dyn PasswordHasher>, AuthError> {
    #[cfg(test)]
    if std::env::var("RUNDTISCH_TEST_PASSWORD_HASHER")
        .ok()
        .as_deref()
        == Some("1")
    {
        return Ok(Box::new(TestPasswordHasher));
    }
    let pepper = secret_bytes(state, AUTH_HASH_PEPPER, 32)?;
    Ok(Box::new(Argon2idHasher::new(pepper)))
}

fn normalize_alias(alias: String) -> Result<String, AuthError> {
    let trimmed = alias.trim().to_string();
    if trimmed.is_empty() || trimmed.len() > 128 {
        Err(AuthError::TypeMismatch)
    } else {
        Ok(trimmed)
    }
}

fn normalize_passkey_label(label: Option<String>) -> Result<Option<String>, AuthError> {
    let Some(label) = label else {
        return Ok(None);
    };
    let label = label.trim().to_string();
    if label.is_empty() {
        return Ok(None);
    }
    if label.chars().count() > 64 {
        return Err(AuthError::TypeMismatch);
    }
    Ok(Some(label))
}

fn session_json(grant: &SessionGrant) -> serde_json::Value {
    json!({
        "token": grant.token,
        "token_type": "Bearer",
        "expires_in": grant.expires_in,
        "user": AccountView::from(&grant.user),
    })
}

fn session_response(status: StatusCode, grant: SessionGrant) -> axum::response::Response {
    let mut response = (status, Json(session_json(&grant))).into_response();
    if let Ok(cookie) = session_cookie_header(&grant.token) {
        response.headers_mut().insert(header::SET_COOKIE, cookie);
    }
    response
}

fn ceremony_response(flow_id: String, options: Value) -> axum::response::Response {
    Json(json!({
        "flow_id": flow_id,
        "options": options,
    }))
    .into_response()
}

pub async fn register_with_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<RegisterPasswordBody>,
) -> impl IntoResponse {
    register_password(State(state), headers, Json(body)).await
}

pub async fn register_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut body): Json<RegisterPasswordBody>,
) -> impl IntoResponse {
    let result: Result<axum::response::Response, AuthError> = async {
        if let Err(err) = check_password_policy(&body.password) {
            body.password.zeroize();
            return Err(err.into());
        }
        let alias = match body.alias {
            Some(alias) => Some(normalize_alias(alias)?),
            None => None,
        };
        let password_hasher = hasher(&state)?;
        let password_hash = match password_hasher.hash(&body.password) {
            Ok(hash) => hash,
            Err(err) => {
                body.password.zeroize();
                return Err(err.into());
            }
        };
        body.password.zeroize();
        let pepper = secret_bytes(&state, AUTH_HASH_PEPPER, 32)?;
        let grant = complete_password_registration(
            &state.db,
            &pepper,
            &body.token,
            password_hash,
            alias,
            cap_user_agent(&headers).as_deref(),
        )
        .await?;
        Ok(session_response(StatusCode::CREATED, grant))
    }
    .await;
    match result {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

pub async fn register_passkey_options(
    State(state): State<AppState>,
    Json(body): Json<InvitePasskeyOptionsBody>,
) -> impl IntoResponse {
    let result: Result<axum::response::Response, AuthError> = async {
        let alias = match body.alias {
            Some(alias) => Some(normalize_alias(alias)?),
            None => None,
        };
        let passkey_label = normalize_passkey_label(body.label)?;
        let pepper = secret_bytes(&state, AUTH_HASH_PEPPER, 32)?;
        let ceremony = PasskeyCeremony::from_app(&state)?;
        let (flow_id, options) = start_invite_passkey(
            &state.db,
            &pepper,
            &ceremony,
            &body.token,
            alias,
            passkey_label,
        )
        .await?;
        Ok(ceremony_response(flow_id, options))
    }
    .await;
    match result {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

pub async fn register_passkey(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<FlowCredentialBody>,
) -> impl IntoResponse {
    let result: Result<axum::response::Response, AuthError> = async {
        let pepper = secret_bytes(&state, AUTH_HASH_PEPPER, 32)?;
        let ceremony = PasskeyCeremony::from_app(&state)?;
        let grant = finish_invite_passkey(
            &state.db,
            &pepper,
            &ceremony,
            &body.flow_id,
            &body.credential,
            cap_user_agent(&headers).as_deref(),
        )
        .await?;
        Ok(session_response(StatusCode::CREATED, grant))
    }
    .await;
    match result {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut body): Json<LoginBody>,
) -> impl IntoResponse {
    let result: Result<axum::response::Response, AuthError> = async {
        let pepper = secret_bytes(&state, AUTH_HASH_PEPPER, 32)?;
        let password_hasher = hasher(&state)?;
        let grant = login_with_password(
            &state.db,
            &pepper,
            password_hasher.as_ref(),
            body.email.as_ref(),
            &body.password,
            cap_user_agent(&headers).as_deref(),
        )
        .await;
        body.password.zeroize();
        Ok(session_response(StatusCode::OK, grant?))
    }
    .await;
    match result {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

pub async fn passkey_login_options(
    State(state): State<AppState>,
    Json(_body): Json<PasskeyLoginOptionsBody>,
) -> impl IntoResponse {
    let result: Result<axum::response::Response, AuthError> = async {
        let ceremony = PasskeyCeremony::from_app(&state)?;
        let (flow_id, options) = start_passkey_login(&state.db, &ceremony).await?;
        Ok(ceremony_response(flow_id, options))
    }
    .await;
    match result {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

pub async fn passkey_login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<FlowCredentialBody>,
) -> impl IntoResponse {
    let result: Result<axum::response::Response, AuthError> = async {
        let pepper = secret_bytes(&state, AUTH_HASH_PEPPER, 32)?;
        let ceremony = PasskeyCeremony::from_app(&state)?;
        let grant = finish_passkey_login(
            &state.db,
            &pepper,
            &ceremony,
            &body.flow_id,
            &body.credential,
            cap_user_agent(&headers).as_deref(),
        )
        .await?;
        Ok(session_response(StatusCode::OK, grant))
    }
    .await;
    match result {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

fn step_up_token_header(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(STEP_UP_HEADER)?.to_str().ok()?.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

async fn require_step_up(
    state: &AppState,
    headers: &HeaderMap,
    user_id: i64,
) -> Result<(), AuthError> {
    let Some(token) = step_up_token_header(headers) else {
        return Err(AuthError::StepUpRequired);
    };
    let pepper = secret_bytes(state, AUTH_HASH_PEPPER, 32)?;
    authenticate_step_up(&state.db, &pepper, &token, user_id).await
}

fn step_up_response(token: &str, expires_in: i64) -> axum::response::Response {
    let mut response = (
        StatusCode::OK,
        Json(json!({
            "expires_in": expires_in,
            "token_type": "StepUp",
        })),
    )
        .into_response();
    response.headers_mut().insert(
        HeaderName::from_static(STEP_UP_HEADER),
        HeaderValue::from_str(token).expect("step-up token is header-safe"),
    );
    response
}

pub async fn step_up_login(
    State(state): State<AppState>,
    SessionUser(user): SessionUser,
    Json(mut body): Json<LoginBody>,
) -> impl IntoResponse {
    let result: Result<axum::response::Response, AuthError> = async {
        let pepper = secret_bytes(&state, AUTH_HASH_PEPPER, 32)?;
        let password_hasher = hasher(&state)?;
        let grant = step_up_with_password(
            &state.db,
            &pepper,
            password_hasher.as_ref(),
            &user,
            body.email.as_ref(),
            &body.password,
        )
        .await;
        body.password.zeroize();
        let grant = grant?;
        Ok(step_up_response(&grant.token, grant.expires_in))
    }
    .await;
    match result {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

pub async fn step_up_passkey_login_options(
    State(state): State<AppState>,
    SessionUser(_user): SessionUser,
    Json(_body): Json<PasskeyLoginOptionsBody>,
) -> impl IntoResponse {
    let result: Result<axum::response::Response, AuthError> = async {
        let ceremony = PasskeyCeremony::from_app(&state)?;
        let (flow_id, options) = start_step_up_passkey_login(&state.db, &ceremony).await?;
        Ok(ceremony_response(flow_id, options))
    }
    .await;
    match result {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

pub async fn step_up_passkey_login(
    State(state): State<AppState>,
    SessionUser(user): SessionUser,
    Json(body): Json<FlowCredentialBody>,
) -> impl IntoResponse {
    let result: Result<axum::response::Response, AuthError> = async {
        let pepper = secret_bytes(&state, AUTH_HASH_PEPPER, 32)?;
        let ceremony = PasskeyCeremony::from_app(&state)?;
        let grant = finish_step_up_passkey_login(
            &state.db,
            &pepper,
            &ceremony,
            user.id,
            &body.flow_id,
            &body.credential,
        )
        .await?;
        Ok(step_up_response(&grant.token, grant.expires_in))
    }
    .await;
    match result {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

pub async fn logout(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    if let Some(token) = presented_token(&headers) {
        if let Ok(pepper) = secret_bytes(&state, AUTH_HASH_PEPPER, 32) {
            let _ = logout_current(&state.db, &pepper, &token).await;
        }
    }
    let mut response = StatusCode::NO_CONTENT.into_response();
    response
        .headers_mut()
        .insert(header::SET_COOKIE, clear_session_cookie_header());
    response
}

pub async fn logout_all(
    State(state): State<AppState>,
    SessionUser(user): SessionUser,
) -> impl IntoResponse {
    match logout_all_for_user(&state.db, user.id).await {
        Ok(()) => {
            let mut response = StatusCode::NO_CONTENT.into_response();
            response
                .headers_mut()
                .insert(header::SET_COOKIE, clear_session_cookie_header());
            response
        }
        Err(err) => err.into_response(),
    }
}

pub async fn me(SessionUser(user): SessionUser) -> impl IntoResponse {
    Json(AccountView::from(&user)).into_response()
}

pub async fn list_sessions(
    State(state): State<AppState>,
    headers: HeaderMap,
    SessionUser(user): SessionUser,
) -> impl IntoResponse {
    let result: Result<axum::response::Response, AuthError> = async {
        let pepper = secret_bytes(&state, AUTH_HASH_PEPPER, 32)?;
        let current_hash =
            presented_token(&headers).and_then(|token| token_hash(&pepper, &token).ok());
        let sessions = list_session_info(&state.db, user.id, current_hash.as_deref()).await?;
        Ok(Json(json!({ "sessions": sessions })).into_response())
    }
    .await;
    match result {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

/// Revoke one of the signed-in user's sessions. Revoking the session that
/// presented this request clears its cookie.
pub async fn revoke_listed_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    SessionUser(user): SessionUser,
    Path(public_id): Path<Uuid>,
) -> impl IntoResponse {
    let result: Result<axum::response::Response, AuthError> = async {
        let pepper = secret_bytes(&state, AUTH_HASH_PEPPER, 32)?;
        let current_hash =
            presented_token(&headers).and_then(|token| token_hash(&pepper, &token).ok());
        let current =
            revoke_session_for_user(&state.db, user.id, public_id, current_hash.as_deref()).await?;
        let mut response = StatusCode::NO_CONTENT.into_response();
        if current {
            response
                .headers_mut()
                .insert(header::SET_COOKIE, clear_session_cookie_header());
        }
        Ok(response)
    }
    .await;
    match result {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct MintSessionBody {
    pub os: String,
    pub os_version: String,
}

fn cli_user_agent(os: &str, os_version: &str) -> Result<String, AuthError> {
    let os = os.trim();
    let os_version = os_version.trim();
    if os.is_empty()
        || os_version.is_empty()
        || os.chars().count() > 64
        || os_version.chars().count() > 64
        || os.chars().any(char::is_control)
        || os_version.chars().any(char::is_control)
    {
        return Err(AuthError::TypeMismatch);
    }
    Ok(format!("cli {os} {os_version}"))
}

/// Mint a bearer session for a local command-line tool. Does not replace the
/// browser session cookie.
pub async fn mint_session(
    State(state): State<AppState>,
    SessionUser(user): SessionUser,
    Json(body): Json<MintSessionBody>,
) -> impl IntoResponse {
    let result: Result<axum::response::Response, AuthError> = async {
        let user_agent = cli_user_agent(&body.os, &body.os_version)?;
        let pepper = secret_bytes(&state, AUTH_HASH_PEPPER, 32)?;
        let grant = mint_cli_session(&state.db, &pepper, &user, &user_agent).await?;
        Ok(Json(session_json(&grant)).into_response())
    }
    .await;
    match result {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct SetPasswordBody {
    pub password: String,
}

/// Set or replace the signed-in user's password. Does not ask for the current
/// password and does not revoke sessions.
pub async fn set_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    SessionUser(user): SessionUser,
    Json(mut body): Json<SetPasswordBody>,
) -> impl IntoResponse {
    let result: Result<axum::response::Response, AuthError> = async {
        require_step_up(&state, &headers, user.id).await?;
        if let Err(err) = check_password_policy(&body.password) {
            body.password.zeroize();
            return Err(err.into());
        }
        let password_hasher = hasher(&state)?;
        let password_hash = match password_hasher.hash(&body.password) {
            Ok(hash) => hash,
            Err(err) => {
                body.password.zeroize();
                return Err(err.into());
            }
        };
        body.password.zeroize();
        let updated = set_session_password(&state.db, user.id, password_hash).await?;
        Ok(Json(AccountView::from(&updated)).into_response())
    }
    .await;
    match result {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

/// Remove the password when at least one passkey remains.
pub async fn clear_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    SessionUser(user): SessionUser,
) -> impl IntoResponse {
    let result = async {
        require_step_up(&state, &headers, user.id).await?;
        clear_session_password(&state.db, user.id).await
    }
    .await;
    match result {
        Ok(updated) => Json(AccountView::from(&updated)).into_response(),
        Err(err) => err.into_response(),
    }
}

pub async fn request_reset(
    State(state): State<AppState>,
    Json(body): Json<RequestResetBody>,
) -> impl IntoResponse {
    let email = body.email.as_ref().to_owned();
    tokio::spawn(async move {
        let Ok(pepper) = secret_bytes(&state, AUTH_HASH_PEPPER, 32) else {
            eprintln!("recovery: missing AUTH_HASH_PEPPER");
            return;
        };
        match request_recovery(&state.db, &pepper, &email, RECOVERY_TTL).await {
            Ok(Some(raw)) => {
                let origin = state
                    .secret(AUTH_WEBAUTHN_RP_ORIGIN)
                    .unwrap_or_else(|_| DEFAULT_WEBAUTHN_RP_ORIGIN.to_string());
                let link = format!("{}/?recover={raw}", origin.trim_end_matches('/'));
                let mail_body = format!(
                    "Use this link to set a new credential for your rundtisch account:\n\n{link}\n"
                );
                if let Err(err) = state
                    .email
                    .send(&email, "Recover your rundtisch account", &mail_body)
                    .await
                {
                    eprintln!("recovery email failed: {err}");
                }
            }
            Ok(None) => {}
            Err(err) => eprintln!("recovery request failed: {err}"),
        }
    });
    (
        StatusCode::ACCEPTED,
        Json(json!({
            "message": "If that account exists, a recovery link has been sent."
        })),
    )
}

pub async fn reset_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut body): Json<ResetPasswordBody>,
) -> impl IntoResponse {
    let result: Result<axum::response::Response, AuthError> = async {
        if let Err(err) = check_password_policy(&body.password) {
            body.password.zeroize();
            return Err(err.into());
        }
        let password_hasher = hasher(&state)?;
        let password_hash = match password_hasher.hash(&body.password) {
            Ok(hash) => hash,
            Err(err) => {
                body.password.zeroize();
                return Err(err.into());
            }
        };
        body.password.zeroize();
        let pepper = secret_bytes(&state, AUTH_HASH_PEPPER, 32)?;
        let grant = complete_password_recovery(
            &state.db,
            &pepper,
            &body.token,
            password_hash,
            cap_user_agent(&headers).as_deref(),
        )
        .await?;
        Ok(session_response(StatusCode::OK, grant))
    }
    .await;
    match result {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

pub async fn reset_passkey_options(
    State(state): State<AppState>,
    Json(body): Json<RecoveryTokenBody>,
) -> impl IntoResponse {
    let result: Result<axum::response::Response, AuthError> = async {
        let passkey_label = normalize_passkey_label(body.label)?;
        let pepper = secret_bytes(&state, AUTH_HASH_PEPPER, 32)?;
        let ceremony = PasskeyCeremony::from_app(&state)?;
        let (flow_id, options) =
            start_recovery_passkey(&state.db, &pepper, &ceremony, &body.token, passkey_label)
                .await?;
        Ok(ceremony_response(flow_id, options))
    }
    .await;
    match result {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

pub async fn reset_passkey(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<FlowCredentialBody>,
) -> impl IntoResponse {
    let result: Result<axum::response::Response, AuthError> = async {
        let pepper = secret_bytes(&state, AUTH_HASH_PEPPER, 32)?;
        let ceremony = PasskeyCeremony::from_app(&state)?;
        let grant = finish_recovery_passkey(
            &state.db,
            &pepper,
            &ceremony,
            &body.flow_id,
            &body.credential,
            cap_user_agent(&headers).as_deref(),
        )
        .await?;
        Ok(session_response(StatusCode::OK, grant))
    }
    .await;
    match result {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

pub async fn list_passkeys(
    State(state): State<AppState>,
    SessionUser(user): SessionUser,
) -> impl IntoResponse {
    match list_passkey_info(&state.db, user.id).await {
        Ok(passkeys) => Json(json!({"passkeys": passkeys})).into_response(),
        Err(err) => err.into_response(),
    }
}

pub async fn passkey_register_options(
    State(state): State<AppState>,
    headers: HeaderMap,
    SessionUser(user): SessionUser,
    Json(body): Json<PasskeyOptionsBody>,
) -> impl IntoResponse {
    let result: Result<axum::response::Response, AuthError> = async {
        require_step_up(&state, &headers, user.id).await?;
        let passkey_label = normalize_passkey_label(body.label)?;
        let ceremony = PasskeyCeremony::from_app(&state)?;
        let (flow_id, options) =
            start_session_passkey(&state.db, &ceremony, &user, passkey_label).await?;
        Ok(ceremony_response(flow_id, options))
    }
    .await;
    match result {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

pub async fn passkey_register(
    State(state): State<AppState>,
    headers: HeaderMap,
    SessionUser(user): SessionUser,
    Json(body): Json<FlowCredentialBody>,
) -> impl IntoResponse {
    let result: Result<axum::response::Response, AuthError> = async {
        require_step_up(&state, &headers, user.id).await?;
        let ceremony = PasskeyCeremony::from_app(&state)?;
        let info = finish_session_passkey(
            &state.db,
            &ceremony,
            user.id,
            &body.flow_id,
            &body.credential,
        )
        .await?;
        Ok((StatusCode::CREATED, Json(json!({"passkey": info}))).into_response())
    }
    .await;
    match result {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

pub async fn passkey_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    SessionUser(user): SessionUser,
    Path(public_id): Path<Uuid>,
) -> impl IntoResponse {
    let result = async {
        require_step_up(&state, &headers, user.id).await?;
        delete_passkey(&state.db, user.id, public_id).await
    }
    .await;
    match result {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => err.into_response(),
    }
}

/// Operator helper used by the demo `auth-link` binary. Not an HTTP handler.
pub async fn mint_invitation_link(
    db: &sea_orm::DatabaseConnection,
    pepper: &[u8],
    email: &str,
    ttl: time::Duration,
) -> Result<String, AuthError> {
    crate::auth::services::mint_invitation(db, pepper, email, ttl).await
}

pub async fn mint_recovery_link(
    db: &sea_orm::DatabaseConnection,
    pepper: &[u8],
    email: &str,
    ttl: time::Duration,
) -> Result<Option<String>, AuthError> {
    create_recovery_token(db, pepper, email, ttl).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::config::{AUTH_WEBAUTHN_RP_ID, AUTH_WEBAUTHN_RP_ORIGIN};
    use crate::auth::entities::recovery_token;
    use crate::auth::migrations::Migrator;
    use crate::auth::services::{create_recovery_token, mint_invitation};
    use crate::email::RecordingEmailSender;
    use axum::body::Body;
    use axum::http::{Request, StatusCode as HttpStatus};
    use sea_orm::{EntityTrait, PaginatorTrait};
    use sea_orm_migration::MigratorTrait;
    use serde_json::json;
    use std::sync::Arc;
    use time::Duration;
    use tower::ServiceExt;
    use webauthn_authenticator_rs::WebauthnAuthenticator;
    use webauthn_authenticator_rs::softpasskey::SoftPasskey;
    use webauthn_rs::prelude::{
        CreationChallengeResponse, PublicKeyCredential, RequestChallengeResponse, Url,
    };

    const PEPPER: &str = "cccccccccccccccccccccccccccccccc";

    async fn app() -> (
        axum::Router,
        sea_orm::DatabaseConnection,
        RecordingEmailSender,
    ) {
        unsafe {
            std::env::set_var(AUTH_HASH_PEPPER, PEPPER);
            std::env::set_var(AUTH_WEBAUTHN_RP_ID, "localhost");
            std::env::set_var(AUTH_WEBAUTHN_RP_ORIGIN, "http://localhost:5173");
            std::env::set_var("RUNDTISCH_TEST_PASSWORD_HASHER", "1");
        }
        let mut opts = sea_orm::ConnectOptions::new("sqlite::memory:");
        opts.max_connections(1);
        let db = sea_orm::Database::connect(opts).await.expect("sqlite");
        Migrator::up(&db, None).await.expect("migrate");
        let email = RecordingEmailSender::new();
        let state = AppState {
            db: db.clone(),
            email: Arc::new(email.clone()),
        };
        let router = axum::Router::new()
            .route(
                "/api/auth/register_with_token",
                axum::routing::post(register_with_token),
            )
            .route(
                "/api/auth/register/password",
                axum::routing::post(register_password),
            )
            .route(
                "/api/auth/register/passkey/options",
                axum::routing::post(register_passkey_options),
            )
            .route(
                "/api/auth/register/passkey",
                axum::routing::post(register_passkey),
            )
            .route("/api/auth/login", axum::routing::post(login))
            .route(
                "/api/auth/passkeys/login/options",
                axum::routing::post(passkey_login_options),
            )
            .route(
                "/api/auth/passkeys/login",
                axum::routing::post(passkey_login),
            )
            .route(
                "/api/auth/step-up/login",
                axum::routing::post(step_up_login),
            )
            .route(
                "/api/auth/step-up/passkeys/login/options",
                axum::routing::post(step_up_passkey_login_options),
            )
            .route(
                "/api/auth/step-up/passkeys/login",
                axum::routing::post(step_up_passkey_login),
            )
            .route("/api/auth/logout", axum::routing::post(logout))
            .route("/api/auth/logout_all", axum::routing::post(logout_all))
            .route("/api/auth/me", axum::routing::get(me))
            .route(
                "/api/auth/sessions",
                axum::routing::get(list_sessions).post(mint_session),
            )
            .route(
                "/api/auth/sessions/{public_id}",
                axum::routing::delete(revoke_listed_session),
            )
            .route(
                "/api/auth/password",
                axum::routing::put(set_password).delete(clear_password),
            )
            .route(
                "/api/auth/request_reset",
                axum::routing::post(request_reset),
            )
            .route("/api/auth/reset", axum::routing::post(reset_password))
            .route(
                "/api/auth/reset/passkey/options",
                axum::routing::post(reset_passkey_options),
            )
            .route(
                "/api/auth/reset/passkey",
                axum::routing::post(reset_passkey),
            )
            .route("/api/auth/passkeys", axum::routing::get(list_passkeys))
            .route(
                "/api/auth/passkeys/register/options",
                axum::routing::post(passkey_register_options),
            )
            .route(
                "/api/auth/passkeys/register",
                axum::routing::post(passkey_register),
            )
            .route(
                "/api/auth/passkeys/{public_id}",
                axum::routing::delete(passkey_delete),
            )
            .with_state(state);
        (router, db, email)
    }

    async fn body_json(response: axum::http::Response<Body>) -> (HttpStatus, serde_json::Value) {
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        if bytes.is_empty() {
            return (status, serde_json::Value::Null);
        }
        let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
        (status, json)
    }

    fn cookie_pair(response: &axum::http::Response<Body>) -> String {
        response
            .headers()
            .get(header::SET_COOKIE)
            .expect("set-cookie")
            .to_str()
            .expect("cookie")
            .split(';')
            .next()
            .expect("cookie pair")
            .to_string()
    }

    async fn post_json(
        app: &axum::Router,
        path: &str,
        body: serde_json::Value,
        cookie: Option<&str>,
        bearer: Option<&str>,
    ) -> axum::http::Response<Body> {
        let mut req = Request::post(path).header("content-type", "application/json");
        if let Some(cookie) = cookie {
            req = req.header("cookie", cookie);
        }
        if let Some(bearer) = bearer {
            req = req.header("authorization", format!("Bearer {bearer}"));
        }
        app.clone()
            .oneshot(req.body(Body::from(body.to_string())).unwrap())
            .await
            .unwrap()
    }

    async fn post_json_step(
        app: &axum::Router,
        path: &str,
        body: serde_json::Value,
        cookie: &str,
        step_up: &str,
    ) -> axum::http::Response<Body> {
        app.clone()
            .oneshot(
                Request::post(path)
                    .header("content-type", "application/json")
                    .header("cookie", cookie)
                    .header("x-step-up-token", step_up)
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    async fn password_step_up(
        app: &axum::Router,
        cookie: &str,
        email: &str,
        password: &str,
    ) -> String {
        let response = post_json(
            app,
            "/api/auth/step-up/login",
            json!({"email": email, "password": password}),
            Some(cookie),
            None,
        )
        .await;
        assert_eq!(response.status(), HttpStatus::OK, "step-up login");
        response
            .headers()
            .get("x-step-up-token")
            .expect("step-up header")
            .to_str()
            .unwrap()
            .to_string()
    }

    async fn invite(db: &sea_orm::DatabaseConnection, email: &str) -> String {
        mint_invitation(db, PEPPER.as_bytes(), email, Duration::hours(24))
            .await
            .expect("invite")
    }

    fn authenticator() -> SoftPasskey {
        SoftPasskey::new(true)
    }

    fn origin() -> Url {
        Url::parse("http://localhost:5173").unwrap()
    }

    #[test]
    fn passkey_labels_are_optional_trimmed_and_limited() {
        assert_eq!(normalize_passkey_label(None).unwrap(), None);
        assert_eq!(normalize_passkey_label(Some("   ".into())).unwrap(), None);
        assert_eq!(
            normalize_passkey_label(Some("  MacBook Touch ID  ".into())).unwrap(),
            Some("MacBook Touch ID".into())
        );
        assert!(normalize_passkey_label(Some("x".repeat(65))).is_err());
    }

    /// SoftPasskey rejects requireResidentKey. The server still advertises it;
    /// the test authenticator only clears the flag on its local copy.
    fn register_soft(
        authenticator: &mut SoftPasskey,
        mut options: CreationChallengeResponse,
    ) -> webauthn_rs::prelude::RegisterPublicKeyCredential {
        if let Some(selection) = options.public_key.authenticator_selection.as_mut() {
            selection.require_resident_key = false;
        }
        authenticator
            .do_registration(origin(), options)
            .expect("soft register")
    }

    fn assert_resident_key_required(json: &serde_json::Value) {
        let selection = &json["options"]["publicKey"]["authenticatorSelection"];
        assert_eq!(selection["residentKey"], "required");
        assert_eq!(selection["requireResidentKey"], true);
    }

    fn assert_direct_attestation(json: &serde_json::Value) {
        assert_eq!(json["options"]["publicKey"]["attestation"], "direct");
    }

    fn assert_discoverable_request(json: &serde_json::Value) {
        let allow = &json["options"]["publicKey"]["allowCredentials"];
        assert!(
            allow.is_null() || allow.as_array().is_some_and(|items| items.is_empty()),
            "{allow}"
        );
    }

    /// SoftPasskey needs an allow list and does not return a user handle.
    /// The signature does not cover userHandle, so attach the account public_id
    /// after signing to exercise discoverable verification.
    fn login_soft(
        authenticator: &mut SoftPasskey,
        request: RequestChallengeResponse,
        credential_id: &str,
        public_id: Uuid,
    ) -> PublicKeyCredential {
        let mut request = serde_json::to_value(&request).expect("request json");
        request["publicKey"]["allowCredentials"] = json!([{
            "type": "public-key",
            "id": credential_id,
        }]);
        let request: RequestChallengeResponse =
            serde_json::from_value(request).expect("request with allow list");
        let mut assertion = authenticator
            .do_authentication(origin(), request)
            .expect("soft login");
        assertion.response.user_handle = Some(public_id.as_bytes().to_vec());
        assertion
    }

    #[tokio::test]
    async fn password_register_login_me_logout_and_replay() {
        let (app, db, _email) = app().await;
        let token = invite(&db, "carol@example.com").await;
        let registered = post_json(
            &app,
            "/api/auth/register/password",
            json!({
                "token": token,
                "password": "unique-passphrase-ok",
                "alias": "carol"
            }),
            None,
            None,
        )
        .await;
        let cookie = cookie_pair(&registered);
        let (status, json) = body_json(registered).await;
        assert_eq!(status, HttpStatus::CREATED, "{json}");
        let bearer = json["token"].as_str().unwrap().to_string();
        assert_eq!(json["user"]["email"], "carol@example.com");
        assert_eq!(json["user"]["has_password"], true);

        let replay = post_json(
            &app,
            "/api/auth/register_with_token",
            json!({
                "token": token,
                "password": "unique-passphrase-ok"
            }),
            None,
            None,
        )
        .await;
        assert_eq!(replay.status(), HttpStatus::UNAUTHORIZED);

        let me = app
            .clone()
            .oneshot(
                Request::get("/api/auth/me")
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, me_json) = body_json(me).await;
        assert_eq!(status, HttpStatus::OK, "{me_json}");
        assert_eq!(me_json["alias"], "carol");

        let me_bearer = app
            .clone()
            .oneshot(
                Request::get("/api/auth/me")
                    .header("authorization", format!("Bearer {bearer}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(me_bearer.status(), HttpStatus::OK);

        let logout = app
            .clone()
            .oneshot(
                Request::post("/api/auth/logout")
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(logout.status(), HttpStatus::NO_CONTENT);
        let after = app
            .oneshot(
                Request::get("/api/auth/me")
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(after.status(), HttpStatus::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn concurrent_invitation_consumption_allows_one_account() {
        let (app, db, _email) = app().await;
        let token = invite(&db, "race@example.com").await;
        let body = json!({
            "token": token,
            "password": "unique-passphrase-ok",
            "alias": "race"
        });
        let (left, right) = tokio::join!(
            post_json(
                &app,
                "/api/auth/register/password",
                body.clone(),
                None,
                None
            ),
            post_json(&app, "/api/auth/register/password", body, None, None),
        );
        let statuses = [left.status(), right.status()];
        assert!(statuses.contains(&HttpStatus::CREATED), "{statuses:?}");
        assert!(
            statuses.contains(&HttpStatus::UNAUTHORIZED)
                || statuses.contains(&HttpStatus::CONFLICT),
            "{statuses:?}"
        );
        let ok = statuses
            .iter()
            .filter(|s| **s == HttpStatus::CREATED)
            .count();
        assert_eq!(ok, 1, "{statuses:?}");
    }

    #[tokio::test]
    async fn login_rejects_unknown_user_and_missing_password() {
        let (app, _, _email) = app().await;
        let response = post_json(
            &app,
            "/api/auth/login",
            json!({"email": "missing@example.com", "password": "unique-passphrase-ok"}),
            None,
            None,
        )
        .await;
        let (status, json) = body_json(response).await;
        assert_eq!(status, HttpStatus::UNAUTHORIZED);
        assert_eq!(json["error"], "invalid_credentials");
    }

    #[tokio::test]
    async fn cli_session_is_a_second_bearer_and_keeps_the_browser_cookie() {
        let (app, db, _email) = app().await;
        let token = invite(&db, "cli@example.com").await;
        let registered = post_json(
            &app,
            "/api/auth/register/password",
            json!({
                "token": token,
                "password": "unique-passphrase-ok",
                "alias": "cli"
            }),
            None,
            None,
        )
        .await;
        let cookie = cookie_pair(&registered);
        assert!(registered.headers().get(header::SET_COOKIE).is_some());

        let anonymous = post_json(
            &app,
            "/api/auth/sessions",
            json!({"os": "linux", "os_version": "6.8"}),
            None,
            None,
        )
        .await;
        assert_eq!(anonymous.status(), HttpStatus::UNAUTHORIZED);

        let minted = post_json(
            &app,
            "/api/auth/sessions",
            json!({"os": "linux", "os_version": "6.8"}),
            Some(&cookie),
            None,
        )
        .await;
        assert!(minted.headers().get(header::SET_COOKIE).is_none());
        let (status, json) = body_json(minted).await;
        assert_eq!(status, HttpStatus::OK, "{json}");
        assert_eq!(json["token_type"], "Bearer");
        assert!(json["expires_in"].as_i64().unwrap() > 60 * 60 * 24);
        let cli_token = json["token"].as_str().unwrap().to_string();

        let browser = app
            .clone()
            .oneshot(
                Request::get("/api/auth/me")
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(browser.status(), HttpStatus::OK);

        let listed = app
            .clone()
            .oneshot(
                Request::get("/api/auth/sessions")
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, listed_json) = body_json(listed).await;
        assert_eq!(status, HttpStatus::OK, "{listed_json}");
        let sessions = listed_json["sessions"].as_array().unwrap();
        assert_eq!(sessions.len(), 2, "{listed_json}");
        assert!(
            sessions
                .iter()
                .any(|row| row["current"] == true && row["user_agent"].is_null())
        );
        assert!(
            sessions
                .iter()
                .any(|row| { row["current"] == false && row["user_agent"] == "cli linux 6.8" })
        );
        assert!(listed_json["sessions"][0].get("token_hash").is_none());
        assert!(listed_json["sessions"][0].get("id").is_none());
        let cli_public_id = sessions
            .iter()
            .find(|row| row["user_agent"] == "cli linux 6.8")
            .unwrap()["public_id"]
            .as_str()
            .unwrap()
            .to_string();
        let current_public_id =
            sessions.iter().find(|row| row["current"] == true).unwrap()["public_id"]
                .as_str()
                .unwrap()
                .to_string();
        assert_ne!(cli_public_id, current_public_id);
        Uuid::parse_str(&cli_public_id).unwrap();

        let cli = app
            .clone()
            .oneshot(
                Request::get("/api/auth/me")
                    .header("authorization", format!("Bearer {cli_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, me) = body_json(cli).await;
        assert_eq!(status, HttpStatus::OK, "{me}");
        assert_eq!(me["email"], "cli@example.com");

        let bad = post_json(
            &app,
            "/api/auth/sessions",
            json!({"os": " ", "os_version": "6.8"}),
            Some(&cookie),
            None,
        )
        .await;
        assert_eq!(bad.status(), HttpStatus::BAD_REQUEST);

        let other_token = invite(&db, "other@example.com").await;
        let other = post_json(
            &app,
            "/api/auth/register/password",
            json!({
                "token": other_token,
                "password": "unique-passphrase-ok",
                "alias": "other"
            }),
            None,
            None,
        )
        .await;
        let other_cookie = cookie_pair(&other);
        let foreign = app
            .clone()
            .oneshot(
                Request::delete(format!("/api/auth/sessions/{cli_public_id}"))
                    .header("cookie", &other_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(foreign.status(), HttpStatus::NOT_FOUND);

        let missing = app
            .clone()
            .oneshot(
                Request::delete("/api/auth/sessions/00000000-0000-4000-8000-000000000099")
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(missing.status(), HttpStatus::NOT_FOUND);

        let revoked = app
            .clone()
            .oneshot(
                Request::delete(format!("/api/auth/sessions/{cli_public_id}"))
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(revoked.status(), HttpStatus::NO_CONTENT);
        assert!(revoked.headers().get(header::SET_COOKIE).is_none());

        let cli_after = app
            .clone()
            .oneshot(
                Request::get("/api/auth/me")
                    .header("authorization", format!("Bearer {cli_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(cli_after.status(), HttpStatus::UNAUTHORIZED);

        let listed_after = app
            .clone()
            .oneshot(
                Request::get("/api/auth/sessions")
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, listed_after_json) = body_json(listed_after).await;
        assert_eq!(status, HttpStatus::OK, "{listed_after_json}");
        let remaining = listed_after_json["sessions"].as_array().unwrap();
        assert_eq!(remaining.len(), 1, "{listed_after_json}");
        assert_eq!(remaining[0]["public_id"], current_public_id);
        assert_eq!(remaining[0]["current"], true);

        let again = app
            .clone()
            .oneshot(
                Request::delete(format!("/api/auth/sessions/{cli_public_id}"))
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(again.status(), HttpStatus::NOT_FOUND);

        let current = app
            .clone()
            .oneshot(
                Request::delete(format!("/api/auth/sessions/{current_public_id}"))
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(current.status(), HttpStatus::NO_CONTENT);
        assert!(current.headers().get(header::SET_COOKIE).is_some());

        let browser_after = app
            .clone()
            .oneshot(
                Request::get("/api/auth/me")
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(browser_after.status(), HttpStatus::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn request_reset_does_not_enumerate_or_return_token() {
        let (app, db, email) = app().await;
        let token = invite(&db, "ada@example.com").await;
        post_json(
            &app,
            "/api/auth/register/password",
            json!({"token": token, "password": "unique-passphrase-ok"}),
            None,
            None,
        )
        .await;

        let known = post_json(
            &app,
            "/api/auth/request_reset",
            json!({"email": "ada@example.com"}),
            None,
            None,
        )
        .await;
        let (status, known_json) = body_json(known).await;
        let missing = post_json(
            &app,
            "/api/auth/request_reset",
            json!({"email": "nope@example.com"}),
            None,
            None,
        )
        .await;
        let (missing_status, missing_json) = body_json(missing).await;
        assert_eq!(status, HttpStatus::ACCEPTED);
        assert_eq!(missing_status, status);
        assert_eq!(known_json, missing_json);
        assert!(known_json.get("token").is_none());

        let mut count = 0;
        let mut messages = Vec::new();
        for _ in 0..200 {
            count = recovery_token::Entity::find().count(&db).await.unwrap();
            messages = email.messages();
            if count == 1 && messages.len() == 1 {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert_eq!(count, 1);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].to, "ada@example.com");
        assert_eq!(messages[0].subject, "Recover your rundtisch account");
        assert!(
            messages[0]
                .body
                .contains("http://localhost:5173/?recover="),
            "recovery email should include a redeem link: {}",
            messages[0].body
        );
    }

    #[tokio::test]
    async fn password_recovery_revokes_old_sessions() {
        let (app, db, _email) = app().await;
        let invite_token = invite(&db, "ada@example.com").await;
        let registered = post_json(
            &app,
            "/api/auth/register/password",
            json!({"token": invite_token, "password": "unique-passphrase-ok"}),
            None,
            None,
        )
        .await;
        let old_cookie = cookie_pair(&registered);
        let recovery = create_recovery_token(
            &db,
            PEPPER.as_bytes(),
            "ada@example.com",
            Duration::hours(1),
        )
        .await
        .unwrap()
        .unwrap();
        let reset = post_json(
            &app,
            "/api/auth/reset",
            json!({"token": recovery, "password": "another-passphrase-ok"}),
            None,
            None,
        )
        .await;
        let (status, json) = body_json(reset).await;
        assert_eq!(status, HttpStatus::OK, "{json}");
        assert_eq!(json["user"]["has_password"], true);
        let old = app
            .clone()
            .oneshot(
                Request::get("/api/auth/me")
                    .header("cookie", old_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(old.status(), HttpStatus::UNAUTHORIZED);
        let login = post_json(
            &app,
            "/api/auth/login",
            json!({"email": "ada@example.com", "password": "another-passphrase-ok"}),
            None,
            None,
        )
        .await;
        assert_eq!(login.status(), HttpStatus::OK);
    }

    #[tokio::test]
    async fn session_password_keeps_the_session_and_refuses_the_last_credential() {
        let (app, db, _email) = app().await;
        let token = invite(&db, "pw@example.com").await;
        let registered = post_json(
            &app,
            "/api/auth/register/password",
            json!({
                "token": token,
                "password": "unique-passphrase-ok",
                "alias": "pw"
            }),
            None,
            None,
        )
        .await;
        let cookie = cookie_pair(&registered);
        let (status, json) = body_json(registered).await;
        assert_eq!(status, HttpStatus::CREATED, "{json}");

        let missing = app
            .clone()
            .oneshot(
                Request::delete("/api/auth/password")
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, err) = body_json(missing).await;
        assert_eq!(status, HttpStatus::FORBIDDEN, "{err}");
        assert_eq!(err["error"], "step_up_required");

        let step = password_step_up(
            &app,
            &cookie,
            "pw@example.com",
            "unique-passphrase-ok",
        )
        .await;

        let removed = app
            .clone()
            .oneshot(
                Request::delete("/api/auth/password")
                    .header("cookie", &cookie)
                    .header("x-step-up-token", &step)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, err) = body_json(removed).await;
        assert_eq!(status, HttpStatus::CONFLICT, "{err}");
        assert_eq!(err["error"], "last_credential");

        let changed = app
            .clone()
            .oneshot(
                Request::put("/api/auth/password")
                    .header("cookie", &cookie)
                    .header("x-step-up-token", &step)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({"password": "another-passphrase-ok"}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, json) = body_json(changed).await;
        assert_eq!(status, HttpStatus::OK, "{json}");
        assert_eq!(json["has_password"], true);

        let me = app
            .clone()
            .oneshot(
                Request::get("/api/auth/me")
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(me.status(), HttpStatus::OK);

        let old_login = post_json(
            &app,
            "/api/auth/login",
            json!({
                "email": "pw@example.com",
                "password": "unique-passphrase-ok"
            }),
            None,
            None,
        )
        .await;
        assert_eq!(old_login.status(), HttpStatus::UNAUTHORIZED);
        let new_login = post_json(
            &app,
            "/api/auth/login",
            json!({
                "email": "pw@example.com",
                "password": "another-passphrase-ok"
            }),
            None,
            None,
        )
        .await;
        assert_eq!(new_login.status(), HttpStatus::OK);

        let mut authenticator = authenticator();
        let started = post_json_step(
            &app,
            "/api/auth/passkeys/register/options",
            json!({"label": "Phone"}),
            &cookie,
            &step,
        )
        .await;
        let (status, options_json) = body_json(started).await;
        assert_eq!(status, HttpStatus::OK, "{options_json}");
        let flow_id = options_json["flow_id"].as_str().unwrap().to_string();
        let options: CreationChallengeResponse =
            serde_json::from_value(options_json["options"].clone()).expect("creation options");
        let credential = register_soft(&mut authenticator, options);
        let finished = post_json_step(
            &app,
            "/api/auth/passkeys/register",
            json!({"flow_id": flow_id, "credential": credential}),
            &cookie,
            &step,
        )
        .await;
        let (status, created_json) = body_json(finished).await;
        assert_eq!(status, HttpStatus::CREATED, "{created_json}");
        let passkey_id = created_json["passkey"]["public_id"]
            .as_str()
            .unwrap()
            .to_string();

        let deleted_passkey = app
            .clone()
            .oneshot(
                Request::delete(format!("/api/auth/passkeys/{passkey_id}"))
                    .header("cookie", &cookie)
                    .header("x-step-up-token", &step)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(deleted_passkey.status(), HttpStatus::NO_CONTENT);

        let still_blocked = app
            .clone()
            .oneshot(
                Request::delete("/api/auth/password")
                    .header("cookie", &cookie)
                    .header("x-step-up-token", &step)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, err) = body_json(still_blocked).await;
        assert_eq!(status, HttpStatus::CONFLICT, "{err}");
        assert_eq!(err["error"], "last_credential");

        let mut authenticator = crate::auth::handlers::tests::authenticator();
        let started = post_json_step(
            &app,
            "/api/auth/passkeys/register/options",
            json!({}),
            &cookie,
            &step,
        )
        .await;
        let (status, options_json) = body_json(started).await;
        assert_eq!(status, HttpStatus::OK, "{options_json}");
        let flow_id = options_json["flow_id"].as_str().unwrap().to_string();
        let options: CreationChallengeResponse =
            serde_json::from_value(options_json["options"].clone()).expect("creation options");
        let credential = register_soft(&mut authenticator, options);
        let finished = post_json_step(
            &app,
            "/api/auth/passkeys/register",
            json!({"flow_id": flow_id, "credential": credential}),
            &cookie,
            &step,
        )
        .await;
        assert_eq!(finished.status(), HttpStatus::CREATED);

        let cleared = app
            .clone()
            .oneshot(
                Request::delete("/api/auth/password")
                    .header("cookie", &cookie)
                    .header("x-step-up-token", &step)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, json) = body_json(cleared).await;
        assert_eq!(status, HttpStatus::OK, "{json}");
        assert_eq!(json["has_password"], false);

        let me = app
            .clone()
            .oneshot(
                Request::get("/api/auth/me")
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, me_json) = body_json(me).await;
        assert_eq!(status, HttpStatus::OK, "{me_json}");
        assert_eq!(me_json["has_password"], false);

        let password_login = post_json(
            &app,
            "/api/auth/login",
            json!({
                "email": "pw@example.com",
                "password": "another-passphrase-ok"
            }),
            None,
            None,
        )
        .await;
        assert_eq!(password_login.status(), HttpStatus::UNAUTHORIZED);

        let listed = app
            .clone()
            .oneshot(
                Request::get("/api/auth/passkeys")
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, listed_json) = body_json(listed).await;
        assert_eq!(status, HttpStatus::OK, "{listed_json}");
        let passkey_id = listed_json["passkeys"][0]["public_id"].as_str().unwrap();
        let last = app
            .oneshot(
                Request::delete(format!("/api/auth/passkeys/{passkey_id}"))
                    .header("cookie", &cookie)
                    .header("x-step-up-token", &step)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, err) = body_json(last).await;
        assert_eq!(status, HttpStatus::CONFLICT, "{err}");
        assert_eq!(err["error"], "last_credential");
    }

    #[tokio::test]
    async fn passkey_register_login_and_last_credential_guard() {
        let (app, db, _email) = app().await;
        let token = invite(&db, "pk@example.com").await;
        let mut authenticator = authenticator();
        let started = post_json(
            &app,
            "/api/auth/register/passkey/options",
            json!({"token": token, "alias": "pk", "label": "Security key"}),
            None,
            None,
        )
        .await;
        let (status, json) = body_json(started).await;
        assert_eq!(status, HttpStatus::OK, "{json}");
        assert_resident_key_required(&json);
        assert_direct_attestation(&json);
        let flow_id = json["flow_id"].as_str().unwrap().to_string();
        let options: CreationChallengeResponse =
            serde_json::from_value(json["options"].clone()).expect("creation options");
        let credential = register_soft(&mut authenticator, options);
        let credential_id = credential.id.clone();
        let finished = post_json(
            &app,
            "/api/auth/register/passkey",
            json!({
                "flow_id": flow_id,
                "credential": credential,
            }),
            None,
            None,
        )
        .await;
        let cookie = cookie_pair(&finished);
        let (status, json) = body_json(finished).await;
        assert_eq!(status, HttpStatus::CREATED, "{json}");
        assert_eq!(json["user"]["has_password"], false);
        let public_id = Uuid::parse_str(json["user"]["public_id"].as_str().unwrap()).unwrap();

        let anonymous = post_json(
            &app,
            "/api/auth/passkeys/login/options",
            json!({}),
            None,
            None,
        )
        .await;
        let (status, login_json) = body_json(anonymous).await;
        assert_eq!(status, HttpStatus::OK, "{login_json}");
        assert_discoverable_request(&login_json);

        let login_started = post_json(
            &app,
            "/api/auth/passkeys/login/options",
            json!({}),
            None,
            None,
        )
        .await;
        let (status, login_json) = body_json(login_started).await;
        assert_eq!(status, HttpStatus::OK, "{login_json}");
        assert_discoverable_request(&login_json);
        let login_flow = login_json["flow_id"].as_str().unwrap().to_string();
        let request: RequestChallengeResponse =
            serde_json::from_value(login_json["options"].clone()).expect("request options");
        let assertion = login_soft(&mut authenticator, request, &credential_id, public_id);
        let logged_in = post_json(
            &app,
            "/api/auth/passkeys/login",
            json!({"flow_id": login_flow, "credential": assertion}),
            None,
            None,
        )
        .await;
        assert_eq!(logged_in.status(), HttpStatus::OK);

        let listed = app
            .clone()
            .oneshot(
                Request::get("/api/auth/passkeys")
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, listed_json) = body_json(listed).await;
        assert_eq!(status, HttpStatus::OK, "{listed_json}");
        assert_eq!(listed_json["passkeys"][0]["label"], "Security key");
        assert!(listed_json["passkeys"][0].get("id").is_none());
        // SoftPasskey puts a nil AAGUID in authData; we persist NULL for that.
        assert!(listed_json["passkeys"][0]["aaguid"].is_null());
        let step_started = post_json(
            &app,
            "/api/auth/step-up/passkeys/login/options",
            json!({}),
            Some(&cookie),
            None,
        )
        .await;
        let (status, step_json) = body_json(step_started).await;
        assert_eq!(status, HttpStatus::OK, "{step_json}");
        let step_flow = step_json["flow_id"].as_str().unwrap().to_string();
        let request: RequestChallengeResponse =
            serde_json::from_value(step_json["options"].clone()).expect("step-up options");
        let assertion = login_soft(&mut authenticator, request, &credential_id, public_id);
        let stepped = post_json(
            &app,
            "/api/auth/step-up/passkeys/login",
            json!({"flow_id": step_flow, "credential": assertion}),
            Some(&cookie),
            None,
        )
        .await;
        let step = stepped
            .headers()
            .get("x-step-up-token")
            .expect("step-up header")
            .to_str()
            .unwrap()
            .to_string();
        assert_eq!(stepped.status(), HttpStatus::OK);
        let public_id = listed_json["passkeys"][0]["public_id"].as_str().unwrap();
        let deleted = app
            .oneshot(
                Request::delete(format!("/api/auth/passkeys/{public_id}"))
                    .header("cookie", &cookie)
                    .header("x-step-up-token", &step)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, err) = body_json(deleted).await;
        assert_eq!(status, HttpStatus::CONFLICT, "{err}");
        assert_eq!(err["error"], "last_credential");
    }

    #[tokio::test]
    async fn recovery_passkey_establishes_credential_and_session() {
        let (app, db, _email) = app().await;
        let invite_token = invite(&db, "recover@example.com").await;
        post_json(
            &app,
            "/api/auth/register/password",
            json!({"token": invite_token, "password": "unique-passphrase-ok"}),
            None,
            None,
        )
        .await;
        let recovery = create_recovery_token(
            &db,
            PEPPER.as_bytes(),
            "recover@example.com",
            Duration::hours(1),
        )
        .await
        .unwrap()
        .unwrap();
        let mut authenticator = authenticator();
        let started = post_json(
            &app,
            "/api/auth/reset/passkey/options",
            json!({"token": recovery, "label": "Recovery passkey"}),
            None,
            None,
        )
        .await;
        let (status, json) = body_json(started).await;
        assert_eq!(status, HttpStatus::OK, "{json}");
        let flow_id = json["flow_id"].as_str().unwrap().to_string();
        assert_resident_key_required(&json);
        assert_direct_attestation(&json);
        let options: CreationChallengeResponse =
            serde_json::from_value(json["options"].clone()).unwrap();
        let credential = register_soft(&mut authenticator, options);
        let finished = post_json(
            &app,
            "/api/auth/reset/passkey",
            json!({"flow_id": flow_id, "credential": credential}),
            None,
            None,
        )
        .await;
        let (status, json) = body_json(finished).await;
        assert_eq!(status, HttpStatus::OK, "{json}");
        assert!(json["token"].is_string());
    }

    #[tokio::test]
    async fn logout_all_revokes_every_session() {
        let (app, db, _email) = app().await;
        let token = invite(&db, "multi@example.com").await;
        post_json(
            &app,
            "/api/auth/register/password",
            json!({"token": token, "password": "unique-passphrase-ok"}),
            None,
            None,
        )
        .await;
        let second = post_json(
            &app,
            "/api/auth/login",
            json!({"email": "multi@example.com", "password": "unique-passphrase-ok"}),
            None,
            None,
        )
        .await;
        let cookie = cookie_pair(&second);
        let (status, json) = body_json(second).await;
        assert_eq!(status, HttpStatus::OK, "{json}");
        let bearer = json["token"].as_str().unwrap().to_string();
        let third = post_json(
            &app,
            "/api/auth/login",
            json!({"email": "multi@example.com", "password": "unique-passphrase-ok"}),
            None,
            None,
        )
        .await;
        let other = cookie_pair(&third);
        let logout = app
            .clone()
            .oneshot(
                Request::post("/api/auth/logout_all")
                    .header("authorization", format!("Bearer {bearer}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(logout.status(), HttpStatus::NO_CONTENT);
        let still = app
            .oneshot(
                Request::get("/api/auth/me")
                    .header("cookie", other)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(still.status(), HttpStatus::UNAUTHORIZED);
        let _ = cookie;
    }
}
