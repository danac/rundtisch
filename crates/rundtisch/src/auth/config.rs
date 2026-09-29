pub const SESSION_TTL: time::Duration = time::Duration::days(14);
pub const INVITATION_TTL: time::Duration = time::Duration::hours(24);
pub const RECOVERY_TTL: time::Duration = time::Duration::hours(1);
pub const CEREMONY_TTL: time::Duration = time::Duration::minutes(5);
pub const LAST_USED_MIN_INTERVAL: time::Duration = time::Duration::minutes(5);
pub const RECOVERY_MIN_INTERVAL: time::Duration = time::Duration::minutes(1);
pub const SESSION_COOKIE: &str = "session";

/// HMAC key for opaque session, invitation, and recovery token hashes.
/// Exactly 32 bytes.
pub const AUTH_HASH_PEPPER: &str = "AUTH_HASH_PEPPER";

pub const AUTH_WEBAUTHN_RP_ID: &str = "AUTH_WEBAUTHN_RP_ID";
pub const AUTH_WEBAUTHN_RP_ORIGIN: &str = "AUTH_WEBAUTHN_RP_ORIGIN";
pub const AUTH_WEBAUTHN_RP_NAME: &str = "AUTH_WEBAUTHN_RP_NAME";

pub const DEFAULT_WEBAUTHN_RP_ID: &str = "localhost";
pub const DEFAULT_WEBAUTHN_RP_ORIGIN: &str = "http://localhost:5173";
pub const DEFAULT_WEBAUTHN_RP_NAME: &str = "rundtisch";
