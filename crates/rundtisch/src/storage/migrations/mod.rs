use sea_orm_migration::prelude::*;

mod m20261013_120000_create_storage_objects;

/// Tracking table for object-store migrations. Separate from `rundtisch_migrations_auth`
/// and from the host app's history table.
pub const MIGRATION_TABLE: &str = "rundtisch_migrations_storage";

/// Storage migrations, oldest first.
pub fn migrations() -> Vec<Box<dyn MigrationTrait>> {
    vec![Box::new(m20261013_120000_create_storage_objects::Migration)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{ConnectOptions, ConnectionTrait, DatabaseBackend, Statement};

    async fn memory_db() -> sea_orm::DatabaseConnection {
        let mut opts = ConnectOptions::new("sqlite::memory:");
        opts.max_connections(1);
        sea_orm::Database::connect(opts).await.expect("sqlite")
    }

    async fn table_names(db: &sea_orm::DatabaseConnection) -> Vec<String> {
        let rows = db
            .query_all_raw(Statement::from_string(
                DatabaseBackend::Sqlite,
                "SELECT name FROM sqlite_master WHERE type = 'table'",
            ))
            .await
            .unwrap();
        rows.into_iter()
            .map(|row| row.try_get::<String>("", "name").unwrap())
            .collect()
    }

    #[tokio::test]
    async fn storage_migration_is_repeatable_and_reversible() {
        let db = memory_db().await;
        Migrator::up(&db, None).await.unwrap();
        Migrator::up(&db, None).await.unwrap();
        let names = table_names(&db).await;
        assert!(names.iter().any(|name| name == "storage_objects"));
        assert!(names.iter().any(|name| name == MIGRATION_TABLE));
        Migrator::down(&db, None).await.unwrap();
        let names = table_names(&db).await;
        assert!(names.iter().all(|name| name != "storage_objects"));
    }
}
