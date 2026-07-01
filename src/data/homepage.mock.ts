import type { HomepageContent } from "../types/homepage";
import { collectionsMock } from "./portfolio.mock";

const FEATURED_LIMIT = 3;

function pickFeaturedCollections() {
  const featured = collectionsMock.filter((collection) => collection.featured);
  return (featured.length > 0 ? featured : collectionsMock).slice(
    0,
    FEATURED_LIMIT,
  );
}

/**
 * Temporary stand-in for `GET /homepage` during backend-free development.
 * TODO(backend-always-on): delete this file once the API is always available.
 */
export const homepageMock: HomepageContent = {
  hero: {
    image: "/images/bird-bluetits.png",
    alt: "A pair of blue tit birds on a flowering twig in teal and yellow",
    artworkId: "blue-tits",
    collectionSlug: "garden-birds",
  },
  featuredCollections: pickFeaturedCollections(),
  categoryPromos: [
    {
      category: "birds",
      image: "/images/bird-robin.png",
      alt: "Watercolor robin perched on a blossoming cherry branch",
    },
    {
      category: "lettering",
      image: "/images/lettering-hello.png",
      alt: "Hand-lettered rainbow watercolor word 'Hello' surrounded by flowers",
    },
    {
      category: "children",
      image: "/images/kids-teaparty.png",
      alt: "A fox cub and a little bird having a tea party among flowers",
    },
  ],
};
