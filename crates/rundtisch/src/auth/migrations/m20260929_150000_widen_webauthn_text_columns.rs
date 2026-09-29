use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::text;
use sea_orm_migration::sea_orm::DatabaseBackend;

/// Widen WebAuthn blob columns that were created as VARCHAR(255).
///
/// `string()` maps to VARCHAR(255) on MySQL. Serialized
/// `PasskeyRegistration` state is ~400+ bytes and stored passkeys are
/// larger, so inserts fail with a backend/data-too-long error on Edge.
/// SQLite does not enforce VARCHAR length, so this migration is a no-op there.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        match manager.get_database_backend() {
            DatabaseBackend::Sqlite => Ok(()),
            DatabaseBackend::MySql | DatabaseBackend::Postgres => {
                manager
                    .alter_table(
                        Table::alter()
                            .table(Ceremony::Table)
                            .modify_column(text(Ceremony::State))
                            .to_owned(),
                    )
                    .await?;
                manager
                    .alter_table(
                        Table::alter()
                            .table(Passkey::Table)
                            .modify_column(text(Passkey::Passkey))
                            .to_owned(),
                    )
                    .await?;
                Ok(())
            }
            other => Err(DbErr::Migration(format!(
                "unsupported database backend for webauthn text widen: {other:?}"
            ))),
        }
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Shrinking back to VARCHAR(255) would truncate live rows; leave as TEXT.
        let _ = manager;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Ceremony {
    #[sea_orm(iden = "auth_webauthn_state")]
    Table,
    State,
}

#[derive(DeriveIden)]
enum Passkey {
    #[sea_orm(iden = "auth_passkeys")]
    Table,
    Passkey,
}
