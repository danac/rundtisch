use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;

use rundtisch::auth::migrations::all_up_migrations;
use rundtisch::traits::db::{Dialect, Migration, Statement, schema_to_sql};

fn migration_sql(statements: &[Statement], dialect: Dialect) -> String {
    statements
        .iter()
        .map(|stmt| schema_to_sql(stmt, dialect))
        .collect::<Vec<_>>()
        .join(";\n\n")
}

fn absolutize(path: PathBuf) -> io::Result<PathBuf> {
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

fn confirm_target(default: &Path) -> io::Result<Option<PathBuf>> {
    let default_abs = absolutize(default.to_path_buf())?;
    print!(
        "Write migrations to {} ? [Y/n/path] ",
        default_abs.display()
    );
    io::stdout().flush()?;

    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    let answer = line.trim();

    if answer.is_empty() || answer.eq_ignore_ascii_case("y") || answer.eq_ignore_ascii_case("yes")
    {
        return Ok(Some(default_abs));
    }
    if answer.eq_ignore_ascii_case("n") || answer.eq_ignore_ascii_case("no") {
        return Ok(None);
    }

    let override_abs = absolutize(PathBuf::from(answer))?;
    println!("Using {}", override_abs.display());
    Ok(Some(override_abs))
}

fn write_sqlite_migrations(
    target: &Path,
    migrations: &[Box<dyn Migration>],
) -> io::Result<()> {
    std::fs::create_dir_all(target)?;

    for migration in migrations {
        let sql = migration_sql(&migration.up(), Dialect::Sqlite);
        let path = target.join(format!("{}.sql", migration.name()));
        std::fs::write(&path, format!("{sql};\n"))?;
        println!("wrote {}", path.display());
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
    let default = std::env::current_dir()?.join("migrations");
    let Some(target) = confirm_target(&default)? else {
        println!("aborted");
        return Ok(());
    };

    let migrations = all_up_migrations();
    write_sqlite_migrations(&target, &migrations)?;
    Ok(())
}
