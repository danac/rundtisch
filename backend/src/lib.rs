#[cfg(not(any(feature = "native", feature = "cloudflare")))]
compile_error!("At least one feature must be enabled: either `native` or `cloudflare`.");

pub mod adapters;
pub mod handlers;
pub mod routes;
pub mod traits;
pub mod app;
pub mod queries;
pub mod models;
pub mod migrations;
