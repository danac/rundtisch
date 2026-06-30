export type Currency = "USD" | "EUR" | "GBP";

export interface Product {
  id: string;
  title: string;
  description: string;
  image: string;
  alt: string;
  price: number;
  currency: Currency;
  /** External link to the store / checkout (e.g. Etsy, Shopify). */
  url: string;
  soldOut?: boolean;
}
