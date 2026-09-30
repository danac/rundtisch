use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::{string_null, uuid, uuid_null};
use sea_orm_migration::sea_orm::DatabaseBackend;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Passkey::Table)
                    .add_column(uuid_null(Passkey::PublicId))
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Passkey::Table)
                    .add_column(string_null(Passkey::Label))
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Ceremony::Table)
                    .add_column(string_null(Ceremony::PasskeyLabel))
                    .to_owned(),
            )
            .await?;

        let select = Query::select()
            .column(Passkey::Id)
            .from(Passkey::Table)
            .to_owned();
        let rows = manager.get_connection().query_all(&select).await?;
        for row in rows {
            let id: i64 = row.try_get("", "id")?;
            let update = Query::update()
                .table(Passkey::Table)
                .value(Passkey::PublicId, crate::auth::models::new_public_id())
                .and_where(Expr::col(Passkey::Id).eq(id))
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
                        .table(Passkey::Table)
                        .modify_column(uuid(Passkey::PublicId))
                        .to_owned(),
                )
                .await?;
        }

        manager
            .create_index(
                Index::create()
                    .name("idx_auth_passkeys_public_id")
                    .table(Passkey::Table)
                    .col(Passkey::PublicId)
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
                    .name("idx_auth_passkeys_public_id")
                    .table(Passkey::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Ceremony::Table)
                    .drop_column(Ceremony::PasskeyLabel)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Passkey::Table)
                    .drop_column(Passkey::Label)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Passkey::Table)
                    .drop_column(Passkey::PublicId)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Passkey {
    #[sea_orm(iden = "auth_passkeys")]
    Table,
    Id,
    PublicId,
    Label,
}

#[derive(DeriveIden)]
enum Ceremony {
    #[sea_orm(iden = "auth_webauthn_state")]
    Table,
    PasskeyLabel,
}

#[cfg(test)]
mod tests {
    use super::super::Migrator;
    use sea_orm::ConnectionTrait;
    use sea_orm::sea_query::{Alias, ExprTrait, Query};
    use sea_orm_migration::MigratorTrait;

    #[tokio::test]
    async fn backfills_public_ids_for_existing_passkeys() {
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        Migrator::up(&db, Some(2)).await.unwrap();
        db.execute_unprepared(
            "INSERT INTO auth_users \
             (public_id, email, alias, role, password_hash, email_verified_at, created_at, updated_at, last_login_at) \
             VALUES ('00000000-0000-4000-8000-000000000001', 'old@example.com', 'old', 'User', NULL, NULL, \
             '2026-09-29T00:00:00Z', '2026-09-29T00:00:00Z', NULL)",
        )
        .await
        .unwrap();
        db.execute_unprepared(
            "INSERT INTO auth_passkeys \
             (user_id, credential_id, passkey, created_at, last_used_at) \
             VALUES (1, 'old-credential', '{}', '2026-09-29T00:00:00Z', NULL)",
        )
        .await
        .unwrap();

        Migrator::up(&db, None).await.unwrap();

        let select = Query::select()
            .columns([Alias::new("public_id"), Alias::new("label")])
            .from(Alias::new("auth_passkeys"))
            .and_where(sea_orm::sea_query::Expr::col(Alias::new("id")).eq(1))
            .to_owned();
        let row = db.query_one(&select).await.unwrap().unwrap();
        let public_id: Option<uuid::Uuid> = row.try_get("", "public_id").unwrap();
        let label: Option<String> = row.try_get("", "label").unwrap();
        assert!(public_id.is_some());
        assert_eq!(label, None);
    }
}
