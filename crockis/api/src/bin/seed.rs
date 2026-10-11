use crockis::library::{data_dir, seed_mock_library};
use crockis::native_platform;
use crockis::{load_environment, migrate};

/// Download the mock photo catalog into `DATA_DIR` and exit.
///
/// Run this explicitly. `crockis-migrate` and the Edge post-deploy job do not
/// seed. Pending auth and library migrations are applied first so a fresh
/// database can be seeded in one command. This does not upsert a bootstrap
/// admin.
///
/// `DATABASE_URL` selects the database. `DATA_DIR` is the file root (default
/// `/data`, the Wasmer volume). On Edge, SSH in and run the package command
/// `seed` (this binary). A second run leaves files whose BLAKE3 digest matches
/// the row and downloads only the gaps.
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    load_environment();

    eprintln!("seed: connecting");
    let db = native_platform::connect().await?;
    eprintln!("seed: applying pending migrations");
    migrate(&db).await?;
    let dir = data_dir();
    eprintln!("seed: writing catalog into {}", dir.display());
    let report = seed_mock_library(&db, &dir).await?;
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
