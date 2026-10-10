use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "pictures")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    #[sea_orm(unique)]
    pub public_id: Uuid,
    pub collection_id: i64,
    /// Pixel width of the stored file.
    pub width: i32,
    /// Pixel height of the stored file.
    pub height: i32,
    pub original_filename: String,
    pub captured_at: TimeDateTimeWithTimeZone,
    /// File name under the collection storage folder. Unique UUID plus an extension.
    /// Separate from `public_id`.
    #[sea_orm(unique)]
    pub storage_filename: String,
    /// Raw BLAKE3 digest of the stored file (32 bytes).
    #[sea_orm(column_type = "Binary(32)")]
    pub checksum_blake3: Vec<u8>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
