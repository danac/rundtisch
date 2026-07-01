import { homepageMock } from "../data/homepage.mock";
import type { HomepageContent } from "../types/homepage";
import { apiGet, hasBackend, withLatency } from "./apiClient";

/**
 * Returns curated homepage content (hero image, featured collections, category promos).
 * REST: `GET /homepage`
 *
 * TODO(backend-always-on): remove `homepageMock` and the `hasBackend` branch below;
 * always call `apiGet<HomepageContent>('/homepage', signal)`.
 */
export async function getHomepageContent(
  signal?: AbortSignal,
): Promise<HomepageContent> {
  if (hasBackend) {
    return apiGet<HomepageContent>("/homepage", signal);
  }
  return withLatency(homepageMock);
}
