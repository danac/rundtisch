import JSZip from 'jszip'
import type { Collection, Photo } from './api'

function slug(value: string) {
  const cleaned = value
    .normalize('NFKD')
    .replace(/[^\w\s.-]+/g, '')
    .trim()
    .replace(/\s+/g, '-')
    .toLowerCase()
  return cleaned || 'photo'
}

function extension(blob: Blob, url: string) {
  const known: Record<string, string> = {
    'image/jpeg': 'jpg',
    'image/png': 'png',
    'image/webp': 'webp',
    'image/avif': 'avif',
    'image/gif': 'gif',
    'image/heic': 'heic',
  }
  if (known[blob.type]) return known[blob.type]
  const match = (url.split('?')[0] ?? '').match(/\.([a-z0-9]+)$/i)
  return match?.[1]?.toLowerCase() ?? 'jpg'
}

function sameOrigin(url: string) {
  if (url.startsWith('/')) return true
  try {
    return new URL(url, window.location.origin).origin === window.location.origin
  } catch {
    return false
  }
}

async function fetchAsset(url: string, token: string | null) {
  const headers = new Headers()
  if (token && sameOrigin(url)) headers.set('Authorization', `Bearer ${token}`)
  const response = await fetch(url, { headers })
  if (!response.ok) throw new Error('Unable to download this file.')
  return response.blob()
}

function saveBlob(blob: Blob, filename: string) {
  const href = URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = href
  link.download = filename
  link.click()
  setTimeout(() => URL.revokeObjectURL(href), 30_000)
}

function photoUrl(photo: Photo) {
  return photo.downloadSrc ?? photo.src
}

function photoName(photo: Photo, blob: Blob) {
  if (photo.filename) return photo.filename
  return `${slug(photo.title ?? photo.id)}.${extension(blob, photoUrl(photo))}`
}

export async function downloadPhoto(photo: Photo, token: string | null) {
  const url = photoUrl(photo)
  const blob = await fetchAsset(url, token)
  saveBlob(blob, photoName(photo, blob))
}

export async function downloadCollection(
  collection: Collection,
  photos: Photo[],
  token: string | null,
) {
  const zip = new JSZip()
  const used = new Set<string>()
  const queue = photos.map((photo, index) => ({ photo, index }))
  const workers = Array.from({ length: Math.min(4, queue.length) }, async () => {
    while (queue.length > 0) {
      const next = queue.shift()
      if (!next) return
      const blob = await fetchAsset(photoUrl(next.photo), token)
      let name = `${String(next.index + 1).padStart(2, '0')}-${photoName(next.photo, blob)}`
      while (used.has(name)) name = `copy-${name}`
      used.add(name)
      zip.file(name, blob)
    }
  })
  await Promise.all(workers)
  saveBlob(await zip.generateAsync({ type: 'blob' }), `${slug(collection.name)}.zip`)
}
