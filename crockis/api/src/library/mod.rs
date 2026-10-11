//! Photo collections and the files behind them.
//!
//! Rows live in `collections` and `pictures`. Image bytes live as
//! `{DATA_DIR}/{storage_folder}/{storage_filename}`, which is
//! `/data/{collection-folder-uuid}/{uuid}.{ext}` on Wasmer Edge.

#[cfg(any(feature = "seed", test))]
pub(crate) mod catalog;
mod entities;
mod handlers;
#[cfg(any(feature = "seed", test))]
pub(crate) mod inspect;
mod queries;
#[cfg(any(feature = "seed", test))]
pub(crate) mod seed;
pub(crate) mod store;

pub use handlers::{get_collection, list_collections, list_photos, photo_file};
#[cfg(feature = "seed")]
pub use seed::{SeedError, seed_mock_library};
pub use store::{DEFAULT_DATA_DIR, data_dir};
