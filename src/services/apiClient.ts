/**
 * Minimal fetch wrapper. When `VITE_API_BASE_URL` is set, services call a
 * real REST backend through `apiGet`; otherwise they fall back to bundled
 * mock data. This keeps the door open for a backend without touching the UI.
 */
export const API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? "";

export const hasBackend = API_BASE_URL.length > 0;

export class ApiError extends Error {
  status: number;
  constructor(message: string, status: number) {
    super(message);
    this.name = "ApiError";
    this.status = status;
  }
}

export async function apiGet<T>(path: string, signal?: AbortSignal): Promise<T> {
  const url = `${API_BASE_URL.replace(/\/$/, "")}/${path.replace(/^\//, "")}`;
  const res = await fetch(url, {
    headers: { Accept: "application/json" },
    signal,
  });

  if (!res.ok) {
    throw new ApiError(`Request to ${path} failed`, res.status);
  }

  return (await res.json()) as T;
}

/** Simulates network latency so loading states are visible with mock data. */
export function withLatency<T>(value: T, ms = 450): Promise<T> {
  return new Promise((resolve) => setTimeout(() => resolve(value), ms));
}
