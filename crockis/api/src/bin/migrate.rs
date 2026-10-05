use crockis::{load_environment, Migrator};
use crockis::native_platform;
use rundtisch::auth::bootstrap::{self, BootstrapAdminOutcome};
use sea_orm_migration::MigratorTrait;

/// Apply pending SeaORM migrations, optionally upsert a bootstrap admin, and exit.
///
/// Wasmer Edge runs this as the `migrate` package command from a
/// `post-deployment` job. It is a one-shot CLI, not an HTTP handler: do not
/// call `Migrator::up` or the bootstrap upsert from `native` or from a request.
///
/// When `RUNDTISCH_BOOTSTRAP_ADMIN_EMAIL` and
/// `RUNDTISCH_BOOTSTRAP_ADMIN_PASSWORD` are both set, insert a verified Admin
/// if that email is absent, or reset the password on the existing row. The
/// existing `public_id`, role, alias, and verification stay the same.
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    load_environment();
        
    eprintln!("migrate: connecting");
    let db = native_platform::connect().await?;
    eprintln!("migrate: applying pending migrations");
    Migrator::up(&db, None).await?;
    match bootstrap::seed_bootstrap_admin_from_env(&db).await? {
        BootstrapAdminOutcome::SkippedUnset => {
            eprintln!("migrate: bootstrap admin unset, skipping");
        }
        BootstrapAdminOutcome::Inserted { email, .. } => {
            eprintln!("migrate: bootstrap admin {email} inserted");
        }
        BootstrapAdminOutcome::Updated { email, .. } => {
            eprintln!("migrate: bootstrap admin {email} password updated");
        }
    }
    db.close().await?;
    eprintln!("migrate: done");
    Ok(())
}
