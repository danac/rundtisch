//! Seed catalog transcribed from the Crockis frontend mock library
//! (Serenity Spa, Alpine Light, Summer Garden, North Coast).
//!
//! Public ids are stable UUIDs so a second seed updates the same rows.
//! The high 32 bits mark the collection row (`018f5c10`) or a picture
//! group (`018f5c11` Serenity Spa, `018f5c12` Alpine Light, `018f5c13`
//! Summer Garden, `018f5c14` North Coast). The low 48 bits are the
//! 1-based index. `storage_folder` uses prefix `018f5c20` and the same index
//! as the collection. `storage_filename` is assigned separately at insert.

use time::OffsetDateTime;
use time::macros::datetime;
use uuid::Uuid;

/// Capture timestamp shared by the seeded mock library.
pub const MOCK_CAPTURED_AT: OffsetDateTime = datetime!(2026-03-12 10:00 UTC);

const COLLECTION_PREFIX: u32 = 0x018f_5c10;
const STORAGE_FOLDER_PREFIX: u32 = 0x018f_5c20;
const SERENITY_PHOTOS: u32 = 0x018f_5c11;
const ALPINE_PHOTOS: u32 = 0x018f_5c12;
const GARDEN_PHOTOS: u32 = 0x018f_5c13;
const COAST_PHOTOS: u32 = 0x018f_5c14;
const CATALOG_VERSION: u128 = 0x0000_7000_8000 << 48;

const fn catalog_uuid(prefix: u32, index: u128) -> Uuid {
    Uuid::from_u128(((prefix as u128) << 96) | CATALOG_VERSION | index)
}

pub const SERENITY_SPA_ID: Uuid = catalog_uuid(COLLECTION_PREFIX, 1);
pub const ALPINE_LIGHT_ID: Uuid = catalog_uuid(COLLECTION_PREFIX, 2);
pub const SUMMER_GARDEN_ID: Uuid = catalog_uuid(COLLECTION_PREFIX, 3);
pub const NORTH_COAST_ID: Uuid = catalog_uuid(COLLECTION_PREFIX, 4);
pub const SERENITY_SPA_FIRST_ID: Uuid = catalog_uuid(SERENITY_PHOTOS, 1);
pub const SERENITY_SPA_COVER_ID: Uuid = catalog_uuid(SERENITY_PHOTOS, 7);
pub const SERENITY_SPA_STORAGE_FOLDER: Uuid = catalog_uuid(STORAGE_FOLDER_PREFIX, 1);
pub const ALPINE_LIGHT_STORAGE_FOLDER: Uuid = catalog_uuid(STORAGE_FOLDER_PREFIX, 2);
pub const SUMMER_GARDEN_STORAGE_FOLDER: Uuid = catalog_uuid(STORAGE_FOLDER_PREFIX, 3);
pub const NORTH_COAST_STORAGE_FOLDER: Uuid = catalog_uuid(STORAGE_FOLDER_PREFIX, 4);

pub struct CatalogPhoto {
    pub public_id: Uuid,
    pub source_url: &'static str,
    /// Preferred download name. The stored extension follows the downloaded bytes.
    pub original_filename: &'static str,
}

pub struct CatalogCollection {
    pub public_id: Uuid,
    /// Stable directory under the data volume. Separate from `public_id`.
    pub storage_folder: Uuid,
    pub name: &'static str,
    pub description: &'static str,
    pub created_at: OffsetDateTime,
    pub cover_public_id: Uuid,
    pub photos: &'static [CatalogPhoto],
}

const SERENITY_SPA: &[CatalogPhoto] = &[
    CatalogPhoto {
        public_id: SERENITY_SPA_FIRST_ID,
        source_url: "https://images.unsplash.com/photo-1600334129128-685c5582fd35?auto=format&fit=crop&w=900&h=900&q=80",
        original_filename: "Stones.jpg",
    },
    CatalogPhoto {
        public_id: catalog_uuid(SERENITY_PHOTOS, 2),
        source_url: "https://images.unsplash.com/photo-1560750588-73207b1ef5b8?auto=format&fit=crop&w=1600&h=900&q=80",
        original_filename: "Pavilion.jpg",
    },
    CatalogPhoto {
        public_id: catalog_uuid(SERENITY_PHOTOS, 3),
        source_url: "https://images.unsplash.com/photo-1544161515-4ab6ce6db874?auto=format&fit=crop&w=1600&h=900&q=80",
        original_filename: "Touch.jpg",
    },
    CatalogPhoto {
        public_id: catalog_uuid(SERENITY_PHOTOS, 4),
        source_url: "https://images.pexels.com/photos/3188/love-romantic-bath-candlelight.jpg?auto=compress&cs=tinysrgb&w=900",
        original_filename: "love-romantic-bath-candlelight.jpg",
    },
    CatalogPhoto {
        public_id: catalog_uuid(SERENITY_PHOTOS, 5),
        source_url: "https://images.unsplash.com/photo-1515377905703-c4788e51af15?auto=format&fit=crop&w=1400&h=900&q=80",
        original_filename: "Oil.jpg",
    },
    CatalogPhoto {
        public_id: catalog_uuid(SERENITY_PHOTOS, 6),
        source_url: "https://images.unsplash.com/photo-1418065460487-3e41a6c84dc5?auto=format&fit=crop&w=1600&h=1000&q=80",
        original_filename: "Grove.jpg",
    },
    CatalogPhoto {
        public_id: catalog_uuid(SERENITY_PHOTOS, 7),
        source_url: "https://images.unsplash.com/photo-1583416750470-965b2707b355?auto=format&fit=crop&w=1000&h=900&q=80",
        original_filename: "Heat.jpg",
    },
    CatalogPhoto {
        public_id: catalog_uuid(SERENITY_PHOTOS, 8),
        source_url: "https://images.unsplash.com/photo-1519824145371-296894a0daa9?auto=format&fit=crop&w=900&h=1100&q=80",
        original_filename: "Hands.jpg",
    },
    CatalogPhoto {
        public_id: catalog_uuid(SERENITY_PHOTOS, 9),
        source_url: "https://images.unsplash.com/photo-1507652313519-d4e9174996dd?auto=format&fit=crop&w=1600&h=900&q=80",
        original_filename: "Bath.jpg",
    },
    CatalogPhoto {
        public_id: catalog_uuid(SERENITY_PHOTOS, 10),
        source_url: "https://images.pexels.com/photos/3764568/pexels-photo-3764568.jpeg?auto=compress&cs=tinysrgb&w=1000",
        original_filename: "pexels-photo-3764568.jpeg",
    },
    CatalogPhoto {
        public_id: catalog_uuid(SERENITY_PHOTOS, 11),
        source_url: "https://images.unsplash.com/photo-1441974231531-c6227db76b6e?auto=format&fit=crop&w=1100&h=900&q=80",
        original_filename: "Walk.jpg",
    },
    CatalogPhoto {
        public_id: catalog_uuid(SERENITY_PHOTOS, 12),
        source_url: "https://images.pexels.com/photos/3865530/pexels-photo-3865530.jpeg?auto=compress&cs=tinysrgb&w=900",
        original_filename: "pexels-photo-3865530.jpeg",
    },
    CatalogPhoto {
        public_id: catalog_uuid(SERENITY_PHOTOS, 13),
        source_url: "https://images.unsplash.com/photo-1570172619644-dfd03ed5d881?auto=format&fit=crop&w=1600&h=900&q=80",
        original_filename: "Mask.jpg",
    },
    CatalogPhoto {
        public_id: catalog_uuid(SERENITY_PHOTOS, 14),
        source_url: "https://images.pexels.com/photos/3757942/pexels-photo-3757942.jpeg?auto=compress&cs=tinysrgb&w=1000",
        original_filename: "pexels-photo-3757942.jpeg",
    },
    CatalogPhoto {
        public_id: catalog_uuid(SERENITY_PHOTOS, 15),
        source_url: "https://images.pexels.com/photos/3865676/pexels-photo-3865676.jpeg?auto=compress&cs=tinysrgb&w=1400",
        original_filename: "pexels-photo-3865676.jpeg",
    },
    CatalogPhoto {
        public_id: catalog_uuid(SERENITY_PHOTOS, 16),
        source_url: "https://images.unsplash.com/photo-1519823551278-64ac92734fb1?auto=format&fit=crop&w=900&h=1100&q=80",
        original_filename: "Close.jpg",
    },
];

const ALPINE_LIGHT: &[CatalogPhoto] = &[
    CatalogPhoto {
        public_id: catalog_uuid(ALPINE_PHOTOS, 1),
        source_url: "https://images.unsplash.com/photo-1464822759023-fed622ff2c3b?auto=format&fit=crop&w=1600&h=1000&q=80",
        original_filename: "Ridge.jpg",
    },
    CatalogPhoto {
        public_id: catalog_uuid(ALPINE_PHOTOS, 2),
        source_url: "https://images.unsplash.com/photo-1483728642387-6c3bdd6c93e5?auto=format&fit=crop&w=1200&h=1600&q=80",
        original_filename: "Summit.jpg",
    },
    CatalogPhoto {
        public_id: catalog_uuid(ALPINE_PHOTOS, 3),
        source_url: "https://images.unsplash.com/photo-1469474968028-56623f02e42e?auto=format&fit=crop&w=1600&h=900&q=80",
        original_filename: "Valley.jpg",
    },
    CatalogPhoto {
        public_id: catalog_uuid(ALPINE_PHOTOS, 4),
        source_url: "https://images.unsplash.com/photo-1470071459604-3b5ec3a7fe05?auto=format&fit=crop&w=1400&h=900&q=80",
        original_filename: "Fog.jpg",
    },
    CatalogPhoto {
        public_id: catalog_uuid(ALPINE_PHOTOS, 5),
        source_url: "https://images.unsplash.com/photo-1501785888041-af3ef285b470?auto=format&fit=crop&w=900&h=1200&q=80",
        original_filename: "Lake.jpg",
    },
    CatalogPhoto {
        public_id: catalog_uuid(ALPINE_PHOTOS, 6),
        source_url: "https://images.unsplash.com/photo-1519681393784-d120267933ba?auto=format&fit=crop&w=1600&h=900&q=80",
        original_filename: "Night.jpg",
    },
];

const SUMMER_GARDEN: &[CatalogPhoto] = &[
    CatalogPhoto {
        public_id: catalog_uuid(GARDEN_PHOTOS, 1),
        source_url: "https://images.unsplash.com/photo-1466781783364-36c955e42a7f?auto=format&fit=crop&w=1400&h=900&q=80",
        original_filename: "Path.jpg",
    },
    CatalogPhoto {
        public_id: catalog_uuid(GARDEN_PHOTOS, 2),
        source_url: "https://images.unsplash.com/photo-1490750967868-88aa4486c946?auto=format&fit=crop&w=900&h=1200&q=80",
        original_filename: "Bloom.jpg",
    },
    CatalogPhoto {
        public_id: catalog_uuid(GARDEN_PHOTOS, 3),
        source_url: "https://images.unsplash.com/photo-1501004318641-b39e6451bec6?auto=format&fit=crop&w=1200&h=900&q=80",
        original_filename: "Glass.jpg",
    },
    CatalogPhoto {
        public_id: catalog_uuid(GARDEN_PHOTOS, 4),
        source_url: "https://images.unsplash.com/photo-1441974231531-c6227db76b6e?auto=format&fit=crop&w=1000&h=900&q=80",
        original_filename: "Wood.jpg",
    },
    CatalogPhoto {
        public_id: catalog_uuid(GARDEN_PHOTOS, 5),
        source_url: "https://images.unsplash.com/photo-1418065460487-3e41a6c84dc5?auto=format&fit=crop&w=1600&h=900&q=80",
        original_filename: "Mist.jpg",
    },
];

const NORTH_COAST: &[CatalogPhoto] = &[
    CatalogPhoto {
        public_id: catalog_uuid(COAST_PHOTOS, 1),
        source_url: "https://images.unsplash.com/photo-1507525428034-b723cf961d3e?auto=format&fit=crop&w=1600&h=900&q=80",
        original_filename: "Shore.jpg",
    },
    CatalogPhoto {
        public_id: catalog_uuid(COAST_PHOTOS, 2),
        source_url: "https://images.unsplash.com/photo-1500375592092-40eb2168fd21?auto=format&fit=crop&w=1200&h=1600&q=80",
        original_filename: "Cliff.jpg",
    },
    CatalogPhoto {
        public_id: catalog_uuid(COAST_PHOTOS, 3),
        source_url: "https://images.unsplash.com/photo-1505118380757-91f5f5632de0?auto=format&fit=crop&w=1600&h=1000&q=80",
        original_filename: "Tide.jpg",
    },
    CatalogPhoto {
        public_id: catalog_uuid(COAST_PHOTOS, 4),
        source_url: "https://images.unsplash.com/photo-1439066615861-d1af74d74000?auto=format&fit=crop&w=1400&h=900&q=80",
        original_filename: "Still.jpg",
    },
    CatalogPhoto {
        public_id: catalog_uuid(COAST_PHOTOS, 5),
        source_url: "https://images.unsplash.com/photo-1475924156734-496f6cac6ec1?auto=format&fit=crop&w=900&h=1100&q=80",
        original_filename: "Surf.jpg",
    },
    CatalogPhoto {
        public_id: catalog_uuid(COAST_PHOTOS, 6),
        source_url: "https://images.unsplash.com/photo-1507525428034-b723cf961d3e?auto=format&fit=crop&w=1600&h=900&q=80",
        original_filename: "Bay.jpg",
    },
    CatalogPhoto {
        public_id: catalog_uuid(COAST_PHOTOS, 7),
        source_url: "https://images.unsplash.com/photo-1473116763249-2faaef81ccda?auto=format&fit=crop&w=1000&h=900&q=80",
        original_filename: "Horizon.jpg",
    },
];

pub static MOCK_CATALOG: &[CatalogCollection] = &[
    CatalogCollection {
        public_id: SERENITY_SPA_ID,
        storage_folder: SERENITY_SPA_STORAGE_FOLDER,
        name: "Serenity Spa",
        description: "A quiet weekend of water, heat, and low light.",
        created_at: datetime!(2026-01-01 0:00 UTC),
        cover_public_id: SERENITY_SPA_COVER_ID,
        photos: SERENITY_SPA,
    },
    CatalogCollection {
        public_id: ALPINE_LIGHT_ID,
        storage_folder: ALPINE_LIGHT_STORAGE_FOLDER,
        name: "Alpine Light",
        description: "High passes and still lakes after the thaw.",
        created_at: datetime!(2026-01-02 0:00 UTC),
        cover_public_id: catalog_uuid(ALPINE_PHOTOS, 1),
        photos: ALPINE_LIGHT,
    },
    CatalogCollection {
        public_id: SUMMER_GARDEN_ID,
        storage_folder: SUMMER_GARDEN_STORAGE_FOLDER,
        name: "Summer Garden",
        description: "Rain, glasshouses, and late flowers.",
        created_at: datetime!(2026-01-03 0:00 UTC),
        cover_public_id: catalog_uuid(GARDEN_PHOTOS, 1),
        photos: SUMMER_GARDEN,
    },
    CatalogCollection {
        public_id: NORTH_COAST_ID,
        storage_folder: NORTH_COAST_STORAGE_FOLDER,
        name: "North Coast",
        description: "Tide lines and long evenings by the water.",
        created_at: datetime!(2026-01-04 0:00 UTC),
        cover_public_id: catalog_uuid(COAST_PHOTOS, 1),
        photos: NORTH_COAST,
    },
];

pub fn mock_catalog() -> &'static [CatalogCollection] {
    MOCK_CATALOG
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn catalog_preserves_the_mock_library() {
        let catalog = mock_catalog();
        assert_eq!(catalog.len(), 4);
        let mut ids = HashSet::new();
        let mut photos = 0;
        for collection in catalog {
            assert!(ids.insert(collection.public_id));
            assert!(ids.insert(collection.storage_folder));
            assert_ne!(collection.storage_folder, collection.public_id);
            assert!(
                collection
                    .photos
                    .iter()
                    .any(|photo| photo.public_id == collection.cover_public_id),
                "cover {} missing",
                collection.cover_public_id
            );
            for photo in collection.photos {
                assert!(ids.insert(photo.public_id));
                assert!(photo.source_url.starts_with("https://"));
                photos += 1;
            }
        }
        assert_eq!(photos, 34);
        assert_eq!(catalog[0].public_id, SERENITY_SPA_ID);
        assert_eq!(catalog[0].storage_folder, SERENITY_SPA_STORAGE_FOLDER);
        assert_eq!(catalog[0].cover_public_id, SERENITY_SPA_COVER_ID);
        assert_eq!(catalog[0].photos[0].public_id, SERENITY_SPA_FIRST_ID);
        assert_eq!(catalog[0].photos[0].original_filename, "Stones.jpg");
        assert_eq!(catalog[0].photos[6].public_id, SERENITY_SPA_COVER_ID);
    }
}
