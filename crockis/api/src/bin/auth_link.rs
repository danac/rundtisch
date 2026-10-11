//! Mint a database-backed invitation or recovery link.
//!
//! Both subcommands need `DATABASE_URL` and `AUTH_HASH_PEPPER` (exactly 32
//! bytes). Pending auth and library migrations are applied first. Only the
//! hash of the opaque token is stored. This command does not seed photos.
//!
//! ```text
//! cargo run -p crockis --bin crockis-auth-link -- invite \
//!   --email user@example.com [--ttl-hours 24] [--base-url http://localhost:5174]
//! cargo run -p crockis --bin crockis-auth-link -- recover \
//!   --email user@example.com [--ttl-hours 1] [--base-url http://localhost:5174]
//! ```

use crockis::migrate;
use crockis::native_platform;
use rundtisch::auth::config::AUTH_HASH_PEPPER;
use rundtisch::auth::handlers::{mint_invitation_link, mint_recovery_link};
use time::Duration;

const DEFAULT_BASE_URL: &str = "http://localhost:5174";

#[derive(Debug)]
enum Command {
    Invite,
    Recover,
}

#[derive(Debug)]
struct Args {
    command: Command,
    email: String,
    ttl_hours: i64,
    base_url: String,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("auth-link: {err}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args(std::env::args().skip(1))?;
    let pepper =
        std::env::var(AUTH_HASH_PEPPER).map_err(|_| format!("{AUTH_HASH_PEPPER} is required"))?;
    if pepper.len() != 32 {
        return Err(format!("{AUTH_HASH_PEPPER} must be exactly 32 bytes").into());
    }
    let db = native_platform::connect().await?;
    migrate(&db).await?;
    let ttl = Duration::hours(args.ttl_hours);
    let (kind, token) = match args.command {
        Command::Invite => (
            "invite",
            mint_invitation_link(&db, pepper.as_bytes(), &args.email, ttl).await?,
        ),
        Command::Recover => {
            let token = mint_recovery_link(&db, pepper.as_bytes(), &args.email, ttl).await?;
            (
                "recover",
                token.ok_or_else(|| "no user with that email".to_string())?,
            )
        }
    };
    db.close().await?;
    println!("{token}");
    println!("{}/?{kind}={token}", args.base_url);
    Ok(())
}

fn parse_args<I>(args: I) -> Result<Args, String>
where
    I: IntoIterator<Item = String>,
{
    let mut args = args.into_iter();
    let command = match args.next().as_deref() {
        Some("invite") => Command::Invite,
        Some("recover") => Command::Recover,
        Some(other) => return Err(format!("unknown command {other}")),
        None => return Err(usage()),
    };
    let mut email = None;
    let mut ttl_hours = None;
    let mut base_url = DEFAULT_BASE_URL.to_string();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--email" => {
                email = Some(
                    args.next()
                        .ok_or_else(|| "--email needs a value".to_string())?,
                );
            }
            "--ttl-hours" => {
                let raw = args
                    .next()
                    .ok_or_else(|| "--ttl-hours needs a value".to_string())?;
                let hours: i64 = raw
                    .parse()
                    .map_err(|_| "--ttl-hours must be a positive integer".to_string())?;
                if hours <= 0 {
                    return Err("--ttl-hours must be a positive integer".to_string());
                }
                ttl_hours = Some(hours);
            }
            "--base-url" => {
                base_url = args
                    .next()
                    .ok_or_else(|| "--base-url needs a value".to_string())?;
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    let email = email.ok_or_else(|| "--email is required".to_string())?;
    let ttl_hours = ttl_hours.unwrap_or(match command {
        Command::Invite => 24,
        Command::Recover => 1,
    });
    Ok(Args {
        command,
        email,
        ttl_hours,
        base_url: base_url.trim_end_matches('/').to_string(),
    })
}

fn usage() -> String {
    "usage: auth-link <invite|recover> --email <email> [--ttl-hours N] [--base-url URL]".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(args: &[&str]) -> Vec<String> {
        args.iter().map(|arg| (*arg).to_string()).collect()
    }

    #[test]
    fn invite_defaults() {
        let args = parse_args(strings(&["invite", "--email", "user@example.com"])).unwrap();
        assert!(matches!(args.command, Command::Invite));
        assert_eq!(args.email, "user@example.com");
        assert_eq!(args.ttl_hours, 24);
        assert_eq!(args.base_url, DEFAULT_BASE_URL);
    }

    #[test]
    fn recover_defaults_and_strips_slash() {
        let args = parse_args(strings(&[
            "recover",
            "--email",
            "user@example.com",
            "--ttl-hours",
            "2",
            "--base-url",
            "http://localhost:5174/",
        ]))
        .unwrap();
        assert!(matches!(args.command, Command::Recover));
        assert_eq!(args.ttl_hours, 2);
        assert_eq!(args.base_url, "http://localhost:5174");
    }

    #[test]
    fn rejects_missing_email() {
        let err = parse_args(strings(&["invite"])).unwrap_err();
        assert!(err.contains("--email"));
    }
}
