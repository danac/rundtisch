use sea_orm::ActiveValue::Set;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
};
use uuid::Uuid;

use crate::auth::entities::{session, user};
use crate::auth::error::DbError;
use crate::auth::models::{NewUser, Session, User};

pub async fn list_users(db: &DatabaseConnection) -> Result<Vec<User>, DbError> {
    let rows = user::Entity::find()
        .order_by_asc(user::Column::Id)
        .all(db)
        .await
        .map_err(DbError::from)?;
    rows.into_iter().map(User::try_from).collect()
}

pub async fn get_user_by_public_id(db: &DatabaseConnection, public_id: Uuid) -> Result<User, DbError> {
    let row = user::Entity::find()
        .filter(user::Column::PublicId.eq(public_id))
        .one(db)
        .await
        .map_err(DbError::from)?
        .ok_or(DbError::NotFound)?;
    User::try_from(row)
}

pub async fn get_user_by_email(
    db: &DatabaseConnection,
    email: &str,
) -> Result<Option<User>, DbError> {
    let row = user::Entity::find()
        .filter(user::Column::Email.eq(email))
        .one(db)
        .await
        .map_err(DbError::from)?;
    row.map(User::try_from).transpose()
}

pub async fn get_user_by_id(db: &DatabaseConnection, id: i64) -> Result<User, DbError> {
    let row = user::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(DbError::from)?
        .ok_or(DbError::NotFound)?;
    User::try_from(row)
}

pub async fn insert_user(db: &DatabaseConnection, new_user: &NewUser) -> Result<i64, DbError> {
    let model = user::ActiveModel {
        public_id: Set(new_user.public_id),
        email: Set(new_user.email.to_string()),
        alias: Set(new_user.alias.clone()),
        role: Set(new_user.role),
        password_hash: Set(new_user.password_hash.clone()),
        created_at: Set(new_user.created_at),
        updated_at: Set(new_user.updated_at),
        ..Default::default()
    };
    let inserted = model.insert(db).await.map_err(DbError::from)?;
    Ok(inserted.id)
}

pub async fn update_user_alias(
    db: &DatabaseConnection,
    public_id: Uuid,
    alias: &str,
    updated_at: time::OffsetDateTime,
) -> Result<u64, DbError> {
    let row = user::Entity::find()
        .filter(user::Column::PublicId.eq(public_id))
        .one(db)
        .await
        .map_err(DbError::from)?
        .ok_or(DbError::NotFound)?;
    let mut active: user::ActiveModel = row.into();
    active.alias = Set(alias.to_owned());
    active.updated_at = Set(updated_at);
    active.update(db).await.map_err(DbError::from)?;
    Ok(1)
}

pub async fn delete_user(db: &DatabaseConnection, public_id: Uuid) -> Result<u64, DbError> {
    let result = user::Entity::delete_many()
        .filter(user::Column::PublicId.eq(public_id))
        .exec(db)
        .await
        .map_err(DbError::from)?;
    Ok(result.rows_affected)
}

pub async fn verify_email(
    db: &DatabaseConnection,
    public_id: Uuid,
    email: &str,
    verified_at: time::OffsetDateTime,
) -> Result<u64, DbError> {
    let Some(row) = user::Entity::find()
        .filter(user::Column::PublicId.eq(public_id))
        .filter(user::Column::Email.eq(email))
        .one(db)
        .await
        .map_err(DbError::from)?
    else {
        return Ok(0);
    };
    let mut active: user::ActiveModel = row.into();
    active.email_verified_at = Set(Some(verified_at));
    active.updated_at = Set(verified_at);
    active.update(db).await.map_err(DbError::from)?;
    Ok(1)
}

pub async fn touch_last_login(
    db: &DatabaseConnection,
    public_id: Uuid,
    last_login_at: time::OffsetDateTime,
) -> Result<(), DbError> {
    let Some(row) = user::Entity::find()
        .filter(user::Column::PublicId.eq(public_id))
        .one(db)
        .await
        .map_err(DbError::from)?
    else {
        return Ok(());
    };
    let mut active: user::ActiveModel = row.into();
    active.last_login_at = Set(Some(last_login_at));
    active.updated_at = Set(last_login_at);
    let _ = active.update(db).await.map_err(DbError::from)?;
    Ok(())
}

pub async fn insert_session(
    db: &DatabaseConnection,
    user_id: i64,
    token_hash: &str,
    created_at: time::OffsetDateTime,
    expires_at: time::OffsetDateTime,
    user_agent: Option<&str>,
) -> Result<(), DbError> {
    let model = session::ActiveModel {
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

pub async fn get_session_by_token_hash(
    db: &DatabaseConnection,
    token_hash: &str,
) -> Result<Option<Session>, DbError> {
    let row = session::Entity::find()
        .filter(session::Column::TokenHash.eq(token_hash))
        .one(db)
        .await
        .map_err(DbError::from)?;
    Ok(row.map(Session::from))
}

pub async fn rotate_session(
    db: &DatabaseConnection,
    id: i64,
    token_hash: &str,
    last_used_at: time::OffsetDateTime,
) -> Result<(), DbError> {
    let row = session::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(DbError::from)?
        .ok_or(DbError::NotFound)?;
    let mut active: session::ActiveModel = row.into();
    active.token_hash = Set(token_hash.to_owned());
    active.last_used_at = Set(last_used_at);
    active.update(db).await.map_err(DbError::from)?;
    Ok(())
}

pub async fn revoke_session(
    db: &DatabaseConnection,
    id: i64,
    revoked_at: time::OffsetDateTime,
) -> Result<(), DbError> {
    let Some(row) = session::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(DbError::from)?
    else {
        return Ok(());
    };
    let mut active: session::ActiveModel = row.into();
    active.revoked_at = Set(Some(revoked_at));
    active.update(db).await.map_err(DbError::from)?;
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
