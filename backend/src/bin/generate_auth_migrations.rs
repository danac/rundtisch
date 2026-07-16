use std::io;
use std::path::Path;
use std::process;

use backend::auth::migrations::all_up_migrations;
use backend::traits::db::{Dialect, Migration, Statement};

fn migration_sql(statements: &[Statement], dialect: Dialect) -> String {
    statements
        .iter()
        .map(|stmt| stmt.to_sql(dialect))
        .collect::<Vec<_>>()
        .join(";\n\n")
}

fn write_dialect_migrations(
    target: &Path,
    dialect_name: &str,
    dialect: Dialect,
    migrations: &[Box<dyn Migration>],
) -> io::Result<()> {
    let dialect_dir = target.join(dialect_name);
    std::fs::create_dir_all(&dialect_dir)?;

    for migration in migrations {
        let sql = migration_sql(&migration.up(), dialect);
        let path = dialect_dir.join(format!("{}.sql", migration.name()));
        std::fs::write(path, format!("{sql};\n"))?;
    }

    Ok(())
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        process::exit(1);
    }
}

fn run() -> io::Result<()> {
    let target_dir = std::env::args().nth(1).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: generate_auth_migrations <target_dir>",
        )
    })?;

    let target = Path::new(&target_dir);
    let migrations = all_up_migrations();

    let dialects = [
        ("sqlite", Dialect::Sqlite),
        ("postgres", Dialect::Postgres),
        ("mysql", Dialect::Mysql),
    ];

    for (name, dialect) in dialects {
        write_dialect_migrations(target, name, dialect, &migrations)?;
    }

    Ok(())
}
