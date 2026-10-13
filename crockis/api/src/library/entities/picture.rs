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
    pub captured_at: TimeDateTimeWithTimeZone,
    /// `storage_objects.id`. The file name, content type, and digests live on that row.
    #[sea_orm(unique)]
    pub object_id: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
