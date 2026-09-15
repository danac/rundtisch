#![allow(dead_code, unused_imports)]

use email_address::EmailAddress;
use sea_query::Iden;
type DateTime = time::OffsetDateTime;

enum Role {
    User,
    Admin,
}

#[derive(Iden)]
pub enum UserTable {
    #[iden = "users"]
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

// struct User {
//     id: i64,
//     email: EmailAddress,
//     alias: String,
//     role: Role,
//     password_hash: Option<String>,
//     email_verified_at: Option<DateTime>,
//     created_at: DateTime,
//     updated_at: DateTime,
//     last_login_at: Option<DateTime>,
// }

#[derive(Iden)]
pub enum EmailVerificationTable {
    #[iden = "email_verifications"]
    Table,
    Id,
    UserId,
    TokenHash,
    ExpiresAt,
}

// struct EmailVerification {
//     id: i64,
//     user_id: i64,
//     token_hash: String,
//     expires_at: DateTime,
// }

#[derive(Iden)]
pub enum RefreshTokenTable {
    #[iden = "refresh_tokens"]
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
