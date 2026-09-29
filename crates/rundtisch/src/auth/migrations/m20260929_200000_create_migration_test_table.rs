use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::{big_pk_auto, timestamp};

/// Deployment probe used to verify that Wasmer runs the candidate package's
/// migrator during the same deployment.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(MigrationTest::Table)
                    .if_not_exists()
                    .col(big_pk_auto(MigrationTest::Id))
                    .col(timestamp(MigrationTest::CreatedAt))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(MigrationTest::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum MigrationTest {
    #[sea_orm(iden = "auth_migration_test")]
    Table,
    Id,
    CreatedAt,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn creates_the_deployment_probe_table() {
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        let manager = SchemaManager::new(&db);

        Migration.up(&manager).await.unwrap();

        assert!(manager.has_table("auth_migration_test").await.unwrap());
    }
}
