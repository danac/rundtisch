use crate::auth::config::{CEREMONY_TTL, RECOVERY_MIN_INTERVAL, SESSION_TTL, STEP_UP_TTL};
use crate::auth::error::{AuthError, DbError};
use crate::auth::models::{
    CEREMONY_INVITE_REGISTER, CEREMONY_LOGIN, CEREMONY_RECOVERY_REGISTER,
    CEREMONY_SESSION_REGISTER, CEREMONY_STEP_UP_LOGIN, CeremonyRecord, NewUser, PasskeyInfo, Role,
    SessionGrant, SessionInfo, User, alias_from_email, new_public_id,
};
use crate::auth::password::{PasswordHasher, dummy_verify};
use crate::auth::queries::{
    clear_password_hash_by_id, consume_ceremony, consume_invitation, consume_recovery_token,
    count_passkeys_for_user, delete_expired_ceremonies, delete_passkey_by_id, find_open_invitation,
    find_open_recovery_user, get_active_session_for_user, get_ceremony,
    get_passkey_by_credential_id, get_passkey_for_user, get_session_by_token_hash,
    get_step_up_by_token_hash, get_user_by_email, get_user_by_id, insert_ceremony,
    insert_invitation, insert_passkey, insert_recovery_token, insert_session, insert_step_up,
    list_active_sessions_for_user, list_passkeys_for_user, lock_user_row, recovery_issued_since,
    revoke_all_sessions, revoke_all_step_ups, revoke_session, set_password_hash_by_id,
    touch_last_login_by_id, touch_passkey_last_used, touch_session_last_used, update_passkey_json,
};
use crate::auth::session::{credential_id_key, generate_session_token, token_hash};
use crate::auth::webauthn::{
    PasskeyCeremony, passkey_from_json, passkey_json, stored_credential_id,
};
use email_address::EmailAddress;
use sea_orm::{DatabaseConnection, TransactionTrait};
use time::OffsetDateTime;

fn now() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

fn invalid_token<T>(result: Result<T, DbError>) -> Result<T, AuthError> {
    match result {
        Ok(value) => Ok(value),
        Err(DbError::NotFound) => Err(AuthError::InvalidToken),
        Err(err) => Err(err.into()),
    }
}

/// Run `body` inside a transaction. The future returns the transaction so this
/// helper can commit it. Dropping the transaction rolls the work back.
async fn transaction<T, F, Fut>(db: &DatabaseConnection, body: F) -> Result<T, AuthError>
where
    F: FnOnce(sea_orm::DatabaseTransaction) -> Fut,
    Fut: std::future::Future<Output = Result<(sea_orm::DatabaseTransaction, T), AuthError>>,
{
    let txn = db.begin().await.map_err(DbError::from)?;
    match body(txn).await {
        Ok((txn, value)) => {
            txn.commit().await.map_err(DbError::from)?;
            Ok(value)
        }
        Err(err) => Err(err),
    }
}

pub async fn mint_invitation(
    db: &DatabaseConnection,
    pepper: &[u8],
    email: &str,
    ttl: time::Duration,
) -> Result<String, AuthError> {
    let email = parse_email(email)?;
    let raw = generate_session_token().map_err(AuthError::Backend)?;
    let hash = token_hash(pepper, &raw).map_err(AuthError::Backend)?;
    let created = now();
    insert_invitation(db, &hash, email.as_ref(), created, created + ttl).await?;
    Ok(raw)
}

/// Insert a recovery token when the account exists. `Ok(None)` means no such user.
pub async fn create_recovery_token(
    db: &DatabaseConnection,
    pepper: &[u8],
    email: &str,
    ttl: time::Duration,
) -> Result<Option<String>, AuthError> {
    let email = parse_email(email)?;
    let Some(user) = get_user_by_email(db, email.as_ref()).await? else {
        let _ = generate_session_token().map_err(AuthError::Backend)?;
        return Ok(None);
    };
    let raw = generate_session_token().map_err(AuthError::Backend)?;
    let hash = token_hash(pepper, &raw).map_err(AuthError::Backend)?;
    let created = now();
    insert_recovery_token(db, user.id, &hash, created, created + ttl).await?;
    Ok(Some(raw))
}

/// Mint a recovery token when the account exists and is not rate-limited.
///
/// Returns `Ok(Some(raw))` when a new token was stored, or `Ok(None)` when the
/// email is unknown or a token was issued within [`RECOVERY_MIN_INTERVAL`].
pub async fn request_recovery(
    db: &DatabaseConnection,
    pepper: &[u8],
    email: &str,
    ttl: time::Duration,
) -> Result<Option<String>, AuthError> {
    let email = parse_email(email)?;
    let Some(user) = get_user_by_email(db, email.as_ref()).await? else {
        let _ = generate_session_token().map_err(AuthError::Backend)?;
        return Ok(None);
    };
    let since = now() - RECOVERY_MIN_INTERVAL;
    if recovery_issued_since(db, user.id, since).await? > 0 {
        return Ok(None);
    }
    let raw = generate_session_token().map_err(AuthError::Backend)?;
    let hash = token_hash(pepper, &raw).map_err(AuthError::Backend)?;
    let created = now();
    insert_recovery_token(db, user.id, &hash, created, created + ttl).await?;
    Ok(Some(raw))
}

pub async fn complete_password_registration(
    db: &DatabaseConnection,
    pepper: &[u8],
    invitation_token: &str,
    password_hash: String,
    alias: Option<String>,
    user_agent: Option<&str>,
) -> Result<SessionGrant, AuthError> {
    let token_hash = token_hash(pepper, invitation_token).map_err(AuthError::Backend)?;
    let alias_owned = alias;
    let ua = user_agent.map(str::to_owned);
    transaction(db, |txn| {
        let password_hash = password_hash.clone();
        let token_hash = token_hash.clone();
        let alias_owned = alias_owned.clone();
        let ua = ua.clone();
        async move {
            let created = now();
            let invitation = invalid_token(consume_invitation(&txn, &token_hash, created).await)?;
            let email = parse_email(&invitation.email)?;
            let alias = match alias_owned {
                Some(alias) => alias,
                None => alias_from_email(email.as_ref()),
            };
            let mut new_user = NewUser::new(email, alias, Role::User, Some(password_hash));
            new_user.stamp_now(created);
            let id = crate::auth::queries::insert_verified_user(&txn, &new_user, created).await?;
            let mut user = User::from_new(id, new_user);
            user.email_verified_at = Some(created);
            let grant = issue_session(&txn, pepper, &user, ua.as_deref(), created).await?;
            touch_last_login_by_id(&txn, user.id, created).await?;
            Ok((txn, grant))
        }
    })
    .await
}

pub async fn login_with_password(
    db: &DatabaseConnection,
    pepper: &[u8],
    hasher: &dyn PasswordHasher,
    email: &str,
    password: &str,
    user_agent: Option<&str>,
) -> Result<SessionGrant, AuthError> {
    let email = parse_email(email)?;
    let user = get_user_by_email(db, email.as_ref()).await?;
    let Some(user) = user else {
        dummy_verify(hasher, password);
        return Err(AuthError::InvalidCredentials);
    };
    let Some(password_hash) = user.password_hash.clone() else {
        dummy_verify(hasher, password);
        return Err(AuthError::InvalidCredentials);
    };
    if !hasher.verify(password, &password_hash)? {
        return Err(AuthError::InvalidCredentials);
    }
    let ua = user_agent.map(str::to_owned);
    transaction(db, |txn| {
        let user = user.clone();
        let ua = ua.clone();
        async move {
            let created = now();
            let grant = issue_session(&txn, pepper, &user, ua.as_deref(), created).await?;
            touch_last_login_by_id(&txn, user.id, created).await?;
            Ok((txn, grant))
        }
    })
    .await
}

pub async fn authenticate_token(
    db: &DatabaseConnection,
    pepper: &[u8],
    raw_token: &str,
) -> Result<(User, i64), AuthError> {
    let hash = token_hash(pepper, raw_token).map_err(AuthError::Backend)?;
    let session = get_session_by_token_hash(db, &hash)
        .await?
        .ok_or(AuthError::InvalidToken)?;
    let created = now();
    if !session.is_active(created) {
        return Err(AuthError::InvalidToken);
    }
    let user = get_user_by_id(db, session.user_id).await?;
    if created - session.last_used_at >= crate::auth::config::LAST_USED_MIN_INTERVAL {
        let _ = touch_session_last_used(db, session.id, created).await;
    }
    Ok((user, session.id))
}

pub async fn list_session_info(
    db: &DatabaseConnection,
    user_id: i64,
    current_token_hash: Option<&str>,
) -> Result<Vec<SessionInfo>, AuthError> {
    let rows = list_active_sessions_for_user(db, user_id, now()).await?;
    Ok(rows
        .into_iter()
        .map(|row| SessionInfo {
            public_id: row.public_id,
            current: current_token_hash.is_some_and(|hash| hash == row.token_hash),
            created_at: row.created_at,
            last_used_at: row.last_used_at,
            expires_at: row.expires_at,
            user_agent: row.user_agent,
        })
        .collect())
}

/// Revoke one active session that belongs to `user_id`. Returns whether that
/// session is the one presenting `current_token_hash`. Missing, expired, or
/// another user's session is not found.
pub async fn revoke_session_for_user(
    db: &DatabaseConnection,
    user_id: i64,
    public_id: uuid::Uuid,
    current_token_hash: Option<&str>,
) -> Result<bool, AuthError> {
    let revoked_at = now();
    let Some(session) = get_active_session_for_user(db, user_id, public_id, revoked_at).await?
    else {
        return Err(AuthError::Db(DbError::NotFound));
    };
    let current = current_token_hash.is_some_and(|hash| hash == session.token_hash);
    revoke_session(db, session.id, revoked_at).await?;
    if current {
        revoke_all_step_ups(db, user_id, revoked_at).await?;
    }
    Ok(current)
}

pub async fn logout_current(
    db: &DatabaseConnection,
    pepper: &[u8],
    raw_token: &str,
) -> Result<(), AuthError> {
    let hash = token_hash(pepper, raw_token).map_err(AuthError::Backend)?;
    if let Some(session) = get_session_by_token_hash(db, &hash).await? {
        let revoked_at = now();
        revoke_session(db, session.id, revoked_at).await?;
        revoke_all_step_ups(db, session.user_id, revoked_at).await?;
    }
    Ok(())
}

pub async fn logout_all_for_user(db: &DatabaseConnection, user_id: i64) -> Result<(), AuthError> {
    let revoked_at = now();
    revoke_all_sessions(db, user_id, revoked_at).await?;
    revoke_all_step_ups(db, user_id, revoked_at).await?;
    Ok(())
}

pub async fn start_invite_passkey(
    db: &DatabaseConnection,
    pepper: &[u8],
    ceremony: &PasskeyCeremony,
    invitation_token: &str,
    alias: Option<String>,
    passkey_label: Option<String>,
) -> Result<(String, serde_json::Value), AuthError> {
    let token_hash = token_hash(pepper, invitation_token).map_err(AuthError::Backend)?;
    let created = now();
    let _ = delete_expired_ceremonies(db, created).await;
    let invitation = find_open_invitation(db, &token_hash, created)
        .await?
        .ok_or(AuthError::InvalidToken)?;
    let email = parse_email(&invitation.email)?;
    let alias = match alias {
        Some(alias) => alias,
        None => alias_from_email(email.as_ref()),
    };
    let public_id = crate::auth::models::new_public_id();
    let (options, state) = ceremony.start_registration(public_id, email.as_ref(), &alias, &[])?;
    let flow_id = new_public_id().to_string();
    insert_ceremony(
        db,
        &CeremonyRecord {
            flow_id: flow_id.clone(),
            kind: CEREMONY_INVITE_REGISTER.into(),
            user_id: None,
            token_hash: Some(token_hash),
            alias: Some(alias),
            passkey_label,
            public_id: Some(public_id),
            state,
            created_at: created,
            expires_at: created + CEREMONY_TTL,
        },
    )
    .await?;
    Ok((flow_id, options))
}

pub async fn finish_invite_passkey(
    db: &DatabaseConnection,
    pepper: &[u8],
    ceremony: &PasskeyCeremony,
    flow_id: &str,
    credential: &serde_json::Value,
    user_agent: Option<&str>,
) -> Result<SessionGrant, AuthError> {
    let record = load_live_ceremony(db, flow_id, CEREMONY_INVITE_REGISTER).await?;
    let (passkey, aaguid) = ceremony.finish_registration(credential, &record.state)?;
    let passkey_body = passkey_json(&passkey)?;
    let credential_id = stored_credential_id(&passkey);
    let ua = user_agent.map(str::to_owned);
    transaction(db, |txn| {
        let record = record.clone();
        let passkey_body = passkey_body.clone();
        let credential_id = credential_id.clone();
        let ua = ua.clone();
        async move {
            let created = now();
            invalid_token(consume_ceremony(&txn, &record.flow_id, created).await)?;
            let token_hash = record.token_hash.ok_or(AuthError::InvalidToken)?;
            let invitation = invalid_token(consume_invitation(&txn, &token_hash, created).await)?;
            let email = parse_email(&invitation.email)?;
            let alias = record
                .alias
                .clone()
                .unwrap_or_else(|| alias_from_email(email.as_ref()));
            let public_id = record
                .public_id
                .unwrap_or_else(crate::auth::models::new_public_id);
            let mut new_user = NewUser::new(email, alias, Role::User, None);
            new_user.public_id = public_id;
            new_user.stamp_now(created);
            let id = crate::auth::queries::insert_verified_user(&txn, &new_user, created).await?;
            let mut user = User::from_new(id, new_user);
            user.email_verified_at = Some(created);
            insert_passkey(
                &txn,
                new_public_id(),
                user.id,
                &credential_id,
                &passkey_body,
                record.passkey_label.as_deref(),
                aaguid,
                created,
            )
            .await?;
            let grant = issue_session(&txn, pepper, &user, ua.as_deref(), created).await?;
            touch_last_login_by_id(&txn, user.id, created).await?;
            Ok((txn, grant))
        }
    })
    .await
}

pub async fn start_session_passkey(
    db: &DatabaseConnection,
    ceremony: &PasskeyCeremony,
    user: &User,
    passkey_label: Option<String>,
) -> Result<(String, serde_json::Value), AuthError> {
    let created = now();
    let _ = delete_expired_ceremonies(db, created).await;
    let existing = list_passkeys_for_user(db, user.id).await?;
    let exclude = credential_ids(&existing)?;
    let (options, state) =
        ceremony.start_registration(user.public_id, user.email.as_ref(), &user.alias, &exclude)?;
    let flow_id = new_public_id().to_string();
    insert_ceremony(
        db,
        &CeremonyRecord {
            flow_id: flow_id.clone(),
            kind: CEREMONY_SESSION_REGISTER.into(),
            user_id: Some(user.id),
            token_hash: None,
            alias: Some(user.alias.clone()),
            passkey_label,
            public_id: Some(user.public_id),
            state,
            created_at: created,
            expires_at: created + CEREMONY_TTL,
        },
    )
    .await?;
    Ok((flow_id, options))
}

pub async fn finish_session_passkey(
    db: &DatabaseConnection,
    ceremony: &PasskeyCeremony,
    user_id: i64,
    flow_id: &str,
    credential: &serde_json::Value,
) -> Result<PasskeyInfo, AuthError> {
    let record = load_live_ceremony(db, flow_id, CEREMONY_SESSION_REGISTER).await?;
    if record.user_id != Some(user_id) {
        return Err(AuthError::InvalidToken);
    }
    let (passkey, aaguid) = ceremony.finish_registration(credential, &record.state)?;
    let passkey_body = passkey_json(&passkey)?;
    let credential_id = stored_credential_id(&passkey);
    transaction(db, |txn| {
        let record = record.clone();
        let passkey_body = passkey_body.clone();
        let credential_id = credential_id.clone();
        async move {
            let created = now();
            invalid_token(consume_ceremony(&txn, &record.flow_id, created).await)?;
            let public_id = new_public_id();
            insert_passkey(
                &txn,
                public_id,
                user_id,
                &credential_id,
                &passkey_body,
                record.passkey_label.as_deref(),
                aaguid,
                created,
            )
            .await?;
            Ok((
                txn,
                PasskeyInfo {
                    public_id,
                    label: record.passkey_label,
                    aaguid,
                    created_at: created,
                    last_used_at: None,
                },
            ))
        }
    })
    .await
}

pub async fn start_passkey_login(
    db: &DatabaseConnection,
    ceremony: &PasskeyCeremony,
) -> Result<(String, serde_json::Value), AuthError> {
    let created = now();
    let _ = delete_expired_ceremonies(db, created).await;
    // No account lookup: an empty allow list does not reveal who has a passkey.
    let (options, state) = ceremony.start_discoverable_authentication()?;
    let flow_id = new_public_id().to_string();
    insert_ceremony(
        db,
        &CeremonyRecord {
            flow_id: flow_id.clone(),
            kind: CEREMONY_LOGIN.into(),
            user_id: None,
            token_hash: None,
            alias: None,
            passkey_label: None,
            public_id: None,
            state,
            created_at: created,
            expires_at: created + CEREMONY_TTL,
        },
    )
    .await?;
    Ok((flow_id, options))
}

pub async fn start_step_up_passkey_login(
    db: &DatabaseConnection,
    ceremony: &PasskeyCeremony,
) -> Result<(String, serde_json::Value), AuthError> {
    let created = now();
    let _ = delete_expired_ceremonies(db, created).await;
    let (options, state) = ceremony.start_discoverable_authentication()?;
    let flow_id = new_public_id().to_string();
    insert_ceremony(
        db,
        &CeremonyRecord {
            flow_id: flow_id.clone(),
            kind: CEREMONY_STEP_UP_LOGIN.into(),
            user_id: None,
            token_hash: None,
            alias: None,
            passkey_label: None,
            public_id: None,
            state,
            created_at: created,
            expires_at: created + CEREMONY_TTL,
        },
    )
    .await?;
    Ok((flow_id, options))
}

/// Password re-authentication for the signed-in user. Does not open a session.
pub async fn step_up_with_password(
    db: &DatabaseConnection,
    pepper: &[u8],
    hasher: &dyn PasswordHasher,
    session_user: &User,
    email: &str,
    password: &str,
) -> Result<StepUpGrant, AuthError> {
    let email = parse_email(email)?;
    if email.as_ref() != session_user.email.as_ref() {
        dummy_verify(hasher, password);
        return Err(AuthError::InvalidCredentials);
    }
    let Some(password_hash) = session_user.password_hash.clone() else {
        dummy_verify(hasher, password);
        return Err(AuthError::InvalidCredentials);
    };
    if !hasher.verify(password, &password_hash)? {
        return Err(AuthError::InvalidCredentials);
    }
    issue_step_up(db, pepper, session_user.id).await
}

pub async fn finish_step_up_passkey_login(
    db: &DatabaseConnection,
    pepper: &[u8],
    ceremony: &PasskeyCeremony,
    session_user_id: i64,
    flow_id: &str,
    credential: &serde_json::Value,
) -> Result<StepUpGrant, AuthError> {
    let record = load_live_ceremony(db, flow_id, CEREMONY_STEP_UP_LOGIN).await?;
    let identified = ceremony.identify_discoverable(credential)?;
    let credential_id = credential_id_key(&identified.credential_id);
    let passkey_row = get_passkey_by_credential_id(db, &credential_id)
        .await?
        .ok_or(AuthError::InvalidCredentials)?;
    let user = match get_user_by_id(db, passkey_row.user_id).await {
        Ok(user) => user,
        Err(DbError::NotFound) => return Err(AuthError::InvalidCredentials),
        Err(err) => return Err(err.into()),
    };
    if user.id != session_user_id || user.public_id != identified.public_id {
        return Err(AuthError::InvalidCredentials);
    }
    let mut stored = passkey_from_json(&passkey_row.passkey)?;
    let auth_result = ceremony.finish_discoverable(&identified, &record.state, &stored)?;
    let updated_json = if auth_result_needs_update(&auth_result) {
        let _ = stored.update_credential(&auth_result);
        Some(passkey_json(&stored)?)
    } else {
        None
    };
    let passkey_id = passkey_row.id;
    transaction(db, |txn| {
        let record = record.clone();
        let updated_json = updated_json.clone();
        async move {
            let created = now();
            invalid_token(consume_ceremony(&txn, &record.flow_id, created).await)?;
            if let Some(json) = updated_json {
                update_passkey_json(&txn, passkey_id, &json, created).await?;
            } else {
                touch_passkey_last_used(&txn, passkey_id, created).await?;
            }
            let grant = issue_step_up_in(&txn, pepper, session_user_id, created).await?;
            Ok((txn, grant))
        }
    })
    .await
}

pub async fn authenticate_step_up(
    db: &DatabaseConnection,
    pepper: &[u8],
    raw_token: &str,
    user_id: i64,
) -> Result<(), AuthError> {
    let hash = token_hash(pepper, raw_token).map_err(AuthError::Backend)?;
    let Some(row) = get_step_up_by_token_hash(db, &hash).await? else {
        return Err(AuthError::StepUpInvalid);
    };
    if row.user_id != user_id || row.revoked_at.is_some() || row.expires_at <= now() {
        return Err(AuthError::StepUpInvalid);
    }
    Ok(())
}

pub struct StepUpGrant {
    pub token: String,
    pub expires_in: i64,
}

async fn issue_step_up(
    db: &DatabaseConnection,
    pepper: &[u8],
    user_id: i64,
) -> Result<StepUpGrant, AuthError> {
    transaction(db, |txn| async move {
        let created = now();
        let grant = issue_step_up_in(&txn, pepper, user_id, created).await?;
        Ok((txn, grant))
    })
    .await
}

async fn issue_step_up_in<C: sea_orm::ConnectionTrait>(
    db: &C,
    pepper: &[u8],
    user_id: i64,
    created: OffsetDateTime,
) -> Result<StepUpGrant, AuthError> {
    revoke_all_step_ups(db, user_id, created).await?;
    let raw = generate_session_token().map_err(AuthError::Backend)?;
    let hash = token_hash(pepper, &raw).map_err(AuthError::Backend)?;
    let expires_at = created + STEP_UP_TTL;
    insert_step_up(db, user_id, &hash, created, expires_at).await?;
    Ok(StepUpGrant {
        token: raw,
        expires_in: STEP_UP_TTL.whole_seconds(),
    })
}

pub async fn finish_passkey_login(
    db: &DatabaseConnection,
    pepper: &[u8],
    ceremony: &PasskeyCeremony,
    flow_id: &str,
    credential: &serde_json::Value,
    user_agent: Option<&str>,
) -> Result<SessionGrant, AuthError> {
    let record = load_live_ceremony(db, flow_id, CEREMONY_LOGIN).await?;
    let identified = ceremony.identify_discoverable(credential)?;
    let credential_id = credential_id_key(&identified.credential_id);
    let passkey_row = get_passkey_by_credential_id(db, &credential_id)
        .await?
        .ok_or(AuthError::InvalidCredentials)?;
    let user = match get_user_by_id(db, passkey_row.user_id).await {
        Ok(user) => user,
        Err(DbError::NotFound) => return Err(AuthError::InvalidCredentials),
        Err(err) => return Err(err.into()),
    };
    if user.public_id != identified.public_id {
        return Err(AuthError::InvalidCredentials);
    }
    let mut stored = passkey_from_json(&passkey_row.passkey)?;
    let auth_result = ceremony.finish_discoverable(&identified, &record.state, &stored)?;
    let updated_json = if auth_result_needs_update(&auth_result) {
        let _ = stored.update_credential(&auth_result);
        Some(passkey_json(&stored)?)
    } else {
        None
    };
    let passkey_id = passkey_row.id;
    let user_id = user.id;
    let ua = user_agent.map(str::to_owned);
    transaction(db, |txn| {
        let record = record.clone();
        let updated_json = updated_json.clone();
        let ua = ua.clone();
        async move {
            let created = now();
            invalid_token(consume_ceremony(&txn, &record.flow_id, created).await)?;
            if let Some(json) = updated_json {
                update_passkey_json(&txn, passkey_id, &json, created).await?;
            } else {
                touch_passkey_last_used(&txn, passkey_id, created).await?;
            }
            let user = get_user_by_id(&txn, user_id).await?;
            let grant = issue_session(&txn, pepper, &user, ua.as_deref(), created).await?;
            touch_last_login_by_id(&txn, user.id, created).await?;
            Ok((txn, grant))
        }
    })
    .await
}

pub async fn list_passkey_info(
    db: &DatabaseConnection,
    user_id: i64,
) -> Result<Vec<PasskeyInfo>, AuthError> {
    let rows = list_passkeys_for_user(db, user_id).await?;
    Ok(rows.iter().map(PasskeyInfo::from).collect())
}

/// Replace the signed-in user's password. Existing sessions stay valid.
pub async fn set_session_password(
    db: &DatabaseConnection,
    user_id: i64,
    password_hash: String,
) -> Result<User, AuthError> {
    transaction(db, |txn| {
        let password_hash = password_hash.clone();
        async move {
            let created = now();
            lock_user_row(&txn, user_id, created).await?;
            set_password_hash_by_id(&txn, user_id, password_hash, created).await?;
            let user = get_user_by_id(&txn, user_id).await?;
            Ok((txn, user))
        }
    })
    .await
}

/// Clear the password when a passkey remains. Refuses when that would leave
/// the account with no sign-in method. Existing sessions stay valid.
pub async fn clear_session_password(
    db: &DatabaseConnection,
    user_id: i64,
) -> Result<User, AuthError> {
    transaction(db, |txn| async move {
        let created = now();
        let user = lock_user_row(&txn, user_id, created).await?;
        let passkeys = count_passkeys_for_user(&txn, user_id).await?;
        if passkeys == 0 {
            return Err(AuthError::LastCredential);
        }
        if user.has_password() {
            clear_password_hash_by_id(&txn, user_id, created).await?;
        }
        let user = get_user_by_id(&txn, user_id).await?;
        Ok((txn, user))
    })
    .await
}

pub async fn delete_passkey(
    db: &DatabaseConnection,
    user_id: i64,
    passkey_public_id: uuid::Uuid,
) -> Result<(), AuthError> {
    transaction(db, |txn| async move {
        let created = now();
        let user = lock_user_row(&txn, user_id, created).await?;
        let Some(row) = get_passkey_for_user(&txn, user_id, passkey_public_id).await? else {
            return Err(AuthError::Db(DbError::NotFound));
        };
        let count = count_passkeys_for_user(&txn, user_id).await?;
        if !user.has_password() && count <= 1 {
            return Err(AuthError::LastCredential);
        }
        let deleted = delete_passkey_by_id(&txn, row.id).await?;
        if deleted == 0 {
            return Err(AuthError::Db(DbError::NotFound));
        }
        Ok((txn, ()))
    })
    .await
}

pub async fn complete_password_recovery(
    db: &DatabaseConnection,
    pepper: &[u8],
    recovery_token: &str,
    password_hash: String,
    user_agent: Option<&str>,
) -> Result<SessionGrant, AuthError> {
    let token_hash = token_hash(pepper, recovery_token).map_err(AuthError::Backend)?;
    let ua = user_agent.map(str::to_owned);
    transaction(db, |txn| {
        let password_hash = password_hash.clone();
        let token_hash = token_hash.clone();
        let ua = ua.clone();
        async move {
            let created = now();
            let user_id = invalid_token(consume_recovery_token(&txn, &token_hash, created).await)?;
            set_password_hash_by_id(&txn, user_id, password_hash, created).await?;
            revoke_all_sessions(&txn, user_id, created).await?;
            revoke_all_step_ups(&txn, user_id, created).await?;
            let user = get_user_by_id(&txn, user_id).await?;
            let grant = issue_session(&txn, pepper, &user, ua.as_deref(), created).await?;
            touch_last_login_by_id(&txn, user.id, created).await?;
            Ok((txn, grant))
        }
    })
    .await
}

pub async fn start_recovery_passkey(
    db: &DatabaseConnection,
    pepper: &[u8],
    ceremony: &PasskeyCeremony,
    recovery_token: &str,
    passkey_label: Option<String>,
) -> Result<(String, serde_json::Value), AuthError> {
    let token_hash = token_hash(pepper, recovery_token).map_err(AuthError::Backend)?;
    let created = now();
    let _ = delete_expired_ceremonies(db, created).await;
    let user_id = find_open_recovery_user(db, &token_hash, created)
        .await?
        .ok_or(AuthError::InvalidToken)?;
    let user = get_user_by_id(db, user_id).await?;
    let existing = list_passkeys_for_user(db, user.id).await?;
    let exclude = credential_ids(&existing)?;
    let (options, state) =
        ceremony.start_registration(user.public_id, user.email.as_ref(), &user.alias, &exclude)?;
    let flow_id = new_public_id().to_string();
    insert_ceremony(
        db,
        &CeremonyRecord {
            flow_id: flow_id.clone(),
            kind: CEREMONY_RECOVERY_REGISTER.into(),
            user_id: Some(user.id),
            token_hash: Some(token_hash),
            alias: Some(user.alias.clone()),
            passkey_label,
            public_id: Some(user.public_id),
            state,
            created_at: created,
            expires_at: created + CEREMONY_TTL,
        },
    )
    .await?;
    Ok((flow_id, options))
}

pub async fn finish_recovery_passkey(
    db: &DatabaseConnection,
    pepper: &[u8],
    ceremony: &PasskeyCeremony,
    flow_id: &str,
    credential: &serde_json::Value,
    user_agent: Option<&str>,
) -> Result<SessionGrant, AuthError> {
    let record = load_live_ceremony(db, flow_id, CEREMONY_RECOVERY_REGISTER).await?;
    let (passkey, aaguid) = ceremony.finish_registration(credential, &record.state)?;
    let passkey_body = passkey_json(&passkey)?;
    let credential_id = stored_credential_id(&passkey);
    let ua = user_agent.map(str::to_owned);
    transaction(db, |txn| {
        let record = record.clone();
        let passkey_body = passkey_body.clone();
        let credential_id = credential_id.clone();
        let ua = ua.clone();
        async move {
            let created = now();
            invalid_token(consume_ceremony(&txn, &record.flow_id, created).await)?;
            let token_hash = record.token_hash.ok_or(AuthError::InvalidToken)?;
            let user_id = invalid_token(consume_recovery_token(&txn, &token_hash, created).await)?;
            if record.user_id != Some(user_id) {
                return Err(AuthError::InvalidToken);
            }
            insert_passkey(
                &txn,
                new_public_id(),
                user_id,
                &credential_id,
                &passkey_body,
                record.passkey_label.as_deref(),
                aaguid,
                created,
            )
            .await?;
            revoke_all_sessions(&txn, user_id, created).await?;
            revoke_all_step_ups(&txn, user_id, created).await?;
            let user = get_user_by_id(&txn, user_id).await?;
            let grant = issue_session(&txn, pepper, &user, ua.as_deref(), created).await?;
            touch_last_login_by_id(&txn, user.id, created).await?;
            Ok((txn, grant))
        }
    })
    .await
}

/// A second session for a command-line tool. The browser cookie is left alone.
pub async fn mint_cli_session(
    db: &DatabaseConnection,
    pepper: &[u8],
    user: &User,
    user_agent: &str,
) -> Result<SessionGrant, AuthError> {
    issue_session(db, pepper, user, Some(user_agent), now()).await
}

async fn issue_session<C: sea_orm::ConnectionTrait>(
    db: &C,
    pepper: &[u8],
    user: &User,
    user_agent: Option<&str>,
    created: OffsetDateTime,
) -> Result<SessionGrant, AuthError> {
    let raw = generate_session_token().map_err(AuthError::Backend)?;
    let hash = token_hash(pepper, &raw).map_err(AuthError::Backend)?;
    let expires_at = created + SESSION_TTL;
    insert_session(db, user.id, &hash, created, expires_at, user_agent).await?;
    Ok(SessionGrant {
        token: raw,
        expires_in: SESSION_TTL.whole_seconds(),
        user: user.clone(),
    })
}

async fn load_live_ceremony(
    db: &DatabaseConnection,
    flow_id: &str,
    kind: &str,
) -> Result<crate::auth::models::CeremonyRecord, AuthError> {
    let record = get_ceremony(db, flow_id)
        .await?
        .ok_or(AuthError::InvalidToken)?;
    if record.kind != kind || record.expires_at <= now() {
        return Err(AuthError::InvalidToken);
    }
    Ok(record)
}

fn parse_email(email: &str) -> Result<EmailAddress, AuthError> {
    let parsed: EmailAddress = email.trim().parse().map_err(|_| AuthError::TypeMismatch)?;
    let normalized = crate::auth::models::normalize_email(&parsed);
    normalized.parse().map_err(|_| AuthError::TypeMismatch)
}

fn credential_ids(rows: &[crate::auth::models::PasskeyRecord]) -> Result<Vec<Vec<u8>>, AuthError> {
    rows.iter()
        .map(|row| {
            let passkey = passkey_from_json(&row.passkey)?;
            Ok(passkey.cred_id().to_vec())
        })
        .collect()
}

fn auth_result_needs_update(result: &webauthn_rs::prelude::AuthenticationResult) -> bool {
    result.needs_update()
}
