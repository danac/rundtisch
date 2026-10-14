mod m20261007_120000_create_library_tables;

use sea_orm::DatabaseConnection;
use sea_orm::DbErr;
use sea_orm_migration::prelude::*;

/// Library migration history. Auth keeps `rundtisch_migrations_auth` and the
/// object store keeps `rundtisch_migrations_storage`.
pub const MIGRATION_TABLE: &str = "crockis_migrations";

pub use rundtisch::auth::Migrator as AuthMigrator;
pub use rundtisch::storage::migrations::Migrator as StorageMigrator;

/// Crockis photo-library migrations.
pub struct LibraryMigrator;

impl MigratorTrait for LibraryMigrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20261007_120000_create_library_tables::Migration)]
    }

    fn migration_table_name() -> sea_orm::DynIden {
        Alias::new(MIGRATION_TABLE).into_iden()
    }
}

/// Apply auth migrations, then object-store migrations, then library migrations.
///
/// Each migrator records history in its own table. `pictures.object_id`
/// references `storage_objects`, so the storage migrator runs first.
pub async fn migrate(db: &DatabaseConnection) -> Result<(), DbErr> {
    AuthMigrator::up(db, None).await?;
    StorageMigrator::up(db, None).await?;
    LibraryMigrator::up(db, None).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{ConnectOptions, ConnectionTrait, DatabaseBackend, Statement};

    async fn memory_db() -> DatabaseConnection {
        let mut opts = ConnectOptions::new("sqlite::memory:");
        opts.max_connections(1);
        sea_orm::Database::connect(opts).await.expect("sqlite")
    }

    async fn table_names(db: &DatabaseConnection) -> Vec<String> {
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
    async fn library_migration_is_repeatable_and_reversible() {
        let db = memory_db().await;
        // MySQL refuses the pictures FK until `storage_objects` exists.
        StorageMigrator::up(&db, None).await.unwrap();
        LibraryMigrator::up(&db, None).await.unwrap();
        LibraryMigrator::up(&db, None).await.unwrap();
        let names = table_names(&db).await;
        assert!(names.iter().any(|name| name == "collections"));
        assert!(names.iter().any(|name| name == "pictures"));
        assert!(names.iter().any(|name| name == MIGRATION_TABLE));

        LibraryMigrator::down(&db, None).await.unwrap();
        let names = table_names(&db).await;
        assert!(
            names
                .iter()
                .all(|name| name != "collections" && name != "pictures")
        );
    }

    #[tokio::test]
    async fn migrate_applies_auth_and_library_tables() {
        let db = memory_db().await;
        migrate(&db).await.unwrap();
        let names = table_names(&db).await;
        assert!(names.iter().any(|name| name == "auth_users"));
        assert!(names.iter().any(|name| name == "rundtisch_migrations_auth"));
        assert!(names.iter().any(|name| name == "storage_objects"));
        assert!(
            names
                .iter()
                .any(|name| name == "rundtisch_migrations_storage")
        );
        assert!(names.iter().any(|name| name == "collections"));
        assert!(names.iter().any(|name| name == "pictures"));
        assert!(names.iter().any(|name| name == MIGRATION_TABLE));
    }
}
