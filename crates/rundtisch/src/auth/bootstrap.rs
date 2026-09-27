use std::fmt;

use email_address::EmailAddress;
use sea_orm::DatabaseConnection;
use time::OffsetDateTime;
use uuid::Uuid;
use zeroize::Zeroize;

use crate::auth::config::AUTH_HASH_PEPPER;
use crate::auth::error::DbError;
use crate::auth::models::{NewUser, Role};
use crate::auth::password::{
    Argon2idHasher, PasswordHashError, PasswordHasher, check_password_policy,
};
use crate::auth::queries::{get_user_by_email, insert_verified_user, update_user_password_hash};

/// Prefix for optional first-admin secrets read by the migrate binary.
pub const BOOTSTRAP_ADMIN_PREFIX: &str = "RUNDTISCH_BOOTSTRAP_ADMIN_";
pub const BOOTSTRAP_ADMIN_EMAIL: &str = "RUNDTISCH_BOOTSTRAP_ADMIN_EMAIL";
pub const BOOTSTRAP_ADMIN_PASSWORD: &str = "RUNDTISCH_BOOTSTRAP_ADMIN_PASSWORD";
pub const BOOTSTRAP_ADMIN_ALIAS: &str = "RUNDTISCH_BOOTSTRAP_ADMIN_ALIAS";

/// Credentials for a bootstrap-admin upsert. `password` is redacted in
/// [`Debug`] and zeroized on drop.
pub struct BootstrapAdminSecrets {
    email: String,
    password: String,
    alias: Option<String>,
}

impl BootstrapAdminSecrets {
    pub fn new(
        email: impl Into<String>,
        password: impl Into<String>,
        alias: Option<String>,
    ) -> Self {
        Self {
            email: email.into(),
            password: password.into(),
            alias,
        }
    }

    pub fn email(&self) -> &str {
        &self.email
    }

    pub fn password(&self) -> &str {
        &self.password
    }

    pub fn alias(&self) -> Option<&str> {
        self.alias.as_deref()
    }
}

impl fmt::Debug for BootstrapAdminSecrets {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BootstrapAdminSecrets")
            .field("email", &self.email)
            .field("password", &"***")
            .field("alias", &self.alias)
            .finish()
    }
}

impl Drop for BootstrapAdminSecrets {
    fn drop(&mut self) {
        self.password.zeroize();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BootstrapAdminOutcome {
    SkippedUnset,
    Inserted { email: String, public_id: Uuid },
    Updated { email: String, public_id: Uuid },
}

#[derive(Debug)]
pub enum BootstrapAdminError {
    IncompleteSecrets,
    InvalidEmail,
    InvalidAlias,
    InvalidPassword,
    MissingPepper,
    Hash(PasswordHashError),
    Db(DbError),
}

impl fmt::Display for BootstrapAdminError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BootstrapAdminError::IncompleteSecrets => {
                write!(
                    f,
                    "set both {BOOTSTRAP_ADMIN_EMAIL} and {BOOTSTRAP_ADMIN_PASSWORD}, or neither"
                )
            }
            BootstrapAdminError::InvalidEmail => write!(f, "invalid bootstrap admin email"),
            BootstrapAdminError::InvalidAlias => write!(f, "invalid bootstrap admin alias"),
            BootstrapAdminError::InvalidPassword => write!(f, "invalid bootstrap admin password"),
            BootstrapAdminError::MissingPepper => {
                write!(f, "{AUTH_HASH_PEPPER} is missing or not 32 bytes")
            }
            BootstrapAdminError::Hash(err) => write!(f, "{err}"),
            BootstrapAdminError::Db(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for BootstrapAdminError {}

impl From<DbError> for BootstrapAdminError {
    fn from(value: DbError) -> Self {
        BootstrapAdminError::Db(value)
    }
}

impl From<PasswordHashError> for BootstrapAdminError {
    fn from(value: PasswordHashError) -> Self {
        BootstrapAdminError::Hash(value)
    }
}

/// Read `RUNDTISCH_BOOTSTRAP_ADMIN_*` values from `get`.
///
/// Missing or whitespace-only values count as unset. Both email and password
/// must be set, or both unset. Alias is optional.
pub fn read_bootstrap_admin_secrets<F>(
    mut get: F,
) -> Result<Option<BootstrapAdminSecrets>, BootstrapAdminError>
where
    F: FnMut(&str) -> Option<String>,
{
    let email = get(BOOTSTRAP_ADMIN_EMAIL).and_then(non_empty);
    let password = get(BOOTSTRAP_ADMIN_PASSWORD).and_then(non_empty);
    let alias = get(BOOTSTRAP_ADMIN_ALIAS).and_then(non_empty);
    match (email, password) {
        (None, None) => Ok(None),
        (Some(email), Some(password)) => {
            Ok(Some(BootstrapAdminSecrets::new(email, password, alias)))
        }
        _ => Err(BootstrapAdminError::IncompleteSecrets),
    }
}

pub fn read_bootstrap_admin_from_env() -> Result<Option<BootstrapAdminSecrets>, BootstrapAdminError>
{
    read_bootstrap_admin_secrets(|name| std::env::var(name).ok())
}

/// Upsert a bootstrap admin for `secrets.email`.
///
/// A missing row becomes a verified Admin. An existing row keeps its
/// `public_id`, role, alias, and verification; only the password hash and
/// `updated_at` change. A unique-constraint race retries as an update.
pub async fn seed_bootstrap_admin(
    db: &DatabaseConnection,
    secrets: &BootstrapAdminSecrets,
    hasher: &dyn PasswordHasher,
) -> Result<BootstrapAdminOutcome, BootstrapAdminError> {
    let email = parse_email(secrets.email())?;
    if check_password_policy(secrets.password()).is_err() {
        return Err(BootstrapAdminError::InvalidPassword);
    }
    let password_hash = hasher.hash(secrets.password())?;
    let now = OffsetDateTime::now_utc();
    if let Some(existing) = get_user_by_email(db, email.as_ref()).await? {
        update_user_password_hash(db, existing.public_id, password_hash, now).await?;
        return Ok(BootstrapAdminOutcome::Updated {
            email: email.to_string(),
            public_id: existing.public_id,
        });
    }
    let alias = resolve_alias(secrets.alias(), &email)?;
    let mut new_user = NewUser::new(email.clone(), alias, Role::Admin, Some(password_hash));
    new_user.assign_public_id();
    new_user.stamp_now(now);
    match insert_verified_user(db, &new_user, now).await {
        Ok(_) => Ok(BootstrapAdminOutcome::Inserted {
            email: email.to_string(),
            public_id: new_user.public_id,
        }),
        Err(DbError::Conflict) => {
            let hash = new_user
                .password_hash
                .ok_or(BootstrapAdminError::Db(DbError::TypeMismatch))?;
            update_existing_password(db, email.as_ref(), hash, now).await
        }
        Err(err) => Err(err.into()),
    }
}

async fn update_existing_password(
    db: &DatabaseConnection,
    email: &str,
    password_hash: String,
    now: OffsetDateTime,
) -> Result<BootstrapAdminOutcome, BootstrapAdminError> {
    let existing = get_user_by_email(db, email)
        .await?
        .ok_or_else(|| BootstrapAdminError::Db(DbError::Conflict))?;
    update_user_password_hash(db, existing.public_id, password_hash, now).await?;
    Ok(BootstrapAdminOutcome::Updated {
        email: email.to_string(),
        public_id: existing.public_id,
    })
}

/// Env-driven entry used by the migrate binary.
///
/// Unset secrets are a no-op. When both email and password are set, pepper
/// is required so the password can be hashed for insert or reset.
pub async fn seed_bootstrap_admin_from_env(
    db: &DatabaseConnection,
) -> Result<BootstrapAdminOutcome, BootstrapAdminError> {
    let Some(secrets) = read_bootstrap_admin_from_env()? else {
        return Ok(BootstrapAdminOutcome::SkippedUnset);
    };
    let hasher = argon_hasher_from_env()?;
    seed_bootstrap_admin(db, &secrets, &hasher).await
}

fn non_empty(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

fn parse_email(value: &str) -> Result<EmailAddress, BootstrapAdminError> {
    value
        .trim()
        .parse()
        .map_err(|_| BootstrapAdminError::InvalidEmail)
}

fn resolve_alias(alias: Option<&str>, email: &EmailAddress) -> Result<String, BootstrapAdminError> {
    if let Some(alias) = alias {
        let trimmed = alias.trim();
        if trimmed.is_empty() {
            return Err(BootstrapAdminError::InvalidAlias);
        }
        return Ok(trimmed.to_string());
    }
    Ok(email
        .as_ref()
        .split('@')
        .next()
        .filter(|part| !part.is_empty())
        .unwrap_or("admin")
        .to_string())
}

fn argon_hasher_from_env() -> Result<Argon2idHasher, BootstrapAdminError> {
    let pepper = std::env::var(AUTH_HASH_PEPPER).map_err(|_| BootstrapAdminError::MissingPepper)?;
    let bytes = pepper.into_bytes();
    if bytes.len() != 32 {
        return Err(BootstrapAdminError::MissingPepper);
    }
    Ok(Argon2idHasher::new(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::migrations;
    use crate::auth::models::{NewUser, Role};
    use crate::auth::password::TestPasswordHasher;
    use crate::auth::queries::{get_user_by_email, insert_user};
    use sea_orm_migration::MigratorTrait;

    struct Migrator;

    impl MigratorTrait for Migrator {
        fn migrations() -> Vec<Box<dyn sea_orm_migration::MigrationTrait>> {
            migrations::migrations()
        }
    }

    async fn db() -> DatabaseConnection {
        let db = sea_orm::Database::connect("sqlite::memory:")
            .await
            .expect("sqlite");
        Migrator::up(&db, None).await.expect("migrate");
        db
    }

    fn map_get<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl FnMut(&str) -> Option<String> + 'a {
        move |name| {
            pairs
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| (*value).to_string())
        }
    }

    #[test]
    fn prefix_matches_secret_names() {
        assert!(BOOTSTRAP_ADMIN_EMAIL.starts_with(BOOTSTRAP_ADMIN_PREFIX));
        assert!(BOOTSTRAP_ADMIN_PASSWORD.starts_with(BOOTSTRAP_ADMIN_PREFIX));
        assert!(BOOTSTRAP_ADMIN_ALIAS.starts_with(BOOTSTRAP_ADMIN_PREFIX));
    }

    #[test]
    fn read_secrets_unset_when_both_missing() {
        let secrets = read_bootstrap_admin_secrets(map_get(&[])).expect("read");
        assert!(secrets.is_none());
    }

    #[test]
    fn read_secrets_unset_when_values_are_whitespace() {
        let secrets = read_bootstrap_admin_secrets(map_get(&[
            (BOOTSTRAP_ADMIN_EMAIL, "  "),
            (BOOTSTRAP_ADMIN_PASSWORD, "\n"),
        ]))
        .expect("read");
        assert!(secrets.is_none());
    }

    #[test]
    fn read_secrets_rejects_only_email() {
        let err =
            read_bootstrap_admin_secrets(map_get(&[(BOOTSTRAP_ADMIN_EMAIL, "admin@example.com")]))
                .expect_err("incomplete");
        assert!(matches!(err, BootstrapAdminError::IncompleteSecrets));
    }

    #[test]
    fn read_secrets_rejects_only_password() {
        let err = read_bootstrap_admin_secrets(map_get(&[(
            BOOTSTRAP_ADMIN_PASSWORD,
            "unique-passphrase-ok",
        )]))
        .expect_err("incomplete");
        assert!(matches!(err, BootstrapAdminError::IncompleteSecrets));
    }

    #[test]
    fn read_secrets_returns_email_password_and_optional_alias() {
        let secrets = read_bootstrap_admin_secrets(map_get(&[
            (BOOTSTRAP_ADMIN_EMAIL, "admin@example.com"),
            (BOOTSTRAP_ADMIN_PASSWORD, "unique-passphrase-ok"),
            (BOOTSTRAP_ADMIN_ALIAS, "root"),
        ]))
        .expect("read")
        .expect("set");
        assert_eq!(secrets.email(), "admin@example.com");
        assert_eq!(secrets.password(), "unique-passphrase-ok");
        assert_eq!(secrets.alias(), Some("root"));
    }

    #[test]
    fn debug_redacts_password() {
        let secrets = BootstrapAdminSecrets::new("admin@example.com", "unique-passphrase-ok", None);
        let debug = format!("{secrets:?}");
        assert!(debug.contains("admin@example.com"));
        assert!(!debug.contains("unique-passphrase-ok"));
        assert!(debug.contains("***"));
    }

    #[tokio::test]
    async fn insert_creates_verified_admin_when_email_is_absent() {
        let db = db().await;
        let secrets = BootstrapAdminSecrets::new(
            "admin@example.com",
            "unique-passphrase-ok",
            Some("root".into()),
        );
        let outcome = seed_bootstrap_admin(&db, &secrets, &TestPasswordHasher)
            .await
            .expect("seed");
        let BootstrapAdminOutcome::Inserted { email, public_id } = outcome else {
            panic!("expected insert, got {outcome:?}");
        };
        assert_eq!(email, "admin@example.com");
        let user = get_user_by_email(&db, "admin@example.com")
            .await
            .expect("lookup")
            .expect("row");
        assert_eq!(user.public_id, public_id);
        assert_eq!(user.alias, "root");
        assert_eq!(user.role, Role::Admin);
        assert_eq!(
            user.password_hash.as_deref(),
            Some("test:unique-passphrase-ok")
        );
        assert!(user.email_verified_at.is_some());
    }

    #[tokio::test]
    async fn insert_derives_alias_from_email_local_part() {
        let db = db().await;
        let secrets =
            BootstrapAdminSecrets::new("bootstrap@example.com", "unique-passphrase-ok", None);
        seed_bootstrap_admin(&db, &secrets, &TestPasswordHasher)
            .await
            .expect("seed");
        let user = get_user_by_email(&db, "bootstrap@example.com")
            .await
            .expect("lookup")
            .expect("row");
        assert_eq!(user.alias, "bootstrap");
        assert_eq!(user.role, Role::Admin);
    }

    #[tokio::test]
    async fn upsert_resets_password_and_keeps_the_same_user() {
        let db = db().await;
        let mut existing = NewUser::new(
            "admin@example.com".parse().unwrap(),
            "plain",
            Role::User,
            Some("test:already-set-pass".into()),
        );
        existing.assign_public_id();
        insert_user(&db, &existing).await.expect("insert user");

        let secrets = BootstrapAdminSecrets::new(
            "admin@example.com",
            "unique-passphrase-ok",
            Some("root".into()),
        );
        let outcome = seed_bootstrap_admin(&db, &secrets, &TestPasswordHasher)
            .await
            .expect("seed");
        assert_eq!(
            outcome,
            BootstrapAdminOutcome::Updated {
                email: "admin@example.com".into(),
                public_id: existing.public_id,
            }
        );
        let user = get_user_by_email(&db, "admin@example.com")
            .await
            .expect("lookup")
            .expect("row");
        assert_eq!(user.public_id, existing.public_id);
        assert_eq!(user.role, Role::User);
        assert_eq!(user.alias, "plain");
        assert_eq!(
            user.password_hash.as_deref(),
            Some("test:unique-passphrase-ok")
        );
        assert!(user.email_verified_at.is_none());
    }

    #[tokio::test]
    async fn second_seed_resets_password_without_changing_public_id() {
        let db = db().await;
        let first = BootstrapAdminSecrets::new(
            "admin@example.com",
            "unique-passphrase-ok",
            Some("root".into()),
        );
        let inserted = seed_bootstrap_admin(&db, &first, &TestPasswordHasher)
            .await
            .expect("insert");
        let BootstrapAdminOutcome::Inserted { public_id, .. } = inserted else {
            panic!("expected insert, got {inserted:?}");
        };

        let second = BootstrapAdminSecrets::new(
            "admin@example.com",
            "another-passphrase-ok",
            Some("other".into()),
        );
        let updated = seed_bootstrap_admin(&db, &second, &TestPasswordHasher)
            .await
            .expect("update");
        assert_eq!(
            updated,
            BootstrapAdminOutcome::Updated {
                email: "admin@example.com".into(),
                public_id,
            }
        );
        let user = get_user_by_email(&db, "admin@example.com")
            .await
            .expect("lookup")
            .expect("row");
        assert_eq!(user.public_id, public_id);
        assert_eq!(user.alias, "root");
        assert_eq!(user.role, Role::Admin);
        assert_eq!(
            user.password_hash.as_deref(),
            Some("test:another-passphrase-ok")
        );
        assert!(user.email_verified_at.is_some());
    }

    #[tokio::test]
    async fn upsert_rejects_invalid_password_without_changing_existing_row() {
        let db = db().await;
        let mut existing = NewUser::new(
            "admin@example.com".parse().unwrap(),
            "plain",
            Role::User,
            Some("test:already-set-pass".into()),
        );
        existing.assign_public_id();
        insert_user(&db, &existing).await.expect("insert user");

        let secrets = BootstrapAdminSecrets::new("admin@example.com", "short", None);
        let err = seed_bootstrap_admin(&db, &secrets, &TestPasswordHasher)
            .await
            .expect_err("invalid password");
        assert!(matches!(err, BootstrapAdminError::InvalidPassword));
        let user = get_user_by_email(&db, "admin@example.com")
            .await
            .expect("lookup")
            .expect("row");
        assert_eq!(user.password_hash.as_deref(), Some("test:already-set-pass"));
        assert_eq!(user.public_id, existing.public_id);
    }

    #[tokio::test]
    async fn insert_rejects_invalid_password_without_writing_a_row() {
        let db = db().await;
        let secrets = BootstrapAdminSecrets::new("admin@example.com", "short", None);
        let err = seed_bootstrap_admin(&db, &secrets, &TestPasswordHasher)
            .await
            .expect_err("invalid password");
        assert!(matches!(err, BootstrapAdminError::InvalidPassword));
        assert!(
            get_user_by_email(&db, "admin@example.com")
                .await
                .expect("lookup")
                .is_none()
        );
    }

    #[tokio::test]
    async fn insert_rejects_invalid_email() {
        let db = db().await;
        let secrets = BootstrapAdminSecrets::new("not-an-email", "unique-passphrase-ok", None);
        let err = seed_bootstrap_admin(&db, &secrets, &TestPasswordHasher)
            .await
            .expect_err("invalid email");
        assert!(matches!(err, BootstrapAdminError::InvalidEmail));
    }

    #[tokio::test]
    async fn from_env_skips_when_bootstrap_secrets_are_unset() {
        let db = db().await;
        if read_bootstrap_admin_from_env().ok().flatten().is_some() {
            return;
        }
        let outcome = seed_bootstrap_admin_from_env(&db).await.expect("seed");
        assert_eq!(outcome, BootstrapAdminOutcome::SkippedUnset);
    }
}
