import { useEffect, useState } from 'react'
import { Link, useParams } from 'react-router-dom'
import { ApiError } from '../api'
import { useAuth } from '../auth/useAuth'
import { Header } from '../components/Header'
import { PhotoMosaic } from '../components/PhotoMosaic'
import { StatusState } from '../components/StatusState'
import { downloadPhotos } from '../download'
import { useCollection, usePhotos } from '../hooks/useLibrary'

export function CollectionPage() {
  const { token } = useAuth()
  const { collectionId } = useParams()
  const collection = useCollection(collectionId)
  const photos = usePhotos(collectionId)
  const pending = collection.isPending || photos.isPending
  const error = collection.error ?? photos.error
  const [selecting, setSelecting] = useState(false)
  const [selectedIds, setSelectedIds] = useState<Set<string>>(() => new Set())
  const [anchor, setAnchor] = useState(-1)
  const [downloading, setDownloading] = useState(false)
  const [selectionFor, setSelectionFor] = useState(collectionId)

  if (selectionFor !== collectionId) {
    setSelectionFor(collectionId)
    setSelecting(false)
    setSelectedIds(new Set())
    setAnchor(-1)
  }

  useEffect(() => {
    if (!selecting) return
    function onKey(event: KeyboardEvent) {
      if (event.key === 'Escape') {
        setSelecting(false)
        setSelectedIds(new Set())
        setAnchor(-1)
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [selecting])

  function stopSelecting() {
    setSelecting(false)
    setSelectedIds(new Set())
    setAnchor(-1)
  }

  function toggle(index: number, shift: boolean) {
    const list = photos.data
    if (!list) return
    const photo = list[index]
    if (!photo) return
    setSelectedIds((current) => {
      const next = new Set(current)
      if (shift && anchor >= 0) {
        const start = Math.min(anchor, index)
        const end = Math.max(anchor, index)
        for (let cursor = start; cursor <= end; cursor += 1) {
          const item = list[cursor]
          if (item) next.add(item.id)
        }
        return next
      }
      if (next.has(photo.id)) next.delete(photo.id)
      else next.add(photo.id)
      return next
    })
    setAnchor(index)
  }

  function selectAll() {
    const list = photos.data ?? []
    setSelectedIds(new Set(list.map((photo) => photo.id)))
    setAnchor(list.length - 1)
  }

  async function saveSelection() {
    const list = photos.data
    const name = collection.data?.name
    if (!list || !name) return
    const chosen = list.filter((photo) => selectedIds.has(photo.id))
    if (chosen.length === 0) return
    setDownloading(true)
    try {
      await downloadPhotos(name, chosen, token)
    } finally {
      setDownloading(false)
    }
  }

  const count = selectedIds.size
  const ready = Boolean(photos.data && !error)

  return (
    <div className={selecting ? 'flex min-h-dvh flex-col pb-24' : 'flex min-h-dvh flex-col'}>
      <Header
        title={collection.data?.name}
        selecting={selecting}
        onToggleSelect={ready ? () => (selecting ? stopSelecting() : setSelecting(true)) : undefined}
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
        {photos.data ? (
          <PhotoMosaic
            photos={photos.data}
            selecting={selecting}
            selectedIds={selectedIds}
            onToggle={toggle}
          />
        ) : null}
      </main>
      {selecting ? (
        <SelectionBar
          count={count}
          busy={downloading}
          onAll={selectAll}
          onDownload={() => {
            void saveSelection()
          }}
          onCancel={stopSelecting}
        />
      ) : null}
    </div>
  )
}

function SelectionBar({
  count,
  busy,
  onAll,
  onDownload,
  onCancel,
}: {
  count: number
  busy: boolean
  onAll: () => void
  onDownload: () => void
  onCancel: () => void
}) {
  return (
    <div className="selection-bar">
      <p className="font-display text-[12px] font-medium tracking-[0.16em] uppercase text-ink">
        {count} selected
      </p>
      <div className="flex items-center gap-5">
        <button type="button" onClick={onAll} className="selection-bar__action">
          All
        </button>
        <button
          type="button"
          onClick={onDownload}
          disabled={count === 0 || busy}
          className="selection-bar__action"
        >
          {busy ? 'Preparing' : 'Download'}
        </button>
        <button type="button" onClick={onCancel} className="selection-bar__action">
          Cancel
        </button>
      </div>
    </div>
  )
}
