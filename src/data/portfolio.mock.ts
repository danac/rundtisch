import type { Artwork } from "../types/portfolio";

/**
 * Local sample portfolio data. This stands in for a future REST response;
 * the shape matches the `Artwork` type so the service can swap to `fetch`
 * without touching any components.
 */
export const portfolioMock: Artwork[] = [
  {
    id: "robin-blossom",
    title: "Robin on Blossom",
    category: "birds",
    image: "/images/bird-robin.png",
    alt: "Watercolor robin perched on a blossoming cherry branch",
    year: 2026,
    featured: true,
  },
  {
    id: "blue-tits",
    title: "Two Blue Tits",
    category: "birds",
    image: "/images/bird-bluetits.png",
    alt: "A pair of blue tit birds on a flowering twig in teal and yellow",
    year: 2026,
    featured: true,
  },
  {
    id: "autumn-owl",
    title: "Autumn Owl",
    category: "birds",
    image: "/images/bird-owl.png",
    alt: "A friendly watercolor owl among autumn leaves",
    year: 2025,
    featured: true,
  },
  {
    id: "hummingbird",
    title: "Hummingbird & Trumpet Flower",
    category: "birds",
    image: "/images/bird-hummingbird.png",
    alt: "An iridescent hummingbird beside a pink trumpet flower",
    year: 2026,
    featured: true,
  },
  {
    id: "hello-lettering",
    title: "Hello in Color",
    category: "lettering",
    image: "/images/lettering-hello.png",
    alt: "Hand-lettered rainbow watercolor word 'Hello' surrounded by flowers",
    year: 2025,
    featured: true,
  },
  {
    id: "be-kind-lettering",
    title: "Be Kind",
    category: "lettering",
    image: "/images/lettering-bekind.png",
    alt: "Hand-lettered 'Be Kind' in pink and blue watercolor with hearts",
    year: 2026,
  },
  {
    id: "tea-party",
    title: "The Tea Party",
    category: "children",
    image: "/images/kids-teaparty.png",
    alt: "A fox cub and a little bird having a tea party among flowers",
    year: 2026,
    featured: true,
  },
  {
    id: "story-bunnies",
    title: "Storytime Bunnies",
    category: "children",
    image: "/images/kids-bunnies.png",
    alt: "Two bunnies reading a book under a toadstool in the forest",
    year: 2025,
  },
];
