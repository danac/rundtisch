import type { ArtworkCategory, Collection } from "./portfolio";

/** Hero artwork/media selected for the homepage first section. */
export interface HomepageHero {
  image: string;
  alt: string;
  /** Source artwork id when the hero image is a portfolio piece. */
  artworkId?: string;
  /** Parent collection slug for optional deep-linking. */
  collectionSlug?: string;
}

/** Curated category card shown on the homepage. Copy stays in i18n; image comes from CMS. */
export interface HomepageCategoryPromo {
  category: ArtworkCategory;
  image: string;
  alt: string;
}

/**
 * Curated homepage payload returned by `GET /homepage`.
 * Marketing copy (headings, CTAs) remains in locale files; this carries
 * only CMS-selected media and portfolio references.
 */
export interface HomepageContent {
  hero: HomepageHero;
  featuredCollections: Collection[];
  categoryPromos: HomepageCategoryPromo[];
  /** ISO timestamp for cache busting / debugging. */
  updatedAt?: string;
}
