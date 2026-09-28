use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(User::Table)
                    .if_not_exists()
                    .col(pk_big_auto(User::Id))
                    .col(uuid(User::PublicId).unique_key())
                    .col(string(User::Email).unique_key())
                    .col(string(User::Alias))
                    .col(string(User::Role))
                    .col(string_null(User::PasswordHash))
                    .col(timestamp_null(User::EmailVerifiedAt))
                    .col(timestamp(User::CreatedAt))
                    .col(timestamp(User::UpdatedAt))
                    .col(timestamp_null(User::LastLoginAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Session::Table)
                    .if_not_exists()
                    .col(pk_big_auto(Session::Id))
                    .col(big_integer(Session::UserId))
                    .col(string(Session::TokenHash).unique_key())
                    .col(timestamp(Session::CreatedAt))
                    .col(timestamp(Session::LastUsedAt))
                    .col(timestamp(Session::ExpiresAt))
                    .col(timestamp_null(Session::RevokedAt))
                    .col(string_null(Session::UserAgent))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_session_user_id")
                            .from(Session::Table, Session::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Session::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(User::Table).to_owned())
            .await?;
        Ok(())
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
enum User {
    #[sea_orm(iden = "auth_users")]
    Table,
    Id,
    PublicId,
    Email,
    Alias,
    Role,
    PasswordHash,
    EmailVerifiedAt,
    CreatedAt,
    UpdatedAt,
    LastLoginAt,
}

#[derive(DeriveIden)]
enum Session {
    #[sea_orm(iden = "auth_sessions")]
    Table,
    Id,
    UserId,
    TokenHash,
    CreatedAt,
    LastUsedAt,
    ExpiresAt,
    RevokedAt,
    UserAgent,
}
