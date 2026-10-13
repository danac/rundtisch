use sea_orm::entity::prelude::*;

/// `pending` until [`super::super::ObjectStore::commit`] marks the row `committed`.
/// Stored as a string so the column is the same on SQLite, MySQL, and Postgres.
#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(16))")]
pub enum ObjectStatus {
    #[sea_orm(string_value = "pending")]
    Pending,
    #[sea_orm(string_value = "committed")]
    Committed,
}

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "storage_objects")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    #[sea_orm(unique)]
    pub public_id: Uuid,
    /// Library-generated relative key. Not part of [`super::super::ObjectInfo`].
    #[sea_orm(unique, column_type = "String(StringLen::N(512))")]
    pub storage_key: String,
    pub size: Option<i64>,
    pub content_type: String,
    #[sea_orm(column_type = "Binary(32)", nullable)]
    pub blake3: Option<Vec<u8>>,
    #[sea_orm(column_type = "Binary(32)", nullable)]
    pub sha256: Option<Vec<u8>>,
    pub original_filename: Option<String>,
    pub status: ObjectStatus,
    pub created_at: TimeDateTimeWithTimeZone,
    pub committed_at: Option<TimeDateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
