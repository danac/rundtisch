import type { Product } from "../types/merch";

/**
 * Local sample shop data. Mirrors a future REST response so `merchService`
 * can switch to a live API without changing the UI.
 */
export const merchMock: Product[] = [
  {
    id: "robin-mug",
    title: "Robin Watercolor Mug",
    description: "Ceramic mug featuring the original robin painting. Dishwasher safe.",
    image: "/images/merch-mug.png",
    alt: "White ceramic mug with a watercolor robin illustration",
    price: 18,
    currency: "USD",
    url: "https://example.com/shop/robin-mug",
  },
  {
    id: "bluebird-tote",
    title: "Bluebird Cotton Tote",
    description: "Sturdy natural cotton tote printed with a bluebird and flowers.",
    image: "/images/merch-tote.png",
    alt: "Natural cotton tote bag printed with a watercolor bluebird",
    price: 24,
    currency: "USD",
    url: "https://example.com/shop/bluebird-tote",
  },
  {
    id: "bird-card-set",
    title: "Bird Greeting Card Set",
    description: "Set of 5 blank cards with envelopes, each a different bird.",
    image: "/images/merch-cards.png",
    alt: "A fanned set of greeting cards featuring watercolor birds",
    price: 16,
    currency: "USD",
    url: "https://example.com/shop/bird-card-set",
  },
  {
    id: "robin-print",
    title: "Robin on Blossom - Art Print",
    description: "Giclée print on archival paper. Available in several sizes.",
    image: "/images/bird-robin.png",
    alt: "Art print of a robin perched on a blossoming branch",
    price: 22,
    currency: "USD",
    url: "https://example.com/shop/robin-print",
  },
  {
    id: "hello-print",
    title: "Hello - Lettering Print",
    description: "Bright hand-lettered print to cheer up any wall or desk.",
    image: "/images/lettering-hello.png",
    alt: "Art print of the rainbow watercolor word 'Hello'",
    price: 20,
    currency: "USD",
    url: "https://example.com/shop/hello-print",
  },
  {
    id: "owl-print",
    title: "Autumn Owl - Art Print",
    description: "Cozy autumn owl giclée print on archival cotton paper.",
    image: "/images/bird-owl.png",
    alt: "Art print of a watercolor owl among autumn leaves",
    price: 22,
    currency: "USD",
    url: "https://example.com/shop/owl-print",
    soldOut: true,
  },
];
