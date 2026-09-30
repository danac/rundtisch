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

        manager
            .create_table(
                Table::create()
                    .table(Passkey::Table)
                    .if_not_exists()
                    .col(pk_big_auto(Passkey::Id))
                    .col(big_integer(Passkey::UserId))
                    .col(string_len(Passkey::CredentialId, 512).unique_key())
                    // Serialized Passkey JSON exceeds MySQL's default VARCHAR(255).
                    .col(text(Passkey::Passkey))
                    .col(timestamp(Passkey::CreatedAt))
                    .col(timestamp_null(Passkey::LastUsedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_passkey_user_id")
                            .from(Passkey::Table, Passkey::UserId)
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
                    .name("idx_auth_passkeys_user_id")
                    .table(Passkey::Table)
                    .col(Passkey::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Recovery::Table)
                    .if_not_exists()
                    .col(pk_big_auto(Recovery::Id))
                    .col(big_integer(Recovery::UserId))
                    .col(string(Recovery::TokenHash).unique_key())
                    .col(timestamp(Recovery::CreatedAt))
                    .col(timestamp(Recovery::ExpiresAt))
                    .col(timestamp_null(Recovery::UsedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_recovery_user_id")
                            .from(Recovery::Table, Recovery::UserId)
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
                    .name("idx_auth_recovery_tokens_user_id")
                    .table(Recovery::Table)
                    .col(Recovery::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Invitation::Table)
                    .if_not_exists()
                    .col(pk_big_auto(Invitation::Id))
                    .col(string(Invitation::TokenHash).unique_key())
                    .col(string(Invitation::Email))
                    .col(timestamp(Invitation::CreatedAt))
                    .col(timestamp(Invitation::ExpiresAt))
                    .col(timestamp_null(Invitation::UsedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Ceremony::Table)
                    .if_not_exists()
                    .col(string(Ceremony::FlowId).primary_key())
                    .col(string(Ceremony::Kind))
                    .col(big_integer_null(Ceremony::UserId))
                    .col(string_null(Ceremony::TokenHash))
                    .col(string_null(Ceremony::Alias))
                    .col(uuid_null(Ceremony::PublicId))
                    // PasskeyRegistration / PasskeyAuthentication JSON is >255 bytes.
                    .col(text(Ceremony::State))
                    .col(timestamp(Ceremony::CreatedAt))
                    .col(timestamp(Ceremony::ExpiresAt))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Ceremony::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Invitation::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Recovery::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Passkey::Table).to_owned())
            .await?;
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

fn string_len<T: IntoIden>(name: T, len: u32) -> ColumnDef {
    ColumnDef::new(name).string_len(len).not_null().take()
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

#[derive(DeriveIden)]
enum Passkey {
    #[sea_orm(iden = "auth_passkeys")]
    Table,
    Id,
    UserId,
    CredentialId,
    Passkey,
    CreatedAt,
    LastUsedAt,
}

#[derive(DeriveIden)]
enum Recovery {
    #[sea_orm(iden = "auth_recovery_tokens")]
    Table,
    Id,
    UserId,
    TokenHash,
    CreatedAt,
    ExpiresAt,
    UsedAt,
}

#[derive(DeriveIden)]
enum Invitation {
    #[sea_orm(iden = "auth_invitations")]
    Table,
    Id,
    TokenHash,
    Email,
    CreatedAt,
    ExpiresAt,
    UsedAt,
}

#[derive(DeriveIden)]
enum Ceremony {
    #[sea_orm(iden = "auth_webauthn_state")]
    Table,
    FlowId,
    Kind,
    UserId,
    TokenHash,
    Alias,
    PublicId,
    State,
    CreatedAt,
    ExpiresAt,
}
