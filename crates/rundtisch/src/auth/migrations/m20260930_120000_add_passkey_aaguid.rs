use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::uuid_null;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Passkey::Table)
                    .add_column(uuid_null(Passkey::Aaguid))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Passkey::Table)
                    .drop_column(Passkey::Aaguid)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Passkey {
    #[sea_orm(iden = "auth_passkeys")]
    Table,
    Aaguid,
}
