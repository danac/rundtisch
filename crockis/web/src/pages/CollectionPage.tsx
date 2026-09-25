import { useState } from 'react'
import { Link, useParams } from 'react-router-dom'
import { ApiError } from '../api'
import { useAuth } from '../auth/useAuth'
import { DownloadControl } from '../components/DownloadControl'
import { Header } from '../components/Header'
import { PhotoMosaic } from '../components/PhotoMosaic'
import { StatusState } from '../components/StatusState'
import { downloadCollection } from '../download'
import { useCollection, usePhotos } from '../hooks/useLibrary'

export function CollectionPage() {
  const { token } = useAuth()
  const { collectionId } = useParams()
  const collection = useCollection(collectionId)
  const photos = usePhotos(collectionId)
  const pending = collection.isPending || photos.isPending
  const error = collection.error ?? photos.error
  const [downloading, setDownloading] = useState(false)

  async function saveCollection() {
    if (!collection.data || !photos.data) return
    setDownloading(true)
    try {
      await downloadCollection(collection.data, photos.data, token)
    } finally {
      setDownloading(false)
    }
  }

  return (
    <div className="flex min-h-dvh flex-col">
      <Header
        title={collection.data?.name}
        accessory={
          photos.data && collection.data ? (
            <DownloadControl
              variant="inline"
              label={`Download ${collection.data.name}`}
              busy={downloading}
              onDownload={() => {
                void saveCollection()
              }}
            />
          ) : null
        }
      />
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
