import { ApiError, type Collection, type CrockisApi, type Photo, type Session, type User } from './types'

const base = import.meta.env.VITE_API_BASE ?? '/api'

async function request<T>(path: string, token?: string, init?: RequestInit): Promise<T> {
  const headers = new Headers(init?.headers)
  if (!headers.has('Accept')) {
    headers.set('Accept', 'application/json')
  }
  if (init?.body && !headers.has('Content-Type')) {
    headers.set('Content-Type', 'application/json')
  }
  if (token) {
    headers.set('Authorization', `Bearer ${token}`)
  }

  const response = await fetch(`${base}${path}`, { ...init, headers })
  if (!response.ok) {
    const detail = await response.text()
    throw new ApiError(response.status, detail || response.statusText)
  }
  if (response.status === 204) {
    return undefined as T
  }
  return (await response.json()) as T
}

export const restApi: CrockisApi = {
  login: (body) =>
    request<Session>('/auth/login', undefined, {
      method: 'POST',
      body: JSON.stringify(body),
    }),
  logout: (token) => request<void>('/auth/logout', token, { method: 'POST' }),
  me: (token) => request<User>('/auth/me', token),
  listCollections: (token) => request<Collection[]>('/collections', token),
  getCollection: (token, id) => request<Collection>(`/collections/${id}`, token),
  listPhotos: (token, collectionId) =>
    request<Photo[]>(`/collections/${collectionId}/photos`, token),
}
