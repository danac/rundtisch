use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::{big_integer, string_len, timestamp, timestamp_null};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(StepUp::Table)
                    .if_not_exists()
                    .col(pk_big_auto(StepUp::Id))
                    .col(big_integer(StepUp::UserId))
                    .col(string_len(StepUp::TokenHash, 64).unique_key())
                    .col(timestamp(StepUp::CreatedAt))
                    .col(timestamp(StepUp::ExpiresAt))
                    .col(timestamp_null(StepUp::RevokedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_step_up_user_id")
                            .from(StepUp::Table, StepUp::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_auth_step_up_tokens_user_id")
                    .table(StepUp::Table)
                    .col(StepUp::UserId)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(StepUp::Table).to_owned())
            .await
    }
}

fn pk_big_auto<T: IntoIden>(name: T) -> ColumnDef {
    ColumnDef::new(name)
        .big_integer()
        .not_null()
        .auto_increment()
        .primary_key()
        .take()
}

#[derive(DeriveIden)]
enum StepUp {
    #[sea_orm(iden = "auth_step_up_tokens")]
    Table,
    Id,
    UserId,
    TokenHash,
    CreatedAt,
    ExpiresAt,
    RevokedAt,
}

#[derive(DeriveIden)]
enum User {
    #[sea_orm(iden = "auth_users")]
    Table,
    Id,
}
