import { useState } from 'react'
import { RowsPhotoAlbum } from 'react-photo-album'
import Lightbox from 'yet-another-react-lightbox'
import type { Photo } from '../api'

type PhotoMosaicProps = {
  photos: Photo[]
}

export function PhotoMosaic({ photos }: PhotoMosaicProps) {
  const [index, setIndex] = useState(-1)

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
        />
      </div>
      <Lightbox
        open={index >= 0}
        index={index}
        close={() => setIndex(-1)}
        slides={photos}
        controller={{ closeOnBackdropClick: true }}
      />
    </>
  )
}
