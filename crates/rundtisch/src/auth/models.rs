#![allow(dead_code, unused_imports)]

use sea_query::Iden;
use email_address::EmailAddress;
use serde::{Deserialize, Serialize};

type DateTime = time::OffsetDateTime;

#[derive(Serialize, Deserialize)]
enum Role {
    User,
    Admin,
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

struct NewUser {
    email: EmailAddress,
    alias: String,
    role: Role,
    password_hash: String,
    created_at: DateTime,
    updated_at: DateTime,
}

#[derive(Serialize, Deserialize)]
pub struct User {
    id: i64,
    email: EmailAddress,
    alias: String,
    role: Role,
    password_hash: Option<String>,
    email_verified_at: Option<DateTime>,
    created_at: DateTime,
    updated_at: DateTime,
    last_login_at: Option<DateTime>,
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
