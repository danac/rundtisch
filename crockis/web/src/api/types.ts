export type AuthMethodKind = 'password' | 'passkey'

export type AuthMethod = {
  id: string
  kind: AuthMethodKind
  label: string
  enrolled: boolean
}

export type User = {
  id: string
  email: string
  name: string
  authMethods: AuthMethod[]
}

export type Session = {
  user: User
  token: string
}

export type Collection = {
  id: string
  name: string
  description: string
  cover: Photo
  photoCount: number
}

export type Photo = {
  id: string
  collectionId: string
  src: string
  width: number
  height: number
  alt: string
  title?: string
  takenAt?: string
  /** Original file when it differs from the display `src`. */
  downloadSrc?: string
  /** Download filename, including extension when known. */
  filename?: string
}

export type LoginRequest = {
  email: string
  password: string
}

export type ResetPasswordRequest = {
  password: string
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
  login(request: LoginRequest): Promise<Session>
  logout(token: string): Promise<void>
  me(token: string): Promise<User>
  resetPassword(token: string, request: ResetPasswordRequest): Promise<void>
  listCollections(token: string): Promise<Collection[]>
  getCollection(token: string, id: string): Promise<Collection>
  listPhotos(token: string, collectionId: string): Promise<Photo[]>
}
