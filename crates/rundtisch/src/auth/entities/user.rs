use sea_orm::entity::prelude::*;

use crate::auth::models::Role;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "auth_users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    #[sea_orm(unique)]
    pub public_id: Uuid,
    #[sea_orm(unique)]
    pub email: String,
    pub alias: String,
    pub role: Role,
    pub password_hash: Option<String>,
    pub email_verified_at: Option<TimeDateTimeWithTimeZone>,
    pub created_at: TimeDateTimeWithTimeZone,
    pub updated_at: TimeDateTimeWithTimeZone,
    pub last_login_at: Option<TimeDateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::session::Entity")]
    Session,
    #[sea_orm(has_many = "super::passkey::Entity")]
    Passkey,
    #[sea_orm(has_many = "super::recovery_token::Entity")]
    RecoveryToken,
}

impl Related<super::session::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Session.def()
    }
}

impl Related<super::passkey::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Passkey.def()
    }
}

impl Related<super::recovery_token::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::RecoveryToken.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
