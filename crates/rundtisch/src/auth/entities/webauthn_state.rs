use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "auth_webauthn_state")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub flow_id: String,
    pub kind: String,
    pub user_id: Option<i64>,
    pub token_hash: Option<String>,
    pub alias: Option<String>,
    pub passkey_label: Option<String>,
    pub public_id: Option<Uuid>,
    pub state: String,
    pub created_at: TimeDateTimeWithTimeZone,
    pub expires_at: TimeDateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
