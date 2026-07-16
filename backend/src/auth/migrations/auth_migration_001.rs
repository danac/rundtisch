use sea_query::{ColumnDef, ForeignKey, ForeignKeyAction, Table};
use crate::traits::db::{Migration, Statement};
use crate::auth::models::*;

pub struct AuthMigration001;

impl Migration for AuthMigration001 {
    fn name(&self) -> &str { "auth_001_create_user_email_password_tables" }

    fn up(&self) -> Vec<Statement> {
        vec![
            Statement::CreateTable(
                Table::create()
                    .table(UserTable::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(UserTable::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(UserTable::Role).string().not_null())
                    .col(ColumnDef::new(UserTable::Alias).string().not_null())
                    .col(ColumnDef::new(UserTable::CreatedAt).string().not_null())
                    .col(ColumnDef::new(UserTable::UpdatedAt).string().not_null())
                    .col(ColumnDef::new(UserTable::DeletedAt).string().null())
                    .col(ColumnDef::new(UserTable::LastLoginAt).string().null())
                    .col(ColumnDef::new(UserTable::LockedUntil).string().null())
                    .col(ColumnDef::new(UserTable::FailedLoginAttempts).integer().not_null().default(0))
                    .to_owned()
            ),
            Statement::CreateTable(
                Table::create()
                    .table(EmailTable::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(EmailTable::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(EmailTable::EmailAddress).string().not_null().unique_key())
                    .col(ColumnDef::new(EmailTable::IsPrimary).boolean().not_null())
                    .col(ColumnDef::new(EmailTable::UserId).integer().not_null())
                    .col(ColumnDef::new(EmailTable::VerifiedAt).string().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_email_user_id")
                            .from_tbl(EmailTable::Table)
                            .from_col(EmailTable::UserId)
                            .to_tbl(UserTable::Table)
                            .to_col(UserTable::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .to_owned()
            ),
            Statement::CreateTable(
                Table::create()
                    .table(PasswordTable::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(PasswordTable::UserId).integer().not_null().primary_key())
                    .col(ColumnDef::new(PasswordTable::Hash).string().not_null())
                    .col(ColumnDef::new(PasswordTable::CreatedAt).string().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_password_user_id")
                            .from_tbl(PasswordTable::Table)
                            .from_col(PasswordTable::UserId)
                            .to_tbl(UserTable::Table)
                            .to_col(UserTable::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .to_owned()
            ),
            Statement::CreateTable(
                Table::create()
                    .table(PasswordHistoryTable::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(PasswordHistoryTable::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(PasswordHistoryTable::UserId).integer().not_null())
                    .col(ColumnDef::new(PasswordHistoryTable::Hash).string().not_null())
                    .col(ColumnDef::new(PasswordHistoryTable::CreatedAt).string().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_password_history_user_id")
                            .from_tbl(PasswordHistoryTable::Table)
                            .from_col(PasswordHistoryTable::UserId)
                            .to_tbl(UserTable::Table)
                            .to_col(UserTable::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .to_owned()
            ),
        ]
    }

    fn down(&self) -> Vec<Statement> {
        vec![
            Statement::DropTable(
                Table::drop()
                    .table(PasswordHistoryTable::Table)
                    .to_owned()
            ),
            Statement::DropTable(
                Table::drop()
                    .table(PasswordTable::Table)
                    .to_owned()
            ),
            Statement::DropTable(
                Table::drop()
                    .table(EmailTable::Table)
                    .to_owned()
            ),
            Statement::DropTable(
                Table::drop()
                    .table(UserTable::Table)
                    .to_owned()
            ),
        ]
    }
}

#[cfg(all(test, feature = "native"))]
mod tests {
    use super::*;
    use crate::traits::db::Dialect;
    use sea_query::Iden;

    async fn execute_statements(pool: &sqlx::SqlitePool, statements: Vec<Statement>) {
        for stmt in statements {
            let sql = stmt.to_sql(Dialect::Sqlite);
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
            EmailTable::Table.to_string(),
            PasswordTable::Table.to_string(),
            PasswordHistoryTable::Table.to_string(),
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
