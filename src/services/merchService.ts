import type { Product } from "../types/merch";
import { merchMock } from "../data/merch.mock";
import { apiGet, hasBackend, withLatency } from "./apiClient";

/**
 * Returns shop products. Resolves bundled mock data today; once a backend
 * exists, set `VITE_API_BASE_URL` and this transparently fetches
 * `GET /products`.
 */
export async function getProducts(signal?: AbortSignal): Promise<Product[]> {
  if (hasBackend) {
    return apiGet<Product[]>("/products", signal);
  }
  return withLatency(merchMock);
}
