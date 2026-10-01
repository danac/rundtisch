use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::{uuid, uuid_null};
use sea_orm_migration::sea_orm::DatabaseBackend;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Session::Table)
                    .add_column(uuid_null(Session::PublicId))
                    .to_owned(),
            )
            .await?;

        let select = Query::select()
            .column(Session::Id)
            .from(Session::Table)
            .to_owned();
        let rows = manager.get_connection().query_all(&select).await?;
        for row in rows {
            let id: i64 = row.try_get("", "id")?;
            let update = Query::update()
                .table(Session::Table)
                .value(Session::PublicId, crate::auth::models::new_public_id())
                .and_where(Expr::col(Session::Id).eq(id))
                .to_owned();
            manager.get_connection().execute(&update).await?;
        }

        if matches!(
            manager.get_database_backend(),
            DatabaseBackend::MySql | DatabaseBackend::Postgres
        ) {
            manager
                .alter_table(
                    Table::alter()
                        .table(Session::Table)
                        .modify_column(uuid(Session::PublicId))
                        .to_owned(),
                )
                .await?;
        }

        manager
            .create_index(
                Index::create()
                    .name("idx_auth_sessions_public_id")
                    .table(Session::Table)
                    .col(Session::PublicId)
                    .unique()
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_auth_sessions_public_id")
                    .table(Session::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Session::Table)
                    .drop_column(Session::PublicId)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Session {
    #[sea_orm(iden = "auth_sessions")]
    Table,
    Id,
    PublicId,
}

#[cfg(test)]
mod tests {
    use super::super::Migrator;
    use sea_orm::ConnectionTrait;
    use sea_orm::sea_query::{Alias, ExprTrait, Query};
    use sea_orm_migration::MigratorTrait;

    #[tokio::test]
    async fn backfills_public_ids_for_existing_sessions() {
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        Migrator::up(&db, Some(5)).await.unwrap();
        db.execute_unprepared(
            "INSERT INTO auth_users \
             (public_id, email, alias, role, password_hash, email_verified_at, created_at, updated_at, last_login_at) \
             VALUES ('00000000-0000-4000-8000-000000000001', 'old@example.com', 'old', 'User', NULL, NULL, \
             '2026-09-29T00:00:00Z', '2026-09-29T00:00:00Z', NULL)",
        )
        .await
        .unwrap();
        db.execute_unprepared(
            "INSERT INTO auth_sessions \
             (user_id, token_hash, created_at, last_used_at, expires_at, revoked_at, user_agent) \
             VALUES (1, 'old-token-hash', '2026-09-29T00:00:00Z', '2026-09-29T00:00:00Z', \
             '2026-10-13T00:00:00Z', NULL, NULL)",
        )
        .await
        .unwrap();

        Migrator::up(&db, None).await.unwrap();

        let select = Query::select()
            .column(Alias::new("public_id"))
            .from(Alias::new("auth_sessions"))
            .and_where(sea_orm::sea_query::Expr::col(Alias::new("id")).eq(1))
            .to_owned();
        let row = db.query_one(&select).await.unwrap().unwrap();
        let public_id: Option<uuid::Uuid> = row.try_get("", "public_id").unwrap();
        assert!(public_id.is_some());
    }
}
