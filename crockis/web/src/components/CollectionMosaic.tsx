import { Link } from 'react-router-dom'
import type { Collection } from '../api'

type CollectionMosaicProps = {
  collections: Collection[]
}

export function CollectionMosaic({ collections }: CollectionMosaicProps) {
  return (
    <div className="photo-mosaic grid grid-cols-1 gap-2 sm:grid-cols-2 lg:grid-cols-4">
      {collections.map((collection, index) => {
        const featured = index === 0
        return (
          <Link
            key={collection.id}
            to={`/collections/${collection.id}`}
            className={[
              'group relative block overflow-hidden bg-void',
              featured ? 'sm:col-span-2 lg:col-span-2 lg:row-span-2' : '',
            ].join(' ')}
          >
            <img
              src={collection.cover.src}
              alt={collection.cover.alt}
              className={[
                'w-full object-cover',
                featured ? 'aspect-[16/10] lg:h-full lg:aspect-auto' : 'aspect-[4/3]',
              ].join(' ')}
            />
            <div className="pointer-events-none absolute inset-0 z-10 bg-gradient-to-t from-black/70 via-black/10 to-transparent opacity-90 transition-opacity group-hover:opacity-100" />
            <div className="absolute inset-x-0 bottom-0 z-10 p-5 md:p-6">
              <h2 className="font-display text-lg font-medium tracking-[0.22em] uppercase md:text-xl">
                {collection.name}
              </h2>
              <p className="mt-2 max-w-md font-body text-sm text-ink/80">
                {collection.description}
              </p>
              <p className="mt-3 font-display text-[11px] tracking-[0.24em] uppercase text-mist">
                {collection.photoCount} photographs
              </p>
            </div>
          </Link>
        )
      })}
    </div>
  )
}
