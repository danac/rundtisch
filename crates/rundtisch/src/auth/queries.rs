use sea_orm::ActiveValue::Set;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, sea_query::Expr,
};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::auth::entities::{
    invitation, passkey, recovery_token, session, step_up, user, webauthn_state,
};
use crate::auth::error::DbError;
use crate::auth::models::{
    CeremonyRecord, InvitationRecord, NewUser, PasskeyRecord, Session, User,
};

pub async fn get_user_by_email<C: ConnectionTrait>(
    db: &C,
    email: &str,
) -> Result<Option<User>, DbError> {
    let row = user::Entity::find()
        .filter(user::Column::Email.eq(email))
        .one(db)
        .await
        .map_err(DbError::from)?;
    row.map(User::try_from).transpose()
}

pub async fn get_user_by_id<C: ConnectionTrait>(db: &C, id: i64) -> Result<User, DbError> {
    let row = user::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(DbError::from)?
        .ok_or(DbError::NotFound)?;
    User::try_from(row)
}

pub async fn insert_user<C: ConnectionTrait>(db: &C, new_user: &NewUser) -> Result<i64, DbError> {
    insert_user_row(db, new_user, None).await
}

/// Same as [`insert_user`], but sets `email_verified_at` in the same insert.
pub async fn insert_verified_user<C: ConnectionTrait>(
    db: &C,
    new_user: &NewUser,
    email_verified_at: OffsetDateTime,
) -> Result<i64, DbError> {
    insert_user_row(db, new_user, Some(email_verified_at)).await
}

async fn insert_user_row<C: ConnectionTrait>(
    db: &C,
    new_user: &NewUser,
    email_verified_at: Option<OffsetDateTime>,
) -> Result<i64, DbError> {
    let model = user::ActiveModel {
        public_id: Set(new_user.public_id),
        email: Set(new_user.email.to_string()),
        alias: Set(new_user.alias.clone()),
        role: Set(new_user.role),
        password_hash: Set(new_user.password_hash.clone()),
        email_verified_at: Set(email_verified_at),
        created_at: Set(new_user.created_at),
        updated_at: Set(new_user.updated_at),
        ..Default::default()
    };
    let inserted = model.insert(db).await.map_err(DbError::from)?;
    Ok(inserted.id)
}

pub async fn update_user_password_hash<C: ConnectionTrait>(
    db: &C,
    public_id: Uuid,
    password_hash: String,
    updated_at: OffsetDateTime,
) -> Result<u64, DbError> {
    let row = user::Entity::find()
        .filter(user::Column::PublicId.eq(public_id))
        .one(db)
        .await
        .map_err(DbError::from)?
        .ok_or(DbError::NotFound)?;
    let mut active: user::ActiveModel = row.into();
    active.password_hash = Set(Some(password_hash));
    active.updated_at = Set(updated_at);
    active.update(db).await.map_err(DbError::from)?;
    Ok(1)
}

pub async fn clear_password_hash_by_id<C: ConnectionTrait>(
    db: &C,
    id: i64,
    updated_at: OffsetDateTime,
) -> Result<(), DbError> {
    let row = user::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(DbError::from)?
        .ok_or(DbError::NotFound)?;
    let mut active: user::ActiveModel = row.into();
    active.password_hash = Set(None);
    active.updated_at = Set(updated_at);
    active.update(db).await.map_err(DbError::from)?;
    Ok(())
}

pub async fn set_password_hash_by_id<C: ConnectionTrait>(
    db: &C,
    id: i64,
    password_hash: String,
    updated_at: OffsetDateTime,
) -> Result<(), DbError> {
    let row = user::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(DbError::from)?
        .ok_or(DbError::NotFound)?;
    let mut active: user::ActiveModel = row.into();
    active.password_hash = Set(Some(password_hash));
    active.updated_at = Set(updated_at);
    active.update(db).await.map_err(DbError::from)?;
    Ok(())
}

pub async fn set_alias_by_id<C: ConnectionTrait>(
    db: &C,
    id: i64,
    alias: String,
    updated_at: OffsetDateTime,
) -> Result<(), DbError> {
    let row = user::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(DbError::from)?
        .ok_or(DbError::NotFound)?;
    let mut active: user::ActiveModel = row.into();
    active.alias = Set(alias);
    active.updated_at = Set(updated_at);
    active.update(db).await.map_err(DbError::from)?;
    Ok(())
}

pub async fn touch_last_login_by_id<C: ConnectionTrait>(
    db: &C,
    id: i64,
    last_login_at: OffsetDateTime,
) -> Result<(), DbError> {
    let Some(row) = user::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(DbError::from)?
    else {
        return Ok(());
    };
    let mut active: user::ActiveModel = row.into();
    active.last_login_at = Set(Some(last_login_at));
    active.updated_at = Set(last_login_at);
    active.update(db).await.map_err(DbError::from)?;
    Ok(())
}

/// Write to the user row so concurrent credential changes serialize on it.
pub async fn lock_user_row<C: ConnectionTrait>(
    db: &C,
    id: i64,
    now: OffsetDateTime,
) -> Result<User, DbError> {
    let row = user::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(DbError::from)?
        .ok_or(DbError::NotFound)?;
    let mut active: user::ActiveModel = row.into();
    active.updated_at = Set(now);
    let updated = active.update(db).await.map_err(DbError::from)?;
    User::try_from(updated)
}

pub async fn insert_session<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
    token_hash: &str,
    created_at: OffsetDateTime,
    expires_at: OffsetDateTime,
    user_agent: Option<&str>,
) -> Result<(), DbError> {
    let model = session::ActiveModel {
        public_id: Set(crate::auth::models::new_public_id()),
        user_id: Set(user_id),
        token_hash: Set(token_hash.to_owned()),
        created_at: Set(created_at),
        last_used_at: Set(created_at),
        expires_at: Set(expires_at),
        user_agent: Set(user_agent.map(str::to_owned)),
        ..Default::default()
    };
    model.insert(db).await.map_err(DbError::from)?;
    Ok(())
}

pub async fn list_active_sessions_for_user<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
    now: OffsetDateTime,
) -> Result<Vec<Session>, DbError> {
    let rows = session::Entity::find()
        .filter(session::Column::UserId.eq(user_id))
        .filter(session::Column::RevokedAt.is_null())
        .filter(session::Column::ExpiresAt.gt(now))
        .order_by_desc(session::Column::LastUsedAt)
        .all(db)
        .await
        .map_err(DbError::from)?;
    Ok(rows.into_iter().map(Session::from).collect())
}

pub async fn get_active_session_for_user<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
    public_id: Uuid,
    now: OffsetDateTime,
) -> Result<Option<Session>, DbError> {
    let row = session::Entity::find()
        .filter(session::Column::PublicId.eq(public_id))
        .filter(session::Column::UserId.eq(user_id))
        .filter(session::Column::RevokedAt.is_null())
        .filter(session::Column::ExpiresAt.gt(now))
        .one(db)
        .await
        .map_err(DbError::from)?;
    Ok(row.map(Session::from))
}

pub async fn get_session_by_token_hash<C: ConnectionTrait>(
    db: &C,
    token_hash: &str,
) -> Result<Option<Session>, DbError> {
    let row = session::Entity::find()
        .filter(session::Column::TokenHash.eq(token_hash))
        .one(db)
        .await
        .map_err(DbError::from)?;
    Ok(row.map(Session::from))
}

pub async fn touch_session_last_used<C: ConnectionTrait>(
    db: &C,
    id: i64,
    last_used_at: OffsetDateTime,
) -> Result<(), DbError> {
    let Some(row) = session::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(DbError::from)?
    else {
        return Ok(());
    };
    let mut active: session::ActiveModel = row.into();
    active.last_used_at = Set(last_used_at);
    active.update(db).await.map_err(DbError::from)?;
    Ok(())
}

pub async fn revoke_session<C: ConnectionTrait>(
    db: &C,
    id: i64,
    revoked_at: OffsetDateTime,
) -> Result<(), DbError> {
    let result = session::Entity::update_many()
        .col_expr(session::Column::RevokedAt, Expr::value(revoked_at))
        .filter(session::Column::Id.eq(id))
        .filter(session::Column::RevokedAt.is_null())
        .exec(db)
        .await
        .map_err(DbError::from)?;
    let _ = result.rows_affected;
    Ok(())
}

pub async fn revoke_all_sessions<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
    revoked_at: OffsetDateTime,
) -> Result<(), DbError> {
    session::Entity::update_many()
        .col_expr(session::Column::RevokedAt, Expr::value(revoked_at))
        .filter(session::Column::UserId.eq(user_id))
        .filter(session::Column::RevokedAt.is_null())
        .exec(db)
        .await
        .map_err(DbError::from)?;
    Ok(())
}

pub async fn insert_step_up<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
    token_hash: &str,
    created_at: OffsetDateTime,
    expires_at: OffsetDateTime,
) -> Result<(), DbError> {
    let model = step_up::ActiveModel {
        user_id: Set(user_id),
        token_hash: Set(token_hash.to_owned()),
        created_at: Set(created_at),
        expires_at: Set(expires_at),
        ..Default::default()
    };
    model.insert(db).await.map_err(DbError::from)?;
    Ok(())
}

pub async fn get_step_up_by_token_hash<C: ConnectionTrait>(
    db: &C,
    token_hash: &str,
) -> Result<Option<step_up::Model>, DbError> {
    step_up::Entity::find()
        .filter(step_up::Column::TokenHash.eq(token_hash))
        .one(db)
        .await
        .map_err(DbError::from)
}

pub async fn revoke_all_step_ups<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
    revoked_at: OffsetDateTime,
) -> Result<(), DbError> {
    step_up::Entity::update_many()
        .col_expr(step_up::Column::RevokedAt, Expr::value(revoked_at))
        .filter(step_up::Column::UserId.eq(user_id))
        .filter(step_up::Column::RevokedAt.is_null())
        .exec(db)
        .await
        .map_err(DbError::from)?;
    Ok(())
}

pub async fn insert_invitation<C: ConnectionTrait>(
    db: &C,
    token_hash: &str,
    email: &str,
    created_at: OffsetDateTime,
    expires_at: OffsetDateTime,
) -> Result<(), DbError> {
    let model = invitation::ActiveModel {
        token_hash: Set(token_hash.to_owned()),
        email: Set(email.to_owned()),
        created_at: Set(created_at),
        expires_at: Set(expires_at),
        ..Default::default()
    };
    model.insert(db).await.map_err(DbError::from)?;
    Ok(())
}

pub async fn find_open_invitation<C: ConnectionTrait>(
    db: &C,
    token_hash: &str,
    now: OffsetDateTime,
) -> Result<Option<InvitationRecord>, DbError> {
    let row = invitation::Entity::find()
        .filter(invitation::Column::TokenHash.eq(token_hash))
        .filter(invitation::Column::UsedAt.is_null())
        .filter(invitation::Column::ExpiresAt.gt(now))
        .one(db)
        .await
        .map_err(DbError::from)?;
    Ok(row.map(InvitationRecord::from))
}

/// Conditional consume. Zero rows means missing, expired, or already used.
pub async fn consume_invitation<C: ConnectionTrait>(
    db: &C,
    token_hash: &str,
    now: OffsetDateTime,
) -> Result<InvitationRecord, DbError> {
    let result = invitation::Entity::update_many()
        .col_expr(invitation::Column::UsedAt, Expr::value(now))
        .filter(invitation::Column::TokenHash.eq(token_hash))
        .filter(invitation::Column::UsedAt.is_null())
        .filter(invitation::Column::ExpiresAt.gt(now))
        .exec(db)
        .await
        .map_err(DbError::from)?;
    if result.rows_affected == 0 {
        return Err(DbError::NotFound);
    }
    let row = invitation::Entity::find()
        .filter(invitation::Column::TokenHash.eq(token_hash))
        .one(db)
        .await
        .map_err(DbError::from)?
        .ok_or(DbError::NotFound)?;
    Ok(InvitationRecord::from(row))
}

pub async fn insert_recovery_token<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
    token_hash: &str,
    created_at: OffsetDateTime,
    expires_at: OffsetDateTime,
) -> Result<(), DbError> {
    let model = recovery_token::ActiveModel {
        user_id: Set(user_id),
        token_hash: Set(token_hash.to_owned()),
        created_at: Set(created_at),
        expires_at: Set(expires_at),
        ..Default::default()
    };
    model.insert(db).await.map_err(DbError::from)?;
    Ok(())
}

pub async fn recovery_issued_since<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
    since: OffsetDateTime,
) -> Result<u64, DbError> {
    recovery_token::Entity::find()
        .filter(recovery_token::Column::UserId.eq(user_id))
        .filter(recovery_token::Column::CreatedAt.gt(since))
        .count(db)
        .await
        .map_err(DbError::from)
}

pub async fn consume_recovery_token<C: ConnectionTrait>(
    db: &C,
    token_hash: &str,
    now: OffsetDateTime,
) -> Result<i64, DbError> {
    let existing = recovery_token::Entity::find()
        .filter(recovery_token::Column::TokenHash.eq(token_hash))
        .filter(recovery_token::Column::UsedAt.is_null())
        .filter(recovery_token::Column::ExpiresAt.gt(now))
        .one(db)
        .await
        .map_err(DbError::from)?
        .ok_or(DbError::NotFound)?;
    let result = recovery_token::Entity::update_many()
        .col_expr(recovery_token::Column::UsedAt, Expr::value(now))
        .filter(recovery_token::Column::TokenHash.eq(token_hash))
        .filter(recovery_token::Column::UsedAt.is_null())
        .filter(recovery_token::Column::ExpiresAt.gt(now))
        .exec(db)
        .await
        .map_err(DbError::from)?;
    if result.rows_affected == 0 {
        return Err(DbError::NotFound);
    }
    Ok(existing.user_id)
}

pub async fn find_open_recovery_user<C: ConnectionTrait>(
    db: &C,
    token_hash: &str,
    now: OffsetDateTime,
) -> Result<Option<i64>, DbError> {
    let row = recovery_token::Entity::find()
        .filter(recovery_token::Column::TokenHash.eq(token_hash))
        .filter(recovery_token::Column::UsedAt.is_null())
        .filter(recovery_token::Column::ExpiresAt.gt(now))
        .one(db)
        .await
        .map_err(DbError::from)?;
    Ok(row.map(|row| row.user_id))
}

pub async fn insert_passkey<C: ConnectionTrait>(
    db: &C,
    public_id: Uuid,
    user_id: i64,
    credential_id: &str,
    passkey_json: &str,
    label: Option<&str>,
    aaguid: Option<Uuid>,
    created_at: OffsetDateTime,
) -> Result<i64, DbError> {
    let model = passkey::ActiveModel {
        public_id: Set(public_id),
        user_id: Set(user_id),
        credential_id: Set(credential_id.to_owned()),
        passkey: Set(passkey_json.to_owned()),
        label: Set(label.map(str::to_owned)),
        aaguid: Set(aaguid),
        created_at: Set(created_at),
        ..Default::default()
    };
    let inserted = model.insert(db).await.map_err(DbError::from)?;
    Ok(inserted.id)
}

pub async fn list_passkeys_for_user<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
) -> Result<Vec<PasskeyRecord>, DbError> {
    let rows = passkey::Entity::find()
        .filter(passkey::Column::UserId.eq(user_id))
        .order_by_asc(passkey::Column::Id)
        .all(db)
        .await
        .map_err(DbError::from)?;
    Ok(rows.into_iter().map(PasskeyRecord::from).collect())
}

pub async fn count_passkeys_for_user<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
) -> Result<u64, DbError> {
    passkey::Entity::find()
        .filter(passkey::Column::UserId.eq(user_id))
        .count(db)
        .await
        .map_err(DbError::from)
}

pub async fn get_passkey_by_credential_id<C: ConnectionTrait>(
    db: &C,
    credential_id: &str,
) -> Result<Option<PasskeyRecord>, DbError> {
    let row = passkey::Entity::find()
        .filter(passkey::Column::CredentialId.eq(credential_id))
        .one(db)
        .await
        .map_err(DbError::from)?;
    Ok(row.map(PasskeyRecord::from))
}

pub async fn get_passkey_for_user<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
    public_id: Uuid,
) -> Result<Option<PasskeyRecord>, DbError> {
    let row = passkey::Entity::find()
        .filter(passkey::Column::PublicId.eq(public_id))
        .filter(passkey::Column::UserId.eq(user_id))
        .one(db)
        .await
        .map_err(DbError::from)?;
    Ok(row.map(PasskeyRecord::from))
}

pub async fn update_passkey_json<C: ConnectionTrait>(
    db: &C,
    id: i64,
    passkey_json: &str,
    last_used_at: OffsetDateTime,
) -> Result<(), DbError> {
    let Some(row) = passkey::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(DbError::from)?
    else {
        return Err(DbError::NotFound);
    };
    let mut active: passkey::ActiveModel = row.into();
    active.passkey = Set(passkey_json.to_owned());
    active.last_used_at = Set(Some(last_used_at));
    active.update(db).await.map_err(DbError::from)?;
    Ok(())
}

pub async fn touch_passkey_last_used<C: ConnectionTrait>(
    db: &C,
    id: i64,
    last_used_at: OffsetDateTime,
) -> Result<(), DbError> {
    let Some(row) = passkey::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(DbError::from)?
    else {
        return Ok(());
    };
    let mut active: passkey::ActiveModel = row.into();
    active.last_used_at = Set(Some(last_used_at));
    active.update(db).await.map_err(DbError::from)?;
    Ok(())
}

pub async fn delete_passkey_by_id<C: ConnectionTrait>(db: &C, id: i64) -> Result<u64, DbError> {
    let result = passkey::Entity::delete_by_id(id)
        .exec(db)
        .await
        .map_err(DbError::from)?;
    Ok(result.rows_affected)
}

pub async fn insert_ceremony<C: ConnectionTrait>(
    db: &C,
    record: &CeremonyRecord,
) -> Result<(), DbError> {
    let model = webauthn_state::ActiveModel {
        flow_id: Set(record.flow_id.clone()),
        kind: Set(record.kind.clone()),
        user_id: Set(record.user_id),
        token_hash: Set(record.token_hash.clone()),
        alias: Set(record.alias.clone()),
        passkey_label: Set(record.passkey_label.clone()),
        public_id: Set(record.public_id),
        state: Set(record.state.clone()),
        created_at: Set(record.created_at),
        expires_at: Set(record.expires_at),
    };
    model.insert(db).await.map_err(DbError::from)?;
    Ok(())
}

pub async fn get_ceremony<C: ConnectionTrait>(
    db: &C,
    flow_id: &str,
) -> Result<Option<CeremonyRecord>, DbError> {
    let row = webauthn_state::Entity::find_by_id(flow_id)
        .one(db)
        .await
        .map_err(DbError::from)?;
    Ok(row.map(CeremonyRecord::from))
}

/// Delete one unexpired ceremony. Zero rows means it was already consumed or expired.
pub async fn consume_ceremony<C: ConnectionTrait>(
    db: &C,
    flow_id: &str,
    now: OffsetDateTime,
) -> Result<(), DbError> {
    let result = webauthn_state::Entity::delete_many()
        .filter(webauthn_state::Column::FlowId.eq(flow_id))
        .filter(webauthn_state::Column::ExpiresAt.gt(now))
        .exec(db)
        .await
        .map_err(DbError::from)?;
    if result.rows_affected == 0 {
        return Err(DbError::NotFound);
    }
    Ok(())
}

pub async fn delete_expired_ceremonies<C: ConnectionTrait>(
    db: &C,
    now: OffsetDateTime,
) -> Result<(), DbError> {
    webauthn_state::Entity::delete_many()
        .filter(webauthn_state::Column::ExpiresAt.lte(now))
        .exec(db)
        .await
        .map_err(DbError::from)?;
    Ok(())
}

impl TryFrom<user::Model> for User {
    type Error = DbError;

    fn try_from(model: user::Model) -> Result<Self, Self::Error> {
        let email = model
            .email
            .parse()
            .map_err(|_| DbError::Backend(format!("invalid stored email {}", model.email)))?;
        Ok(Self {
            id: model.id,
            public_id: model.public_id,
            email,
            alias: model.alias,
            role: model.role,
            password_hash: model.password_hash,
            email_verified_at: model.email_verified_at,
            created_at: model.created_at,
            updated_at: model.updated_at,
            last_login_at: model.last_login_at,
        })
    }
}

impl From<session::Model> for Session {
    fn from(model: session::Model) -> Self {
        Self {
            id: model.id,
            public_id: model.public_id,
            user_id: model.user_id,
            token_hash: model.token_hash,
            created_at: model.created_at,
            last_used_at: model.last_used_at,
            expires_at: model.expires_at,
            revoked_at: model.revoked_at,
            user_agent: model.user_agent,
        }
    }
}

impl From<passkey::Model> for PasskeyRecord {
    fn from(model: passkey::Model) -> Self {
        Self {
            id: model.id,
            public_id: model.public_id,
            user_id: model.user_id,
            credential_id: model.credential_id,
            passkey: model.passkey,
            label: model.label,
            aaguid: model.aaguid,
            created_at: model.created_at,
            last_used_at: model.last_used_at,
        }
    }
}

impl From<invitation::Model> for InvitationRecord {
    fn from(model: invitation::Model) -> Self {
        Self {
            id: model.id,
            token_hash: model.token_hash,
            email: model.email,
            created_at: model.created_at,
            expires_at: model.expires_at,
            used_at: model.used_at,
        }
    }
}

impl From<webauthn_state::Model> for CeremonyRecord {
    fn from(model: webauthn_state::Model) -> Self {
        Self {
            flow_id: model.flow_id,
            kind: model.kind,
            user_id: model.user_id,
            token_hash: model.token_hash,
            alias: model.alias,
            passkey_label: model.passkey_label,
            public_id: model.public_id,
            state: model.state,
            created_at: model.created_at,
            expires_at: model.expires_at,
        }
    }
}
