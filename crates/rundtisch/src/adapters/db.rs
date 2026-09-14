#[cfg(feature = "cloudflare")]
pub mod d1;

#[cfg(feature = "native")]
pub mod sqlite;
