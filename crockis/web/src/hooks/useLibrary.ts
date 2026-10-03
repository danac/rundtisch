import { useQuery } from '@tanstack/react-query'
import { api } from '../api'
import { useAuth } from '../auth/useAuth'

export function useCollections() {
  const { user } = useAuth()
  return useQuery({
    queryKey: ['collections'],
    enabled: Boolean(user),
    queryFn: () => api.listCollections(),
  })
}

export function useCollection(collectionId: string | undefined) {
  const { user } = useAuth()
  return useQuery({
    queryKey: ['collections', collectionId],
    enabled: Boolean(user && collectionId),
    queryFn: () => api.getCollection(collectionId!),
  })
}

export function usePhotos(collectionId: string | undefined) {
  const { user } = useAuth()
  return useQuery({
    queryKey: ['collections', collectionId, 'photos'],
    enabled: Boolean(user && collectionId),
    queryFn: () => api.listPhotos(collectionId!),
  })
}
