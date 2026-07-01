export type ArtworkCategory = "birds" | "lettering" | "children";

export interface Artwork {
  id: string;
  title: string;
  /** Public path or absolute URL to a lower-resolution thumbnail. */
  thumbnail: string;
  /** Public path or absolute URL to the full-resolution image. */
  image: string;
  /** Short alt / description text for accessibility. */
  alt: string;
  /** Optional year the piece was made. */
  year?: number;
}

export interface Collection {
  id: string;
  /** URL segment: /portfolio/:slug */
  slug: string;
  title: string;
  category: ArtworkCategory;
  description: string;
  /** Marks collections to surface on the homepage. */
  featured?: boolean;
  artworks: Artwork[];
}
