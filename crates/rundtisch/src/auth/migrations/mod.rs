use sea_orm_migration::prelude::*;

mod m20260924_120000_create_auth_tables;

/// Auth migrations, oldest first. The application `Migrator` includes this list.
pub fn migrations() -> Vec<Box<dyn MigrationTrait>> {
    vec![Box::new(m20260924_120000_create_auth_tables::Migration)]
}
