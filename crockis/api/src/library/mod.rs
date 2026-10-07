//! Photo collections and the files behind them.
//!
//! Rows live in `collections` and `pictures`. Image bytes live as
//! `{DATA_DIR}/{storage_filename}`, which is `/data/{uuid}.{ext}` on Wasmer Edge.

pub(crate) mod catalog;
mod entities;
mod handlers;
mod queries;
pub(crate) mod seed;
pub(crate) mod store;

pub use handlers::{get_collection, list_collections, list_photos, photo_file};
pub use seed::seed_if_enabled;
pub use store::{DEFAULT_DATA_DIR, data_dir};
