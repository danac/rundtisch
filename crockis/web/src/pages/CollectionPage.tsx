import { Link, useParams } from 'react-router-dom'
import { ApiError } from '../api'
import { Header } from '../components/Header'
import { PhotoMosaic } from '../components/PhotoMosaic'
import { StatusState } from '../components/StatusState'
import { useCollection, usePhotos } from '../hooks/useLibrary'

export function CollectionPage() {
  const { collectionId } = useParams()
  const collection = useCollection(collectionId)
  const photos = usePhotos(collectionId)
  const pending = collection.isPending || photos.isPending
  const error = collection.error ?? photos.error

  return (
    <div className="flex min-h-dvh flex-col">
      <Header title={collection.data?.name} />
      <main className="flex-1">
        {pending ? <StatusState message="Opening collection" /> : null}
        {error ? (
          <div className="px-6">
            <StatusState
              message={error instanceof ApiError ? error.message : 'Unable to load this collection.'}
            />
            <p className="pb-16 text-center">
              <Link
                to="/collections"
                className="font-display text-[11px] tracking-[0.24em] uppercase text-mist hover:text-ink"
              >
                Back to collections
              </Link>
            </p>
          </div>
        ) : null}
        {photos.data ? <PhotoMosaic photos={photos.data} /> : null}
      </main>
    </div>
  )
}
