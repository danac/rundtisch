import { mockCollections, mockPhotosByCollection, mockUser } from './mock-data'
import { ApiError, type CrockisApi } from './types'

const wait = (ms = 220) => new Promise((resolve) => setTimeout(resolve, ms))

function requireToken(token: string) {
  if (!token) {
    throw new ApiError(401, 'Sign in to view your library.')
  }
}

export const mockApi: CrockisApi = {
  async login({ email, password }) {
    await wait()
    if (!email.includes('@') || password.length < 1) {
      throw new ApiError(400, 'Enter a valid email and password.')
    }
    return {
      user: { ...mockUser, email, name: email.split('@')[0] ?? 'Friend' },
      token: `mock.${btoa(email)}`,
    }
  },

  async logout(_token) {
    await wait(80)
  },

  async me(token) {
    await wait(80)
    requireToken(token)
    return mockUser
  },

  async listCollections(token) {
    await wait()
    requireToken(token)
    return mockCollections
  },

  async getCollection(token, id) {
    await wait()
    requireToken(token)
    const collection = mockCollections.find((item) => item.id === id)
    if (!collection) {
      throw new ApiError(404, 'Collection not found.')
    }
    return collection
  },

  async listPhotos(token, collectionId) {
    await wait()
    requireToken(token)
    const photos = mockPhotosByCollection[collectionId]
    if (!photos) {
      throw new ApiError(404, 'Collection not found.')
    }
    return photos
  },
}
