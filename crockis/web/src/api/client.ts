import { ApiError, type Collection, type CrockisApi, type Photo } from './types'

async function request<T>(path: string): Promise<T> {
  const response = await fetch(path, { credentials: 'include' })
  if (!response.ok) {
    let message = response.status === 404 ? 'Collection not found.' : 'Unable to load collections.'
    try {
      const body = (await response.json()) as { error?: unknown }
      if (typeof body.error === 'string' && body.error.length > 0) {
        message = body.error
      }
    } catch {
      // Keep the status fallback when the body is not JSON.
    }
    throw new ApiError(response.status, message)
  }
  return (await response.json()) as T
}

export const api: CrockisApi = {
  listCollections() {
    return request<Collection[]>('/api/collections')
  },

  getCollection(id) {
    return request<Collection>(`/api/collections/${encodeURIComponent(id)}`)
  },

  listPhotos(collectionId) {
    return request<Photo[]>(`/api/collections/${encodeURIComponent(collectionId)}/photos`)
  },
}
