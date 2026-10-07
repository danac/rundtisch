use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::{
    big_integer, big_integer_null, big_pk_auto, integer, string, timestamp, uuid,
};

#[derive(DeriveMigrationName)]
pub struct Migration;

/// Collections and pictures.
///
/// `collections.thumbnail_picture_id` is a nullable pointer at `pictures.id`
/// with no database foreign key. A FK in both directions (collection → cover
/// picture, picture → collection) makes inserts and deletes order-dependent
/// on MySQL. The seed inserts the collection, then its pictures, then sets
/// the thumbnail.
///
/// `public_id` uses the same `uuid` column as the auth tables (MySQL
/// `binary(16)`). `storage_filename` stays a separate unique string.
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Collection::Table)
                    .if_not_exists()
                    .col(big_pk_auto(Collection::Id))
                    .col(uuid(Collection::PublicId).unique_key())
                    .col(string(Collection::Name))
                    .col(string(Collection::Description))
                    .col(timestamp(Collection::CreatedAt))
                    .col(big_integer_null(Collection::ThumbnailPictureId))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Picture::Table)
                    .if_not_exists()
                    .col(big_pk_auto(Picture::Id))
                    .col(uuid(Picture::PublicId).unique_key())
                    .col(big_integer(Picture::CollectionId))
                    .col(integer(Picture::Width))
                    .col(integer(Picture::Height))
                    .col(string(Picture::OriginalFilename))
                    .col(timestamp(Picture::CapturedAt))
                    .col(string(Picture::StorageFilename).unique_key())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_pictures_collection_id")
                            .from(Picture::Table, Picture::CollectionId)
                            .to(Collection::Table, Collection::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_pictures_collection_id")
                    .table(Picture::Table)
                    .col(Picture::CollectionId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Picture::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Collection::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Collection {
    #[sea_orm(iden = "collections")]
    Table,
    Id,
    PublicId,
    Name,
    Description,
    CreatedAt,
    ThumbnailPictureId,
}

#[derive(DeriveIden)]
enum Picture {
    #[sea_orm(iden = "pictures")]
    Table,
    Id,
    PublicId,
    CollectionId,
    Width,
    Height,
    OriginalFilename,
    CapturedAt,
    StorageFilename,
}
