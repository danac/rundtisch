use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "collections")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    #[sea_orm(unique)]
    pub public_id: String,
    pub name: String,
    pub description: String,
    pub created_at: TimeDateTimeWithTimeZone,
    pub sort_order: i32,
    /// Row id of a picture in this collection, or null when no cover is chosen.
    /// Not a database foreign key; see the library migration.
    pub thumbnail_picture_id: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
