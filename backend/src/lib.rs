#[cfg(not(any(feature = "native", feature = "cloudflare")))]
compile_error!("At least one feature must be enabled: either `native` or `cloudflare`.");

pub mod app_state;
pub mod handlers;
pub mod adapters;
pub mod routes;
pub mod traits;

pub use app_state::AppState;
