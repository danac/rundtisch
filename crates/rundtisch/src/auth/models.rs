#![allow(dead_code, unused_imports)]

use email_address::EmailAddress;
use sea_orm::DeriveActiveEnum;
use sea_orm::EnumIter;
use sea_orm::prelude::StringLen;
use serde::{Deserialize, Serialize};

type DateTime = time::OffsetDateTime;

fn now_utc() -> DateTime {
    DateTime::now_utc()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::None)")]
pub enum Role {
    #[sea_orm(string_value = "User")]
    User,
    #[sea_orm(string_value = "Admin")]
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

/// Opaque UUIDv4. Integer [`User::id`] stays the row id / FK target and is not
/// a public identifier. `public_id` is also the WebAuthn user handle.
pub fn public_id_from_bytes(mut bytes: [u8; 16]) -> uuid::Uuid {
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    uuid::Uuid::from_bytes(bytes)
}

pub fn new_public_id() -> uuid::Uuid {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("CSPRNG available for UUIDv4");
    public_id_from_bytes(bytes)
}

pub fn normalize_email(email: &EmailAddress) -> String {
    email.as_ref().trim().to_ascii_lowercase()
}

pub fn alias_from_email(email: &str) -> String {
    email
        .split('@')
        .next()
        .filter(|part| !part.is_empty())
        .unwrap_or("user")
        .to_string()
}

/// Row to insert into `auth_users`. Timestamps default to now so JSON
/// clients can omit them; handlers overwrite both with a single clock read.
/// `public_id` is assigned server-side (never accepted from JSON).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewUser {
    #[serde(skip_deserializing, default = "uuid::Uuid::nil")]
    pub public_id: uuid::Uuid,
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

/// Body for `PATCH /api/auth/users/{public_id}` — only alias may change.
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
            public_id: new_public_id(),
            email,
            alias: alias.into(),
            role,
            password_hash,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn stamp_now(&mut self, now: DateTime) {
        self.created_at = now;
        self.updated_at = now;
    }

    pub fn assign_public_id(&mut self) {
        self.public_id = new_public_id();
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    /// Integer row id; FK target. Not serialized.
    #[serde(skip_serializing)]
    pub id: i64,
    pub public_id: uuid::Uuid,
    pub email: EmailAddress,
    pub alias: String,
    pub role: Role,
    #[serde(skip_serializing)]
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
            public_id: new_user.public_id,
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

    pub fn has_password(&self) -> bool {
        self.password_hash.as_ref().is_some_and(|hash| !hash.is_empty())
    }
}

/// Public account fields returned by login, recovery, and `GET /auth/me`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountView {
    pub public_id: uuid::Uuid,
    pub email: EmailAddress,
    pub alias: String,
    pub role: Role,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: DateTime,
    pub has_password: bool,
}

impl From<&User> for AccountView {
    fn from(user: &User) -> Self {
        Self {
            public_id: user.public_id,
            email: user.email.clone(),
            alias: user.alias.clone(),
            role: user.role,
            created_at: user.created_at,
            has_password: user.has_password(),
        }
    }
}

pub fn datetime_to_rfc3339(dt: DateTime) -> String {
    dt.format(&time::format_description::well_known::Rfc3339)
        .expect("OffsetDateTime is always a valid RFC 3339 timestamp")
}

/// Row in `auth_sessions`. `token_hash` is HMAC-SHA-256 of the raw bearer
/// token with `AUTH_HASH_PEPPER`; the raw token is never stored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Session {
    pub id: i64,
    pub user_id: i64,
    pub token_hash: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: DateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub last_used_at: DateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub expires_at: DateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub revoked_at: Option<DateTime>,
    pub user_agent: Option<String>,
}

impl Session {
    pub fn is_active(&self, now: DateTime) -> bool {
        self.revoked_at.is_none() && self.expires_at > now
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PasskeyRecord {
    pub id: i64,
    pub user_id: i64,
    pub credential_id: String,
    pub passkey: String,
    pub created_at: DateTime,
    pub last_used_at: Option<DateTime>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PasskeyInfo {
    pub id: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: DateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub last_used_at: Option<DateTime>,
}

impl From<&PasskeyRecord> for PasskeyInfo {
    fn from(row: &PasskeyRecord) -> Self {
        Self {
            id: row.id,
            created_at: row.created_at,
            last_used_at: row.last_used_at,
        }
    }
}

#[derive(Debug, Clone)]
pub struct InvitationRecord {
    pub id: i64,
    pub token_hash: String,
    pub email: String,
    pub created_at: DateTime,
    pub expires_at: DateTime,
    pub used_at: Option<DateTime>,
}

#[derive(Debug, Clone)]
pub struct CeremonyRecord {
    pub flow_id: String,
    pub kind: String,
    pub user_id: Option<i64>,
    pub token_hash: Option<String>,
    pub alias: Option<String>,
    pub public_id: Option<uuid::Uuid>,
    pub state: String,
    pub created_at: DateTime,
    pub expires_at: DateTime,
}

pub const CEREMONY_INVITE_REGISTER: &str = "invite_register";
pub const CEREMONY_RECOVERY_REGISTER: &str = "recovery_register";
pub const CEREMONY_SESSION_REGISTER: &str = "session_register";
pub const CEREMONY_LOGIN: &str = "login";

/// Successful authentication result. `token` is the raw bearer secret.
#[derive(Debug, Clone)]
pub struct SessionGrant {
    pub token: String,
    pub expires_in: i64,
    pub user: User,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_public_id_is_uuid_v4() {
        let id = new_public_id();
        assert_eq!(id.get_version(), Some(uuid::Version::Random));
        assert_ne!(id, new_public_id());
    }

    #[test]
    fn user_json_omits_integer_id() {
        let user = User::from_new(
            42,
            NewUser::new(
                "alice@example.com".parse().unwrap(),
                "alice",
                Role::User,
                None,
            ),
        );
        let value = serde_json::to_value(&user).expect("serialize user");
        assert!(value.get("id").is_none(), "{value}");
        assert!(value.get("password_hash").is_none(), "{value}");
        let public_id = value["public_id"].as_str().expect("public_id string");
        uuid::Uuid::parse_str(public_id).expect("public_id is a uuid");
        assert_eq!(public_id, user.public_id.to_string());
    }

    #[test]
    fn new_user_json_ignores_client_public_id() {
        let user: NewUser = serde_json::from_str(
            r#"{"email":"bob@example.com","alias":"bob","role":"User","public_id":"11111111-1111-4111-8111-111111111111"}"#,
        )
        .expect("deserialize NewUser");
        assert_eq!(user.public_id, uuid::Uuid::nil());
        assert_eq!(user.alias, "bob");
    }
}
