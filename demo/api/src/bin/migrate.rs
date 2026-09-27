use rundtisch::auth::bootstrap::{self, BootstrapAdminOutcome};
use rundtisch_demo::Migrator;
use rundtisch_demo::native_platform;
use sea_orm_migration::MigratorTrait;

/// Apply pending SeaORM migrations, optionally seed a bootstrap admin, and exit.
///
/// Wasmer Edge runs this as the `migrate` package command from a
/// `pre-deployment` job. It is a one-shot CLI, not an HTTP handler: do not
/// call `Migrator::up` or the bootstrap insert from `native` or from a request.
///
/// When `RUNDTISCH_BOOTSTRAP_ADMIN_EMAIL` and
/// `RUNDTISCH_BOOTSTRAP_ADMIN_PASSWORD` are both set, insert a verified Admin
/// only if that email is not already in `auth_users`. An existing row is left
/// unchanged.
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("migrate: connecting");
    let db = native_platform::connect().await?;
    eprintln!("migrate: applying pending migrations");
    Migrator::up(&db, None).await?;
    match bootstrap::seed_bootstrap_admin_from_env(&db).await? {
        BootstrapAdminOutcome::SkippedUnset => {
            eprintln!("migrate: bootstrap admin unset, skipping");
        }
        BootstrapAdminOutcome::SkippedExists { email } => {
            eprintln!("migrate: bootstrap admin {email} already exists, skipping");
        }
        BootstrapAdminOutcome::Inserted { email, .. } => {
            eprintln!("migrate: bootstrap admin {email} inserted");
        }
    }
    db.close().await?;
    eprintln!("migrate: done");
    Ok(())
}
