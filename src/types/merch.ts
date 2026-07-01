export type Currency = "USD" | "EUR" | "GBP";

export interface Product {
  id: string;
  title: string;
  description: string;
  /** Public path or absolute URL to a lower-resolution thumbnail. */
  thumbnail: string;
  /** Public path or absolute URL to the full-resolution image. */
  image: string;
  alt: string;
  price: number;
  currency: Currency;
  /** External link to the store / checkout (e.g. Etsy, Shopify). */
  url: string;
  soldOut?: boolean;
}
