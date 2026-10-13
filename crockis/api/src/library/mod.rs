//! Photo collections and the files behind them.
//!
//! Rows live in `collections` and `pictures`. Image bytes live in the
//! object store. Each collection's `storage_folder` is the key prefix, so
//! files land at `{root}/{storage_folder}/{uuid}.{ext}`.

#[cfg(any(feature = "seed", test))]
pub(crate) mod catalog;
mod entities;
mod handlers;
#[cfg(any(feature = "seed", test))]
pub(crate) mod inspect;
mod queries;
#[cfg(any(feature = "seed", test))]
pub(crate) mod seed;

pub use handlers::{get_collection, list_collections, list_photos, photo_file};
#[cfg(feature = "seed")]
pub use seed::{SeedError, seed_mock_library};
