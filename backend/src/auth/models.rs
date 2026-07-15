use email_address::EmailAddress;

type DateTime = time::OffsetDateTime;

enum Role {
    User,
    Admin
}

struct User {
    id: i64,
    created_at: DateTime,
    role: Role,
    last_login_at: Option<DateTime>,
    locked_until: Option<DateTime>,
    failed_login_attempts: i32,
    deleted_at: Option<DateTime>,
    updated_at: DateTime
}

struct Email {
    id: i64,
    email_address: EmailAddress,
    is_primary: bool,
    user_id: i64,
    verified_at: Option<DateTime>
}

struct Password {
    user_id: i64,
    hash: String,
    created_at: DateTime,
}

struct PasswordHistory {
    id: i64,
    user_id: i64,
    hash: String,
    created_at: DateTime
}
