use sea_orm_migration::prelude::*;

mod m20260929_120000_create_auth_tables;

/// Tracking table for auth migrations. A host database can keep its own
/// SeaORM history in `seaql_migrations`.
pub const MIGRATION_TABLE: &str = "rundtisch_migrations_auth";

/// Auth migrations, oldest first.
pub fn migrations() -> Vec<Box<dyn MigrationTrait>> {
    vec![Box::new(
        m20260929_120000_create_auth_tables::Migration,
    )]
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
