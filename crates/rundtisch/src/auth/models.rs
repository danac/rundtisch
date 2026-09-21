#![allow(dead_code, unused_imports)]

use email_address::EmailAddress;
use sea_query::Iden;
use serde::{Deserialize, Serialize};

type DateTime = time::OffsetDateTime;

fn now_utc() -> DateTime {
    DateTime::now_utc()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    User,
    Admin,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::User => "User",
            Role::Admin => "Admin",
        }
    }
}

#[derive(Iden)]
pub enum UserTable {
    #[iden = "auth_users"]
    Table,
    Id,
    Email,
    Alias,
    Role,
    PasswordHash,
    EmailVerifiedAt,
    CreatedAt,
    UpdatedAt,
    LastLoginAt,
}

/// Row to insert into [`UserTable`]. Timestamps default to now so JSON
/// clients can omit them; handlers overwrite both with a single clock read.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewUser {
    pub email: EmailAddress,
    pub alias: String,
    pub role: Role,
    #[serde(default)]
    pub password_hash: Option<String>,
    #[serde(with = "time::serde::rfc3339", default = "now_utc")]
    pub created_at: DateTime,
    #[serde(with = "time::serde::rfc3339", default = "now_utc")]
    pub updated_at: DateTime,
}

/// Body for `PATCH /api/auth/users/{id}` — only alias may change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserAlias {
    pub alias: String,
}

impl NewUser {
    pub fn new(
        email: EmailAddress,
        alias: impl Into<String>,
        role: Role,
        password_hash: Option<String>,
    ) -> Self {
        let now = now_utc();
        Self {
            email,
            alias: alias.into(),
            role,
            password_hash,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn stamp_now(&mut self) {
        let now = now_utc();
        self.created_at = now;
        self.updated_at = now;
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub email: EmailAddress,
    pub alias: String,
    pub role: Role,
    pub password_hash: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub email_verified_at: Option<DateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: DateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: DateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub last_login_at: Option<DateTime>,
}

impl User {
    pub fn from_new(id: i64, new_user: NewUser) -> Self {
        Self {
            id,
            email: new_user.email,
            alias: new_user.alias,
            role: new_user.role,
            password_hash: new_user.password_hash,
            email_verified_at: None,
            created_at: new_user.created_at,
            updated_at: new_user.updated_at,
            last_login_at: None,
        }
    }
}

pub fn datetime_to_rfc3339(dt: DateTime) -> String {
    dt.format(&time::format_description::well_known::Rfc3339)
        .expect("OffsetDateTime is always a valid RFC 3339 timestamp")
}

#[derive(Iden)]
pub enum RefreshTokenTable {
    #[iden = "auth_refresh_tokens"]
    Table,
    Id,
    UserId,
    TokenHash,
    ExpiresAt,
    Revoked,
}

// struct RefreshToken {
//     id: i64,
//     user_id: i64,
//     token_hash: String,
//     expires_at: DateTime,
//     revoked: bool,
// }
