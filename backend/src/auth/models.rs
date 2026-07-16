use email_address::EmailAddress;
use sea_query::Iden;
type DateTime = time::OffsetDateTime;

enum Role {
    User,
    Admin
}

#[derive(Iden)]
pub enum UserTable {
    Table,
    Id,
    Role,
    Alias,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    LastLoginAt,
    LockedUntil,
    FailedLoginAttempts
}

// struct User {
//     id: i64,
//     created_at: DateTime,
//     role: Role,
//     alias: String,
//     last_login_at: Option<DateTime>,
//     locked_until: Option<DateTime>,
//     failed_login_attempts: i32,
//     deleted_at: Option<DateTime>,
//     updated_at: DateTime
// }

#[derive(Iden)]
pub enum EmailTable {
    Table,
    Id,
    EmailAddress,
    IsPrimary,
    UserId,
    VerifiedAt,
}

// struct Email {
//     id: i64,
//     email_address: EmailAddress,
//     is_primary: bool,
//     user_id: i64,
//     verified_at: Option<DateTime>
// }

#[derive(Iden)]
pub enum PasswordTable {
    Table,
    Id,
    UserId,
    Hash,
    CreatedAt,
}

// struct Password {
//     user_id: i64,
//     hash: String,
//     created_at: DateTime,
// }

#[derive(Iden)]
pub enum PasswordHistoryTable {
    Table,
    Id,
    UserId,
    Hash,
    CreatedAt,
}

// struct PasswordHistory {
//     id: i64,
//     user_id: i64,
//     hash: String,
//     created_at: DateTime
// }
