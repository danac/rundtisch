//! Platform-agnostic layer for Axum apps that run on Cloudflare Workers and as native binaries.

pub mod auth;
pub mod db;
pub mod platform;
pub mod runtime;

pub use platform::{AppState, Platform};
