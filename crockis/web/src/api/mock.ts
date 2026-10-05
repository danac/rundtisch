import { mockCollections, mockPhotosByCollection } from './mock-data'
import { ApiError, type CrockisApi } from './types'

const wait = (ms = 220) => new Promise((resolve) => setTimeout(resolve, ms))

export const mockApi: CrockisApi = {
  async listCollections() {
    await wait()
    return mockCollections
  },

  async getCollection(id) {
    await wait()
    const collection = mockCollections.find((item) => item.id === id)
    if (!collection) {
      throw new ApiError(404, 'Collection not found.')
    }
    return collection
  },

  async listPhotos(collectionId) {
    await wait()
    const photos = mockPhotosByCollection[collectionId]
    if (!photos) {
      throw new ApiError(404, 'Collection not found.')
    }
    return photos
  },
}
