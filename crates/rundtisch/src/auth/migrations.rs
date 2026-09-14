use crate::traits::db::Migration;

pub mod auth_migration_001;

pub fn all_up_migrations() -> Vec<Box<dyn Migration>> {
    vec![
        Box::new(auth_migration_001::AuthMigration001)
    ]
}