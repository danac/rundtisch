export type Collection = {
  /** Hyphenated UUID. `/collections/:collectionId` uses this value. */
  id: string
  name: string
  description: string
  cover: Photo | null
  photoCount: number
}

export type Photo = {
  /** Hyphenated UUID. Routes and `/api/photos/{id}/file` use this, not the storage filename. */
  id: string
  collectionId: string
  src: string
  width: number
  height: number
  takenAt?: string
  /** Original file when it differs from the display `src`. */
  downloadSrc?: string
  /** Download filename, including extension when known. */
  filename?: string
}

export class ApiError extends Error {
  readonly status: number

  constructor(status: number, message: string) {
    super(message)
    this.name = 'ApiError'
    this.status = status
  }
}

export type CrockisApi = {
  listCollections(): Promise<Collection[]>
  getCollection(id: string): Promise<Collection>
  listPhotos(collectionId: string): Promise<Photo[]>
}
