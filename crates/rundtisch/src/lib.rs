//! Platform-agnostic layer for Axum apps that run on Cloudflare Workers and as native binaries.

pub mod adapters;
pub mod app;
pub mod auth;
pub mod traits;

pub use app::AppState;
pub use traits::Platform;
