use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::{
    big_integer_null, big_pk_auto, string, string_len, string_len_uniq, string_null, timestamp,
    timestamp_null, uuid_uniq,
};

#[derive(DeriveMigrationName)]
pub struct Migration;

/// `storage_objects` is the library's source of truth for stored files.
///
/// `storage_key` is `VARCHAR(512)` rather than 1024. A unique index on
/// `utf8mb4` `VARCHAR(1024)` does not fit in MySQL InnoDB's 3072-byte index
/// limit. Generated keys are `{prefix/}{uuid}[.ext]` and stay well under 512.
///
/// `public_id` uses the same `uuid` column as auth (MySQL `binary(16)`).
/// `blake3` and `sha256` are raw 32-byte digests, null while the row is pending.
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(StorageObject::Table)
                    .if_not_exists()
                    .col(big_pk_auto(StorageObject::Id))
                    .col(uuid_uniq(StorageObject::PublicId))
                    .col(string_len_uniq(StorageObject::StorageKey, 512))
                    .col(big_integer_null(StorageObject::Size))
                    .col(string(StorageObject::ContentType))
                    .col(
                        ColumnDef::new(StorageObject::Blake3)
                            .binary_len(32)
                            .null()
                            .take(),
                    )
                    .col(
                        ColumnDef::new(StorageObject::Sha256)
                            .binary_len(32)
                            .null()
                            .take(),
                    )
                    .col(string_null(StorageObject::OriginalFilename))
                    .col(string_len(StorageObject::Status, 16))
                    .col(timestamp(StorageObject::CreatedAt))
                    .col(timestamp_null(StorageObject::CommittedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_storage_objects_status_created_at")
                    .table(StorageObject::Table)
                    .col(StorageObject::Status)
                    .col(StorageObject::CreatedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(StorageObject::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum StorageObject {
    #[sea_orm(iden = "storage_objects")]
    Table,
    Id,
    PublicId,
    StorageKey,
    Size,
    ContentType,
    Blake3,
    Sha256,
    OriginalFilename,
    Status,
    CreatedAt,
    CommittedAt,
}
