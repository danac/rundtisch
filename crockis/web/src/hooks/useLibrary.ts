import { useQuery } from '@tanstack/react-query'
import { api } from '../api'
import { useAuth } from '../auth/useAuth'

export function useCollections() {
  const { token } = useAuth()
  return useQuery({
    queryKey: ['collections'],
    enabled: Boolean(token),
    queryFn: () => api.listCollections(token!),
  })
}

export function useCollection(collectionId: string | undefined) {
  const { token } = useAuth()
  return useQuery({
    queryKey: ['collections', collectionId],
    enabled: Boolean(token && collectionId),
    queryFn: () => api.getCollection(token!, collectionId!),
  })
}

export function usePhotos(collectionId: string | undefined) {
  const { token } = useAuth()
  return useQuery({
    queryKey: ['collections', collectionId, 'photos'],
    enabled: Boolean(token && collectionId),
    queryFn: () => api.listPhotos(token!, collectionId!),
  })
}
