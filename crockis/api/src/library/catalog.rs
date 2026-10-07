//! Seed catalog transcribed from the Crockis frontend mock library
//! (`serenity-spa`, `alpine-light`, `summer-garden`, `north-coast`).

use time::OffsetDateTime;
use time::macros::datetime;

/// Capture timestamp shared by the seeded mock library.
pub const MOCK_CAPTURED_AT: OffsetDateTime = datetime!(2026-03-12 10:00 UTC);

pub struct CatalogPhoto {
    pub public_id: &'static str,
    pub source_url: &'static str,
    pub alt: &'static str,
    pub title: &'static str,
    /// Preferred download name. The stored extension follows the downloaded bytes.
    pub original_filename: &'static str,
    pub sort_order: i32,
}

pub struct CatalogCollection {
    pub public_id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub created_at: OffsetDateTime,
    pub sort_order: i32,
    pub cover_public_id: &'static str,
    pub photos: &'static [CatalogPhoto],
}

const SERENITY_SPA: &[CatalogPhoto] = &[
    CatalogPhoto {
        public_id: "serenity-spa-01",
        source_url: "https://images.unsplash.com/photo-1600334129128-685c5582fd35?auto=format&fit=crop&w=900&h=900&q=80",
        alt: "Hot stones along a guest’s back",
        title: "Stones",
        original_filename: "Stones.jpg",
        sort_order: 0,
    },
    CatalogPhoto {
        public_id: "serenity-spa-02",
        source_url: "https://images.unsplash.com/photo-1560750588-73207b1ef5b8?auto=format&fit=crop&w=1600&h=900&q=80",
        alt: "Private lounge beside an indoor pool",
        title: "Pavilion",
        original_filename: "Pavilion.jpg",
        sort_order: 1,
    },
    CatalogPhoto {
        public_id: "serenity-spa-03",
        source_url: "https://images.unsplash.com/photo-1544161515-4ab6ce6db874?auto=format&fit=crop&w=1600&h=900&q=80",
        alt: "Oil poured during a massage",
        title: "Touch",
        original_filename: "Touch.jpg",
        sort_order: 2,
    },
    CatalogPhoto {
        public_id: "serenity-spa-04",
        source_url: "https://images.pexels.com/photos/3188/love-romantic-bath-candlelight.jpg?auto=compress&cs=tinysrgb&w=900",
        alt: "Rolled towel with candles and a tulip",
        title: "Ritual",
        original_filename: "love-romantic-bath-candlelight.jpg",
        sort_order: 3,
    },
    CatalogPhoto {
        public_id: "serenity-spa-05",
        source_url: "https://images.unsplash.com/photo-1515377905703-c4788e51af15?auto=format&fit=crop&w=1400&h=900&q=80",
        alt: "Drop of essential oil",
        title: "Oil",
        original_filename: "Oil.jpg",
        sort_order: 4,
    },
    CatalogPhoto {
        public_id: "serenity-spa-06",
        source_url: "https://images.unsplash.com/photo-1418065460487-3e41a6c84dc5?auto=format&fit=crop&w=1600&h=1000&q=80",
        alt: "Fog over a pine forest",
        title: "Grove",
        original_filename: "Grove.jpg",
        sort_order: 5,
    },
    CatalogPhoto {
        public_id: "serenity-spa-07",
        source_url: "https://images.unsplash.com/photo-1583416750470-965b2707b355?auto=format&fit=crop&w=1000&h=900&q=80",
        alt: "Cedar sauna in low light",
        title: "Heat",
        original_filename: "Heat.jpg",
        sort_order: 6,
    },
    CatalogPhoto {
        public_id: "serenity-spa-08",
        source_url: "https://images.unsplash.com/photo-1519824145371-296894a0daa9?auto=format&fit=crop&w=900&h=1100&q=80",
        alt: "Hands during a massage",
        title: "Hands",
        original_filename: "Hands.jpg",
        sort_order: 7,
    },
    CatalogPhoto {
        public_id: "serenity-spa-09",
        source_url: "https://images.unsplash.com/photo-1507652313519-d4e9174996dd?auto=format&fit=crop&w=1600&h=900&q=80",
        alt: "Freestanding bath in a quiet room",
        title: "Bath",
        original_filename: "Bath.jpg",
        sort_order: 8,
    },
    CatalogPhoto {
        public_id: "serenity-spa-10",
        source_url: "https://images.pexels.com/photos/3764568/pexels-photo-3764568.jpeg?auto=compress&cs=tinysrgb&w=1000",
        alt: "Neck and shoulder treatment",
        title: "Quiet",
        original_filename: "pexels-photo-3764568.jpeg",
        sort_order: 9,
    },
    CatalogPhoto {
        public_id: "serenity-spa-11",
        source_url: "https://images.unsplash.com/photo-1441974231531-c6227db76b6e?auto=format&fit=crop&w=1100&h=900&q=80",
        alt: "Path through a green forest",
        title: "Walk",
        original_filename: "Walk.jpg",
        sort_order: 10,
    },
    CatalogPhoto {
        public_id: "serenity-spa-12",
        source_url: "https://images.pexels.com/photos/3865530/pexels-photo-3865530.jpeg?auto=compress&cs=tinysrgb&w=900",
        alt: "Close massage on the collarbone",
        title: "Soft",
        original_filename: "pexels-photo-3865530.jpeg",
        sort_order: 11,
    },
    CatalogPhoto {
        public_id: "serenity-spa-13",
        source_url: "https://images.unsplash.com/photo-1570172619644-dfd03ed5d881?auto=format&fit=crop&w=1600&h=900&q=80",
        alt: "Facial mask applied with a brush",
        title: "Mask",
        original_filename: "Mask.jpg",
        sort_order: 12,
    },
    CatalogPhoto {
        public_id: "serenity-spa-14",
        source_url: "https://images.pexels.com/photos/3757942/pexels-photo-3757942.jpeg?auto=compress&cs=tinysrgb&w=1000",
        alt: "Guest resting after treatment",
        title: "Rest",
        original_filename: "pexels-photo-3757942.jpeg",
        sort_order: 13,
    },
    CatalogPhoto {
        public_id: "serenity-spa-15",
        source_url: "https://images.pexels.com/photos/3865676/pexels-photo-3865676.jpeg?auto=compress&cs=tinysrgb&w=1400",
        alt: "Oils, flowers, and stones",
        title: "Blend",
        original_filename: "pexels-photo-3865676.jpeg",
        sort_order: 14,
    },
    CatalogPhoto {
        public_id: "serenity-spa-16",
        source_url: "https://images.unsplash.com/photo-1519823551278-64ac92734fb1?auto=format&fit=crop&w=900&h=1100&q=80",
        alt: "Therapist’s hands on the back",
        title: "Close",
        original_filename: "Close.jpg",
        sort_order: 15,
    },
];

const ALPINE_LIGHT: &[CatalogPhoto] = &[
    CatalogPhoto {
        public_id: "alpine-light-01",
        source_url: "https://images.unsplash.com/photo-1464822759023-fed622ff2c3b?auto=format&fit=crop&w=1600&h=1000&q=80",
        alt: "Sunlit alpine ridge",
        title: "Ridge",
        original_filename: "Ridge.jpg",
        sort_order: 0,
    },
    CatalogPhoto {
        public_id: "alpine-light-02",
        source_url: "https://images.unsplash.com/photo-1483728642387-6c3bdd6c93e5?auto=format&fit=crop&w=1200&h=1600&q=80",
        alt: "Snow peak against a clear sky",
        title: "Summit",
        original_filename: "Summit.jpg",
        sort_order: 1,
    },
    CatalogPhoto {
        public_id: "alpine-light-03",
        source_url: "https://images.unsplash.com/photo-1469474968028-56623f02e42e?auto=format&fit=crop&w=1600&h=900&q=80",
        alt: "Valley at sunrise",
        title: "Valley",
        original_filename: "Valley.jpg",
        sort_order: 2,
    },
    CatalogPhoto {
        public_id: "alpine-light-04",
        source_url: "https://images.unsplash.com/photo-1470071459604-3b5ec3a7fe05?auto=format&fit=crop&w=1400&h=900&q=80",
        alt: "Fog over mountain pines",
        title: "Fog",
        original_filename: "Fog.jpg",
        sort_order: 3,
    },
    CatalogPhoto {
        public_id: "alpine-light-05",
        source_url: "https://images.unsplash.com/photo-1501785888041-af3ef285b470?auto=format&fit=crop&w=900&h=1200&q=80",
        alt: "Lake below the peaks",
        title: "Lake",
        original_filename: "Lake.jpg",
        sort_order: 4,
    },
    CatalogPhoto {
        public_id: "alpine-light-06",
        source_url: "https://images.unsplash.com/photo-1519681393784-d120267933ba?auto=format&fit=crop&w=1600&h=900&q=80",
        alt: "Night sky over a mountain range",
        title: "Night",
        original_filename: "Night.jpg",
        sort_order: 5,
    },
];

const SUMMER_GARDEN: &[CatalogPhoto] = &[
    CatalogPhoto {
        public_id: "summer-garden-01",
        source_url: "https://images.unsplash.com/photo-1466781783364-36c955e42a7f?auto=format&fit=crop&w=1400&h=900&q=80",
        alt: "Lush garden path",
        title: "Path",
        original_filename: "Path.jpg",
        sort_order: 0,
    },
    CatalogPhoto {
        public_id: "summer-garden-02",
        source_url: "https://images.unsplash.com/photo-1490750967868-88aa4486c946?auto=format&fit=crop&w=900&h=1200&q=80",
        alt: "Close-up of spring flowers",
        title: "Bloom",
        original_filename: "Bloom.jpg",
        sort_order: 1,
    },
    CatalogPhoto {
        public_id: "summer-garden-03",
        source_url: "https://images.unsplash.com/photo-1501004318641-b39e6451bec6?auto=format&fit=crop&w=1200&h=900&q=80",
        alt: "Greenhouse foliage",
        title: "Glass",
        original_filename: "Glass.jpg",
        sort_order: 2,
    },
    CatalogPhoto {
        public_id: "summer-garden-04",
        source_url: "https://images.unsplash.com/photo-1441974231531-c6227db76b6e?auto=format&fit=crop&w=1000&h=900&q=80",
        alt: "Woodland garden walk",
        title: "Wood",
        original_filename: "Wood.jpg",
        sort_order: 3,
    },
    CatalogPhoto {
        public_id: "summer-garden-05",
        source_url: "https://images.unsplash.com/photo-1418065460487-3e41a6c84dc5?auto=format&fit=crop&w=1600&h=900&q=80",
        alt: "Mist in the trees",
        title: "Mist",
        original_filename: "Mist.jpg",
        sort_order: 4,
    },
];

const NORTH_COAST: &[CatalogPhoto] = &[
    CatalogPhoto {
        public_id: "north-coast-01",
        source_url: "https://images.unsplash.com/photo-1507525428034-b723cf961d3e?auto=format&fit=crop&w=1600&h=900&q=80",
        alt: "Turquoise shoreline",
        title: "Shore",
        original_filename: "Shore.jpg",
        sort_order: 0,
    },
    CatalogPhoto {
        public_id: "north-coast-02",
        source_url: "https://images.unsplash.com/photo-1500375592092-40eb2168fd21?auto=format&fit=crop&w=1200&h=1600&q=80",
        alt: "Cliff above the sea",
        title: "Cliff",
        original_filename: "Cliff.jpg",
        sort_order: 1,
    },
    CatalogPhoto {
        public_id: "north-coast-03",
        source_url: "https://images.unsplash.com/photo-1505118380757-91f5f5632de0?auto=format&fit=crop&w=1600&h=1000&q=80",
        alt: "Waves from above",
        title: "Tide",
        original_filename: "Tide.jpg",
        sort_order: 2,
    },
    CatalogPhoto {
        public_id: "north-coast-04",
        source_url: "https://images.unsplash.com/photo-1439066615861-d1af74d74000?auto=format&fit=crop&w=1400&h=900&q=80",
        alt: "Calm mountain lake",
        title: "Still",
        original_filename: "Still.jpg",
        sort_order: 3,
    },
    CatalogPhoto {
        public_id: "north-coast-05",
        source_url: "https://images.unsplash.com/photo-1475924156734-496f6cac6ec1?auto=format&fit=crop&w=900&h=1100&q=80",
        alt: "Evening surf",
        title: "Surf",
        original_filename: "Surf.jpg",
        sort_order: 4,
    },
    CatalogPhoto {
        public_id: "north-coast-06",
        source_url: "https://images.unsplash.com/photo-1507525428034-b723cf961d3e?auto=format&fit=crop&w=1600&h=900&q=80",
        alt: "Warm sand and sea",
        title: "Bay",
        original_filename: "Bay.jpg",
        sort_order: 5,
    },
    CatalogPhoto {
        public_id: "north-coast-07",
        source_url: "https://images.unsplash.com/photo-1473116763249-2faaef81ccda?auto=format&fit=crop&w=1000&h=900&q=80",
        alt: "Open water",
        title: "Horizon",
        original_filename: "Horizon.jpg",
        sort_order: 6,
    },
];

pub static MOCK_CATALOG: &[CatalogCollection] = &[
    CatalogCollection {
        public_id: "serenity-spa",
        name: "Serenity Spa",
        description: "A quiet weekend of water, heat, and low light.",
        created_at: datetime!(2026-01-01 0:00 UTC),
        sort_order: 0,
        cover_public_id: "serenity-spa-07",
        photos: SERENITY_SPA,
    },
    CatalogCollection {
        public_id: "alpine-light",
        name: "Alpine Light",
        description: "High passes and still lakes after the thaw.",
        created_at: datetime!(2026-01-02 0:00 UTC),
        sort_order: 1,
        cover_public_id: "alpine-light-01",
        photos: ALPINE_LIGHT,
    },
    CatalogCollection {
        public_id: "summer-garden",
        name: "Summer Garden",
        description: "Rain, glasshouses, and late flowers.",
        created_at: datetime!(2026-01-03 0:00 UTC),
        sort_order: 2,
        cover_public_id: "summer-garden-01",
        photos: SUMMER_GARDEN,
    },
    CatalogCollection {
        public_id: "north-coast",
        name: "North Coast",
        description: "Tide lines and long evenings by the water.",
        created_at: datetime!(2026-01-04 0:00 UTC),
        sort_order: 3,
        cover_public_id: "north-coast-01",
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
        assert_eq!(catalog[0].public_id, "serenity-spa");
        assert_eq!(catalog[0].cover_public_id, "serenity-spa-07");
        assert_eq!(catalog[0].photos[0].title, "Stones");
    }
}
