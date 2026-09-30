use sea_orm_migration::prelude::*;

mod m20260929_120000_create_auth_tables;
mod m20260929_150000_widen_webauthn_text_columns;
mod m20260929_180000_add_passkey_public_id_and_label;
mod m20260930_120000_add_passkey_aaguid;

/// Tracking table for auth migrations. A host database can keep its own
/// SeaORM history in `seaql_migrations`.
pub const MIGRATION_TABLE: &str = "rundtisch_migrations_auth";

/// Auth migrations, oldest first.
pub fn migrations() -> Vec<Box<dyn MigrationTrait>> {
    vec![
        Box::new(m20260929_120000_create_auth_tables::Migration),
        Box::new(m20260929_150000_widen_webauthn_text_columns::Migration),
        Box::new(m20260929_180000_add_passkey_public_id_and_label::Migration),
        Box::new(m20260930_120000_add_passkey_aaguid::Migration),
    ]
}

/// Applies [`migrations`] and records them in [`MIGRATION_TABLE`].
pub struct Migrator;

impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        migrations()
    }

    fn migration_table_name() -> sea_orm::DynIden {
        Alias::new(MIGRATION_TABLE).into_iden()
    }
}
