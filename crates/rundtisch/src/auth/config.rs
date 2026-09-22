pub const ACCESS_TTL: time::Duration = time::Duration::minutes(15);
pub const SESSION_TTL: time::Duration = time::Duration::days(14);
pub const ACTIVATION_TTL: time::Duration = time::Duration::hours(24);
pub const JWT_LEEWAY: time::Duration = time::Duration::seconds(30);
pub const SESSION_COOKIE: &str = "session";
