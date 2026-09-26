use sea_orm::DatabaseConnection;
use sea_orm::DbErr;

/// Open the database named by `DATABASE_URL`, or by Wasmer Edge `DB_*` vars.
///
/// `sqlite://`, `mysql://`, and `postgres://` `DATABASE_URL` values are all
/// valid. On Edge, managed MySQL injects `DB_HOST`, `DB_PORT`, `DB_NAME`,
/// `DB_USERNAME`, and `DB_PASSWORD` instead of a single URL.
pub async fn connect() -> Result<DatabaseConnection, DbErr> {
    let url = database_url().map_err(DbErr::Custom)?;
    sea_orm::Database::connect(&url).await
}

pub fn database_url() -> Result<String, String> {
    if let Ok(url) = std::env::var("DATABASE_URL") {
        return Ok(url);
    }
    mysql_url_from_wasmer_env()
}

fn mysql_url_from_wasmer_env() -> Result<String, String> {
    let host = std::env::var("DB_HOST").map_err(|_| {
        "set DATABASE_URL or the Wasmer database variables DB_HOST, DB_PORT, DB_NAME, DB_USERNAME, and DB_PASSWORD"
            .to_string()
    })?;
    let port = std::env::var("DB_PORT").unwrap_or_else(|_| "3306".to_string());
    let name = std::env::var("DB_NAME")
        .map_err(|_| "DB_NAME is required when DATABASE_URL is unset".to_string())?;
    let username = std::env::var("DB_USERNAME")
        .map_err(|_| "DB_USERNAME is required when DATABASE_URL is unset".to_string())?;
    let password = std::env::var("DB_PASSWORD").unwrap_or_default();
    let ssl_mode = ssl_mode_for_host(&host, std::env::var("DB_SSL_MODE").ok().as_deref());
    Ok(mysql_url(
        &username, &password, &host, &port, &name, &ssl_mode,
    ))
}

pub fn mysql_url(
    username: &str,
    password: &str,
    host: &str,
    port: &str,
    name: &str,
    ssl_mode: &str,
) -> String {
    format!(
        "mysql://{}:{}@{}:{}/{}?ssl-mode={}",
        encode_userinfo(username),
        encode_userinfo(password),
        host,
        port,
        name,
        ssl_mode
    )
}

/// Wasmer managed MySQL uses a private CA, so the client encrypts without
/// verifying the chain (`REQUIRED`). Local MySQL usually has no TLS.
pub fn ssl_mode_for_host(host: &str, override_mode: Option<&str>) -> String {
    if let Some(value) = override_mode {
        if let Some(mode) = parse_ssl_mode(value) {
            return mode.to_string();
        }
    }
    default_ssl_mode(host).to_string()
}

fn parse_ssl_mode(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "disabled" | "disable" | "off" | "false" => Some("DISABLED"),
        "preferred" | "prefer" => Some("PREFERRED"),
        "required" | "require" | "on" | "true" => Some("REQUIRED"),
        "verify_ca" | "verify-ca" => Some("VERIFY_CA"),
        "verify_identity" | "verify-identity" => Some("VERIFY_IDENTITY"),
        _ => None,
    }
}

fn default_ssl_mode(host: &str) -> &'static str {
    let host = host.to_ascii_lowercase();
    if host.starts_with("db.") || host.contains("wasmer") {
        "REQUIRED"
    } else {
        "PREFERRED"
    }
}

fn encode_userinfo(value: &str) -> String {
    const SAFE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
    let mut out = String::new();
    for byte in value.bytes() {
        if SAFE.contains(&byte) {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasmer_hosts_require_tls_without_cert_verification() {
        assert_eq!(
            ssl_mode_for_host("db.us-losa1.wasmer.app", None),
            "REQUIRED"
        );
    }

    #[test]
    fn local_hosts_prefer_tls() {
        assert_eq!(ssl_mode_for_host("127.0.0.1", None), "PREFERRED");
    }

    #[test]
    fn explicit_ssl_mode_wins() {
        assert_eq!(
            ssl_mode_for_host("db.example.wasmer.app", Some("disabled")),
            "DISABLED"
        );
        assert_eq!(ssl_mode_for_host("localhost", Some("required")), "REQUIRED");
    }

    #[test]
    fn builds_mysql_url_and_encodes_password() {
        let url = mysql_url(
            "demo",
            "p@ss:word",
            "127.0.0.1",
            "3307",
            "rundtisch_dev",
            "REQUIRED",
        );
        assert_eq!(
            url,
            "mysql://demo:p%40ss%3Aword@127.0.0.1:3307/rundtisch_dev?ssl-mode=REQUIRED"
        );
    }
}
