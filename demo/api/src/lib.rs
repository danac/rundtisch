mod handlers;
#[cfg(feature = "native")]
pub mod native_platform;
mod routes;

pub use routes::build_router;
