mod handlers;
mod routes;
#[cfg(feature = "native")]
pub mod native_platform;

pub use routes::build_router;
