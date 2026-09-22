pub const ACCESS_TTL: time::Duration = time::Duration::minutes(15);
pub const SESSION_TTL: time::Duration = time::Duration::days(14);
pub const ACTIVATION_TTL: time::Duration = time::Duration::hours(24);
pub const JWT_LEEWAY: time::Duration = time::Duration::seconds(30);
pub const SESSION_COOKIE: &str = "session";

/// `SecretStore` / env names. Access and verify keys stay distinct; pepper is
/// neither JWT secret.
pub const AUTH_JWT_ACCESS_SECRET: &str = "AUTH_JWT_ACCESS_SECRET";
pub const AUTH_JWT_VERIFY_SECRET: &str = "AUTH_JWT_VERIFY_SECRET";
pub const AUTH_HASH_PEPPER: &str = "AUTH_HASH_PEPPER";
