use crate::auth::models::{datetime_to_rfc3339, NewUser, UserTable};
use sea_query::{
    Asterisk, DeleteStatement, Expr, ExprTrait, InsertStatement, Query, SelectStatement,
    UpdateStatement,
};
use uuid::Uuid;

pub fn user_list_query() -> SelectStatement {
    Query::select()
        .column(Asterisk)
        .from(UserTable::Table)
        .to_owned()
}

pub fn user_get_query(public_id: Uuid) -> SelectStatement {
    Query::select()
        .column(Asterisk)
        .from(UserTable::Table)
        .and_where(Expr::col(UserTable::PublicId).eq(public_id.to_string()))
        .to_owned()
}

pub fn user_insert_query(user: &NewUser) -> InsertStatement {
    Query::insert()
        .into_table(UserTable::Table)
        .columns([
            UserTable::PublicId,
            UserTable::Email,
            UserTable::Alias,
            UserTable::Role,
            UserTable::PasswordHash,
            UserTable::CreatedAt,
            UserTable::UpdatedAt,
        ])
        .values_panic([
            user.public_id.to_string().into(),
            user.email.to_string().into(),
            user.alias.clone().into(),
            user.role.as_str().into(),
            user.password_hash.clone().into(),
            datetime_to_rfc3339(user.created_at).into(),
            datetime_to_rfc3339(user.updated_at).into(),
        ])
        .to_owned()
}

pub fn user_update_alias_query(
    public_id: Uuid,
    alias: &str,
    updated_at: time::OffsetDateTime,
) -> UpdateStatement {
    Query::update()
        .table(UserTable::Table)
        .values([
            (UserTable::Alias, alias.into()),
            (
                UserTable::UpdatedAt,
                datetime_to_rfc3339(updated_at).into(),
            ),
        ])
        .and_where(Expr::col(UserTable::PublicId).eq(public_id.to_string()))
        .to_owned()
}

pub fn user_delete_query(public_id: Uuid) -> DeleteStatement {
    Query::delete()
        .from_table(UserTable::Table)
        .and_where(Expr::col(UserTable::PublicId).eq(public_id.to_string()))
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::migrations::auth_migration_001::AuthMigration001;
    use crate::auth::models::Role;
    use crate::traits::db::{query_to_sql, schema_to_sql, Dialect, Error, Migration, Value};
    use email_address::EmailAddress;

    fn sample_user() -> NewUser {
        let created = time::OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
        let updated = time::OffsetDateTime::from_unix_timestamp(1_700_000_100).unwrap();
        NewUser {
            public_id: uuid::Uuid::parse_str("11111111-1111-4111-8111-111111111111").unwrap(),
            email: "alice@example.com".parse().unwrap(),
            alias: "alice".into(),
            role: Role::Admin,
            password_hash: Some("hash".into()),
            created_at: created,
            updated_at: updated,
        }
    }

    fn bind<'q>(
        sql: &'q str,
        values: &'q [Value],
    ) -> sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments> {
        let mut q = sqlx::query(sqlx::AssertSqlSafe(sql));
        for v in values {
            q = match v {
                Value::Int(x) => q.bind(*x),
                Value::Float(x) => q.bind(*x),
                Value::Text(x) => q.bind(x.as_str()),
                Value::Bool(x) => q.bind(*x),
                Value::Bytes(x) => q.bind(x.as_slice()),
                Value::Null => q.bind(Option::<i64>::None),
            };
        }
        q
    }

    async fn migrated_pool() -> sqlx::SqlitePool {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:")
            .await
            .expect("connect to in-memory sqlite");
        for stmt in AuthMigration001.up() {
            let sql = schema_to_sql(&stmt, Dialect::Sqlite);
            sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
                .execute(&pool)
                .await
                .expect("execute migration statement");
        }
        pool
    }

    #[test]
    fn user_insert_query_sql() {
        let (sql, values) = query_to_sql(&user_insert_query(&sample_user()), Dialect::Sqlite)
            .expect("render insert");
        assert_eq!(
            sql,
            "INSERT INTO \"auth_users\" (\"public_id\", \"email\", \"alias\", \"role\", \"password_hash\", \"created_at\", \"updated_at\") VALUES (?, ?, ?, ?, ?, ?, ?)"
        );
        assert_eq!(
            values
                .iter()
                .map(|v| match v {
                    Value::Text(s) => s.as_str(),
                    Value::Null => "NULL",
                    _ => panic!("unexpected bind {v:?}"),
                })
                .collect::<Vec<_>>(),
            [
                "11111111-1111-4111-8111-111111111111",
                "alice@example.com",
                "alice",
                "Admin",
                "hash",
                "2023-11-14T22:13:20Z",
                "2023-11-14T22:15:00Z",
            ]
        );
    }

    #[test]
    fn user_delete_query_sql() {
        let public_id = uuid::Uuid::parse_str("11111111-1111-4111-8111-111111111111").unwrap();
        let (sql, values) =
            query_to_sql(&user_delete_query(public_id), Dialect::Sqlite).expect("render delete");
        assert_eq!(sql, "DELETE FROM \"auth_users\" WHERE \"public_id\" = ?");
        match values.as_slice() {
            [Value::Text(s)] if s == "11111111-1111-4111-8111-111111111111" => {}
            other => panic!("unexpected binds: {other:?}"),
        }
    }

    #[test]
    fn user_get_query_sql() {
        let public_id = uuid::Uuid::parse_str("22222222-2222-4222-8222-222222222222").unwrap();
        let (sql, values) =
            query_to_sql(&user_get_query(public_id), Dialect::Sqlite).expect("render get");
        assert_eq!(sql, "SELECT * FROM \"auth_users\" WHERE \"public_id\" = ?");
        match values.as_slice() {
            [Value::Text(s)] if s == "22222222-2222-4222-8222-222222222222" => {}
            other => panic!("unexpected binds: {other:?}"),
        }
    }

    #[test]
    fn user_update_alias_query_sql() {
        let public_id = uuid::Uuid::parse_str("33333333-3333-4333-8333-333333333333").unwrap();
        let updated = time::OffsetDateTime::from_unix_timestamp(1_700_000_200).unwrap();
        let (sql, values) =
            query_to_sql(&user_update_alias_query(public_id, "bob", updated), Dialect::Sqlite)
                .expect("render update");
        assert_eq!(
            sql,
            "UPDATE \"auth_users\" SET \"alias\" = ?, \"updated_at\" = ? WHERE \"public_id\" = ?"
        );
        assert_eq!(
            values
                .iter()
                .map(|v| match v {
                    Value::Text(s) => s.as_str(),
                    other => panic!("unexpected bind {other:?}"),
                })
                .collect::<Vec<_>>(),
            ["bob", "2023-11-14T22:16:40Z", "33333333-3333-4333-8333-333333333333"]
        );
    }

    #[tokio::test]
    async fn insert_list_delete_round_trip() {
        let pool = migrated_pool().await;
        let user = sample_user();
        let (sql, values) =
            query_to_sql(&user_insert_query(&user), Dialect::Sqlite).expect("render insert");
        bind(&sql, &values)
            .execute(&pool)
            .await
            .expect("insert user");

        let (sql, values) =
            query_to_sql(&user_list_query(), Dialect::Sqlite).expect("render list");
        let row = bind(&sql, &values)
            .fetch_one(&pool)
            .await
            .expect("list users");
        let email: String = sqlx::Row::try_get(&row, "email").unwrap();
        let alias: String = sqlx::Row::try_get(&row, "alias").unwrap();
        let role: String = sqlx::Row::try_get(&row, "role").unwrap();
        assert_eq!(email, "alice@example.com");
        assert_eq!(alias, "alice");
        assert_eq!(role, "Admin");

        let (sql, values) =
            query_to_sql(&user_delete_query(user.public_id), Dialect::Sqlite).expect("render delete");
        let deleted = bind(&sql, &values)
            .execute(&pool)
            .await
            .expect("delete user");
        assert_eq!(deleted.rows_affected(), 1);

        let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM auth_users")
            .fetch_one(&pool)
            .await
            .expect("count users");
        assert_eq!(remaining, 0);
    }

    #[tokio::test]
    async fn delete_missing_user_affects_no_rows() {
        let pool = migrated_pool().await;
        let missing = uuid::Uuid::parse_str("99999999-9999-4999-8999-999999999999").unwrap();
        let (sql, values) =
            query_to_sql(&user_delete_query(missing), Dialect::Sqlite).expect("render delete");
        let deleted = bind(&sql, &values)
            .execute(&pool)
            .await
            .expect("delete missing");
        assert_eq!(deleted.rows_affected(), 0);
    }

    #[tokio::test]
    async fn insert_duplicate_public_id_is_unique_violation() {
        let pool = migrated_pool().await;
        let first = sample_user();
        let (sql, values) =
            query_to_sql(&user_insert_query(&first), Dialect::Sqlite).expect("render insert");
        bind(&sql, &values)
            .execute(&pool)
            .await
            .expect("insert first user");

        let mut second = sample_user();
        second.email = "other@example.com".parse().unwrap();
        let (sql, values) =
            query_to_sql(&user_insert_query(&second), Dialect::Sqlite).expect("render insert");
        let err = bind(&sql, &values)
            .execute(&pool)
            .await
            .expect_err("duplicate public_id");
        assert!(err.to_string().to_ascii_lowercase().contains("unique"));
    }

    #[tokio::test]
    async fn insert_duplicate_email_is_unique_violation() {
        let pool = migrated_pool().await;
        let user = sample_user();
        let (sql, values) =
            query_to_sql(&user_insert_query(&user), Dialect::Sqlite).expect("render insert");
        bind(&sql, &values)
            .execute(&pool)
            .await
            .expect("insert first user");
        let err = bind(&sql, &values)
            .execute(&pool)
            .await
            .expect_err("duplicate email");
        assert!(err.to_string().to_ascii_lowercase().contains("unique"));
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn sqlite_executor_decodes_inserted_user() {
        use crate::adapters::db::sqlite::SqliteExecutor;
        use crate::auth::models::User;
        use crate::traits::db::DatabaseExecutor;
        use time::format_description::well_known::Rfc3339;

        let pool = migrated_pool().await;
        let exec = SqliteExecutor::from_pool(pool);
        let new_user = sample_user();
        let id = exec
            .insert(user_insert_query(&new_user))
            .await
            .expect("insert");
        let users: Vec<User> = exec.fetch_all(user_list_query()).await.expect("list");
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].id, id);
        assert_eq!(users[0].public_id, new_user.public_id);
        assert_eq!(users[0].email, new_user.email);
        assert_eq!(users[0].alias, new_user.alias);
        assert_eq!(users[0].role, Role::Admin);
        assert_eq!(users[0].password_hash.as_deref(), Some("hash"));
        assert_eq!(users[0].email_verified_at, None);
        assert_eq!(
            users[0].created_at.format(&Rfc3339).unwrap(),
            datetime_to_rfc3339(new_user.created_at)
        );
        assert_eq!(users[0].last_login_at, None);

        let deleted = exec
            .execute(user_delete_query(new_user.public_id))
            .await
            .expect("delete");
        assert_eq!(deleted.rows_affected, 1);
        let users: Vec<User> = exec.fetch_all(user_list_query()).await.expect("list empty");
        assert!(users.is_empty());
    }

    #[test]
    fn new_user_parses_email() {
        let email: EmailAddress = "bob@example.com".parse().unwrap();
        let user = NewUser::new(email.clone(), "bob", Role::User, None);
        assert_eq!(user.email, email);
        assert_eq!(user.alias, "bob");
        assert_eq!(user.role, Role::User);
        assert_eq!(user.password_hash, None);
        assert_ne!(user.public_id, uuid::Uuid::nil());
    }

    #[test]
    fn conflict_error_display() {
        assert_eq!(Error::Conflict.to_string(), "conflict");
        assert_eq!(Error::NotFound.to_string(), "not found");
    }
}
