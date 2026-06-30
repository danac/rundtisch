import type { Collection } from "../types/portfolio";
import { collectionsMock } from "../data/portfolio.mock";
import { apiGet, hasBackend, withLatency } from "./apiClient";

/**
 * Returns all portfolio collections. Today this resolves bundled mock data;
 * once a backend exists, set `VITE_API_BASE_URL` and this transparently
 * fetches `GET /collections`.
 */
export async function getCollections(
  signal?: AbortSignal,
): Promise<Collection[]> {
  if (hasBackend) {
    return apiGet<Collection[]>("/collections", signal);
  }
  return withLatency(collectionsMock);
}

/**
 * Returns a single collection by URL slug, or undefined if not found.
 * REST: `GET /collections/:slug`
 */
export async function getCollectionBySlug(
  slug: string,
  signal?: AbortSignal,
): Promise<Collection | undefined> {
  if (hasBackend) {
    return apiGet<Collection>(`/collections/${slug}`, signal);
  }
  const collections = await withLatency(collectionsMock);
  return collections.find((c) => c.slug === slug);
}

/**
 * Returns featured collections for the homepage, falling back to the first N
 * when none are marked featured.
 */
export async function getFeaturedCollections(
  limit = 3,
  signal?: AbortSignal,
): Promise<Collection[]> {
  const all = await getCollections(signal);
  const featured = all.filter((c) => c.featured);
  return (featured.length > 0 ? featured : all).slice(0, limit);
}
