use rundtisch_demo::native_platform;
use rundtisch_demo::Migrator;
use sea_orm_migration::MigratorTrait;

#[tokio::main]
async fn main() -> Result<(), sea_orm::DbErr> {
    let db = native_platform::connect().await?;
    Migrator::up(&db, None).await?;
    Ok(())
}
