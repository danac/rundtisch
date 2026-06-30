export type ArtworkCategory = "birds" | "lettering" | "children";

export interface Artwork {
  id: string;
  title: string;
  category: ArtworkCategory;
  /** Public path or absolute URL to the image. */
  image: string;
  /** Short alt / description text for accessibility. */
  alt: string;
  /** Optional year the piece was made. */
  year?: number;
  /** Marks pieces to surface on the homepage. */
  featured?: boolean;
}
