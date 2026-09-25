import { useState } from 'react'
import { RowsPhotoAlbum } from 'react-photo-album'
import Lightbox from 'yet-another-react-lightbox'
import Download from 'yet-another-react-lightbox/plugins/download'
import type { Photo } from '../api'
import { useAuth } from '../auth/useAuth'
import { downloadPhoto } from '../download'
import { DownloadControl } from './DownloadControl'

type PhotoMosaicProps = {
  photos: Photo[]
  selecting?: boolean
  selectedIds?: ReadonlySet<string>
  onToggle?: (index: number, shift: boolean) => void
}

export function PhotoMosaic({ photos, selecting = false, selectedIds, onToggle }: PhotoMosaicProps) {
  const { token } = useAuth()
  const [index, setIndex] = useState(-1)
  const [busyId, setBusyId] = useState<string | null>(null)

  async function save(photo: Photo) {
    setBusyId(photo.id)
    try {
      await downloadPhoto(photo, token)
    } finally {
      setBusyId(null)
    }
  }

  return (
    <>
      <div className={selecting ? 'photo-mosaic is-selecting' : 'photo-mosaic'}>
        <RowsPhotoAlbum
          photos={photos}
          spacing={8}
          padding={0}
          targetRowHeight={300}
          rowConstraints={{ maxPhotos: 4 }}
          onClick={({ index: next, event }) => {
            if (selecting) {
              onToggle?.(next, event.shiftKey)
              return
            }
            setIndex(next)
          }}
          render={{
            button: ({ className, ...props }, { photo }) => (
              <button
                {...props}
                className={[className, selectedIds?.has(photo.id) ? 'is-selected' : '']
                  .filter(Boolean)
                  .join(' ')}
                aria-pressed={selecting ? selectedIds?.has(photo.id) : undefined}
              />
            ),
            extras: (_, { photo }) =>
              selecting ? (
                <span className="select-mark" aria-hidden>
                  <svg viewBox="0 0 24 24" className="h-3 w-3" fill="none">
                    <path
                      d="M6.5 12.5 10 16l7.5-8"
                      stroke="currentColor"
                      strokeWidth="1.8"
                      strokeLinecap="round"
                      strokeLinejoin="round"
                    />
                  </svg>
                </span>
              ) : (
                <DownloadControl
                  nested
                  label={`Download ${photo.title ?? photo.alt}`}
                  busy={busyId === photo.id}
                  onDownload={() => {
                    void save(photo)
                  }}
                />
              ),
          }}
        />
      </div>
      <Lightbox
        plugins={[Download]}
        open={index >= 0}
        index={index}
        close={() => setIndex(-1)}
        slides={photos}
        controller={{ closeOnBackdropClick: true }}
        download={{
          download: ({ slide }) => {
            const photo = photos.find((item) => item.src === slide.src)
            if (photo) void save(photo)
          },
        }}
      />
    </>
  )
}
