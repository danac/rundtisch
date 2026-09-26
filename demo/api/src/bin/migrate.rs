use rundtisch_demo::native_platform;
use rundtisch_demo::Migrator;
use sea_orm_migration::MigratorTrait;

/// Apply pending SeaORM migrations and exit.
///
/// Wasmer Edge runs this as the `migrate` package command from a
/// `pre-deployment` job. It is a one-shot CLI, not an HTTP handler: do not
/// call `Migrator::up` from `native` or from a request.
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), sea_orm::DbErr> {
    eprintln!("migrate: connecting");
    let db = native_platform::connect().await?;
    eprintln!("migrate: applying pending migrations");
    Migrator::up(&db, None).await?;
    db.close().await?;
    eprintln!("migrate: done");
    Ok(())
}
