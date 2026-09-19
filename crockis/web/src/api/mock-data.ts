import type { Collection, Photo, User } from './types'

function unsplash(photoId: string, width: number, height: number): string {
  return `https://images.unsplash.com/${photoId}?auto=format&fit=crop&w=${width}&h=${height}&q=80`
}

function pexels(id: string, file: string, width: number): string {
  return `https://images.pexels.com/photos/${id}/${file}?auto=compress&cs=tinysrgb&w=${width}`
}

function photo(
  collectionId: string,
  index: number,
  src: string,
  width: number,
  height: number,
  alt: string,
  title: string,
): Photo {
  return {
    id: `${collectionId}-${String(index).padStart(2, '0')}`,
    collectionId,
    src,
    width,
    height,
    alt,
    title,
    takenAt: '2026-03-12T10:00:00.000Z',
  }
}

const serenityPhotos: Photo[] = [
  photo('serenity-spa', 1, unsplash('photo-1600334129128-685c5582fd35', 900, 900), 900, 900, 'Hot stones along a guest’s back', 'Stones'),
  photo('serenity-spa', 2, unsplash('photo-1560750588-73207b1ef5b8', 1600, 900), 1600, 900, 'Private lounge beside an indoor pool', 'Pavilion'),
  photo('serenity-spa', 3, unsplash('photo-1544161515-4ab6ce6db874', 1600, 900), 1600, 900, 'Oil poured during a massage', 'Touch'),
  photo('serenity-spa', 4, pexels('3188', 'love-romantic-bath-candlelight.jpg', 900), 900, 900, 'Rolled towel with candles and a tulip', 'Ritual'),
  photo('serenity-spa', 5, unsplash('photo-1515377905703-c4788e51af15', 1400, 900), 1400, 900, 'Drop of essential oil', 'Oil'),
  photo('serenity-spa', 6, unsplash('photo-1418065460487-3e41a6c84dc5', 1600, 1000), 1600, 1000, 'Fog over a pine forest', 'Grove'),
  photo('serenity-spa', 7, unsplash('photo-1583416750470-965b2707b355', 1000, 900), 1000, 900, 'Cedar sauna in low light', 'Heat'),
  photo('serenity-spa', 8, unsplash('photo-1519824145371-296894a0daa9', 900, 1100), 900, 1100, 'Hands during a massage', 'Hands'),
  photo('serenity-spa', 9, unsplash('photo-1507652313519-d4e9174996dd', 1600, 900), 1600, 900, 'Freestanding bath in a quiet room', 'Bath'),
  photo('serenity-spa', 10, pexels('3764568', 'pexels-photo-3764568.jpeg', 1000), 1000, 900, 'Neck and shoulder treatment', 'Quiet'),
  photo('serenity-spa', 11, unsplash('photo-1441974231531-c6227db76b6e', 1100, 900), 1100, 900, 'Path through a green forest', 'Walk'),
  photo('serenity-spa', 12, pexels('3865530', 'pexels-photo-3865530.jpeg', 900), 900, 1100, 'Close massage on the collarbone', 'Soft'),
  photo('serenity-spa', 13, unsplash('photo-1570172619644-dfd03ed5d881', 1600, 900), 1600, 900, 'Facial mask applied with a brush', 'Mask'),
  photo('serenity-spa', 14, pexels('3757942', 'pexels-photo-3757942.jpeg', 1000), 1000, 900, 'Guest resting after treatment', 'Rest'),
  photo('serenity-spa', 15, pexels('3865676', 'pexels-photo-3865676.jpeg', 1400), 1400, 900, 'Oils, flowers, and stones', 'Blend'),
  photo('serenity-spa', 16, unsplash('photo-1519823551278-64ac92734fb1', 900, 1100), 900, 1100, 'Therapist’s hands on the back', 'Close'),
]

const alpsPhotos: Photo[] = [
  photo('alpine-light', 1, unsplash('photo-1464822759023-fed622ff2c3b', 1600, 1000), 1600, 1000, 'Sunlit alpine ridge', 'Ridge'),
  photo('alpine-light', 2, unsplash('photo-1483728642387-6c3bdd6c93e5', 1200, 1600), 1200, 1600, 'Snow peak against a clear sky', 'Summit'),
  photo('alpine-light', 3, unsplash('photo-1469474968028-56623f02e42e', 1600, 900), 1600, 900, 'Valley at sunrise', 'Valley'),
  photo('alpine-light', 4, unsplash('photo-1470071459604-3b5ec3a7fe05', 1400, 900), 1400, 900, 'Fog over mountain pines', 'Fog'),
  photo('alpine-light', 5, unsplash('photo-1501785888041-af3ef285b470', 900, 1200), 900, 1200, 'Lake below the peaks', 'Lake'),
  photo('alpine-light', 6, unsplash('photo-1519681393784-d120267933ba', 1600, 900), 1600, 900, 'Night sky over a mountain range', 'Night'),
]

const gardenPhotos: Photo[] = [
  photo('summer-garden', 1, unsplash('photo-1466781783364-36c955e42a7f', 1400, 900), 1400, 900, 'Lush garden path', 'Path'),
  photo('summer-garden', 2, unsplash('photo-1490750967868-88aa4486c946', 900, 1200), 900, 1200, 'Close-up of spring flowers', 'Bloom'),
  photo('summer-garden', 3, unsplash('photo-1501004318641-b39e6451bec6', 1200, 900), 1200, 900, 'Greenhouse foliage', 'Glass'),
  photo('summer-garden', 4, unsplash('photo-1441974231531-c6227db76b6e', 1000, 900), 1000, 900, 'Woodland garden walk', 'Wood'),
  photo('summer-garden', 5, unsplash('photo-1418065460487-3e41a6c84dc5', 1600, 900), 1600, 900, 'Mist in the trees', 'Mist'),
]

const coastPhotos: Photo[] = [
  photo('north-coast', 1, unsplash('photo-1507525428034-b723cf961d3e', 1600, 900), 1600, 900, 'Turquoise shoreline', 'Shore'),
  photo('north-coast', 2, unsplash('photo-1476673160081-cf065307f649', 1200, 1600), 1200, 1600, 'Cliff above the sea', 'Cliff'),
  photo('north-coast', 3, unsplash('photo-1505118380757-91f5f5632de0', 1600, 1000), 1600, 1000, 'Waves from above', 'Tide'),
  photo('north-coast', 4, unsplash('photo-1439066615861-d1af74d74000', 1400, 900), 1400, 900, 'Calm mountain lake', 'Still'),
  photo('north-coast', 5, unsplash('photo-1475924156734-496f6cac6ec1', 900, 1100), 900, 1100, 'Evening surf', 'Surf'),
  photo('north-coast', 6, unsplash('photo-1507525428034-b723cf961d3e', 1600, 900), 1600, 900, 'Warm sand and sea', 'Bay'),
  photo('north-coast', 7, unsplash('photo-1476673160081-cf065307f649', 1000, 900), 1000, 900, 'Open water', 'Horizon'),
]

export const mockPhotosByCollection: Record<string, Photo[]> = {
  'serenity-spa': serenityPhotos,
  'alpine-light': alpsPhotos,
  'summer-garden': gardenPhotos,
  'north-coast': coastPhotos,
}

export const mockCollections: Collection[] = [
  {
    id: 'serenity-spa',
    name: 'Serenity Spa',
    description: 'A quiet weekend of water, heat, and low light.',
    cover: serenityPhotos[6]!,
    photoCount: serenityPhotos.length,
  },
  {
    id: 'alpine-light',
    name: 'Alpine Light',
    description: 'High passes and still lakes after the thaw.',
    cover: alpsPhotos[0]!,
    photoCount: alpsPhotos.length,
  },
  {
    id: 'summer-garden',
    name: 'Summer Garden',
    description: 'Rain, glasshouses, and late flowers.',
    cover: gardenPhotos[0]!,
    photoCount: gardenPhotos.length,
  },
  {
    id: 'north-coast',
    name: 'North Coast',
    description: 'Tide lines and long evenings by the water.',
    cover: coastPhotos[0]!,
    photoCount: coastPhotos.length,
  },
]

export const mockUser: User = {
  id: 'user-dana',
  email: 'dana@crockis.test',
  name: 'Dana',
}
