use crockis::library::seed_mock_library;
use crockis::native_platform;
use crockis::{load_environment, migrate};
use rundtisch::storage::{ObjectStore, local_root_from_env, storage_from_env};

/// Download the mock photo catalog into the object store and exit.
///
/// Run this explicitly. `crockis-migrate` and the Edge post-deploy job do not
/// seed. Pending auth, storage, and library migrations are applied first so a
/// fresh database can be seeded in one command. This does not upsert a
/// bootstrap admin.
///
/// `DATABASE_URL` selects the database. `STORAGE_BACKEND=local` (the default)
/// writes under `STORAGE_LOCAL_ROOT`, then `DATA_DIR`, then `/data`. On Edge,
/// SSH in and run the package command `seed` (this binary). A second run
/// leaves files whose BLAKE3 digest matches the row and restores only gaps
/// whose bytes still match that digest.
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    load_environment();

    eprintln!("seed: connecting");
    let db = native_platform::connect().await?;
    eprintln!("seed: applying pending migrations");
    migrate(&db).await?;
    let storage = storage_from_env()?;
    let root = local_root_from_env();
    eprintln!("seed: writing catalog into {}", root.display());
    let store = ObjectStore::new(db.clone(), storage);
    let report = seed_mock_library(&store).await?;
    eprintln!(
        "seed: collections_inserted={} pictures_inserted={} files_written={} pictures_already_stored={}",
        report.collections_inserted,
        report.pictures_inserted,
        report.files_written,
        report.pictures_present
    );
    db.close().await?;
    eprintln!("seed: done");
    Ok(())
}
