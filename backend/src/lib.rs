#[cfg(not(any(feature = "native", feature = "cloudflare", feature = "tools")))]
compile_error!("At least one feature must be enabled: either `native`, `cloudflare`, or `tools`.");

pub mod adapters;
pub mod handlers;
pub mod routes;
pub mod traits;
pub mod app;
pub mod auth;
