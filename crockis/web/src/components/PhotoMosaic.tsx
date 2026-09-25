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
}

export function PhotoMosaic({ photos }: PhotoMosaicProps) {
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
      <div className="photo-mosaic">
        <RowsPhotoAlbum
          photos={photos}
          spacing={8}
          padding={0}
          targetRowHeight={300}
          rowConstraints={{ maxPhotos: 4 }}
          onClick={({ index: next }) => setIndex(next)}
          render={{
            extras: (_, { photo }) => (
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
