pub mod cloudflare;
pub use cloudflare::CloudFlareWorker;

#[cfg(not(target_arch = "wasm32"))]
pub mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::NativePlatform;
