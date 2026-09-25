import { ApiError } from '../api'
import { CollectionMosaic } from '../components/CollectionMosaic'
import { Header } from '../components/Header'
import { StatusState } from '../components/StatusState'
import { useCollections } from '../hooks/useLibrary'

export function CollectionsPage() {
  const { data, isPending, error } = useCollections()

  return (
    <div className="flex min-h-dvh flex-col">
      <Header />
      <main className="flex-1">
        {isPending ? <StatusState message="Gathering collections" /> : null}
        {error ? (
          <StatusState
            message={error instanceof ApiError ? error.message : 'Unable to load collections.'}
          />
        ) : null}
        {data ? <CollectionMosaic collections={data} /> : null}
      </main>
    </div>
  )
}
