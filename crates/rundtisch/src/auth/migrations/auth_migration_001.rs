use sea_query::{ColumnDef, ForeignKey, ForeignKeyAction, Table, TableStatement};
use crate::traits::db::{Migration, Statement};
use crate::auth::models::*;

pub struct AuthMigration001;

impl Migration for AuthMigration001 {
    fn name(&self) -> &str { "001_auth_create_users_and_token_tables" }

    fn up(&self) -> Vec<Statement> {
        vec![
            Statement::TableStatement(TableStatement::Create(
                Table::create()
                    .table(UserTable::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(UserTable::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(UserTable::Email).string().not_null().unique_key())
                    .col(ColumnDef::new(UserTable::Alias).string().not_null())
                    .col(ColumnDef::new(UserTable::Role).string().not_null())
                    .col(ColumnDef::new(UserTable::PasswordHash).string().null())
                    .col(ColumnDef::new(UserTable::EmailVerifiedAt).string().null())
                    .col(ColumnDef::new(UserTable::CreatedAt).string().not_null())
                    .col(ColumnDef::new(UserTable::UpdatedAt).string().not_null())
                    .col(ColumnDef::new(UserTable::LastLoginAt).string().null())
                    .to_owned()
            )),
            Statement::TableStatement(TableStatement::Create(
                Table::create()
                    .table(RefreshTokenTable::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(RefreshTokenTable::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(RefreshTokenTable::UserId).integer().not_null())
                    .col(ColumnDef::new(RefreshTokenTable::TokenHash).string().not_null().unique_key())
                    .col(ColumnDef::new(RefreshTokenTable::ExpiresAt).string().not_null())
                    .col(ColumnDef::new(RefreshTokenTable::Revoked).boolean().not_null().default(false))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_refresh_token_user_id")
                            .from_tbl(RefreshTokenTable::Table)
                            .from_col(RefreshTokenTable::UserId)
                            .to_tbl(UserTable::Table)
                            .to_col(UserTable::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .to_owned()
            )),
        ]
    }

    fn down(&self) -> Vec<Statement> {
        vec![
            Statement::TableStatement(TableStatement::Drop(
                Table::drop()
                    .table(RefreshTokenTable::Table)
                    .to_owned()
            )),
            Statement::TableStatement(TableStatement::Drop(
                Table::drop()
                    .table(UserTable::Table)
                    .to_owned()
            )),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::db::{schema_to_sql, Dialect};
    use sea_query::Iden;

    async fn execute_statements(pool: &sqlx::SqlitePool, statements: Vec<Statement>) {
        for stmt in statements {
            let sql = schema_to_sql(&stmt, Dialect::Sqlite);
            sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
                .execute(pool)
                .await
                .expect("execute migration statement");
        }
    }

    async fn list_user_tables(pool: &sqlx::SqlitePool) -> Vec<String> {
        sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
        )
            .fetch_all(pool)
            .await
            .expect("list tables")
    }

    fn auth_table_names() -> Vec<String> {
        vec![
            UserTable::Table.to_string(),
            RefreshTokenTable::Table.to_string(),
        ]
    }

    #[tokio::test]
    async fn up_then_down_is_inverse() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:")
            .await
            .expect("connect to in-memory sqlite");
        let migration = AuthMigration001;

        execute_statements(&pool, migration.up()).await;
        let mut expected = auth_table_names();
        expected.sort();
        assert_eq!(list_user_tables(&pool).await, expected);

        execute_statements(&pool, migration.down()).await;
        assert!(list_user_tables(&pool).await.is_empty());
    }
}
