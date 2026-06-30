import type { Artwork } from "../types/portfolio";
import { portfolioMock } from "../data/portfolio.mock";
import { apiGet, hasBackend, withLatency } from "./apiClient";

/**
 * Returns all portfolio artworks. Today this resolves bundled mock data;
 * once a backend exists, set `VITE_API_BASE_URL` and this transparently
 * fetches `GET /artworks`.
 */
export async function getArtworks(signal?: AbortSignal): Promise<Artwork[]> {
  if (hasBackend) {
    return apiGet<Artwork[]>("/artworks", signal);
  }
  return withLatency(portfolioMock);
}

export async function getFeaturedArtworks(
  limit = 6,
  signal?: AbortSignal,
): Promise<Artwork[]> {
  const all = await getArtworks(signal);
  const featured = all.filter((a) => a.featured);
  return (featured.length > 0 ? featured : all).slice(0, limit);
}
