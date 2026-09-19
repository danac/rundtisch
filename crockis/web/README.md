# Crockis — frontend

React SPA for a private photo library. Collections and image metadata are loaded through a typed client so a rundtisch REST API can replace the placeholders later.

## Tech stack

| Component | Role |
|-----------|------|
| Vite 8 | Dev server, HMR, production build |
| React 19 + TypeScript | UI |
| React Router 7 | Collections, collection, and login routes |
| TanStack Query | Cache and loaders for collection / photo fetches |
| Tailwind CSS v4 | Styling (`@tailwindcss/vite`) |
| react-photo-album | Justified photo mosaic |
| yet-another-react-lightbox | Full-size viewer |

## Pages

| Route | Purpose |
|-------|---------|
| `/login` | Email / password sign-in |
| `/collections` | Tile list of collections |
| `/collections/:collectionId` | Photo mosaic for one collection |

Unauthenticated visits to collection routes redirect to `/login`.

## Data layer

UI code talks only to `src/api` (`CrockisApi`). Two implementations:

- **Mock** (`src/api/mock.ts`, default) — in-memory collections and Unsplash placeholders
- **REST** (`src/api/rest.ts`) — same methods against `/api`

Switch with `VITE_USE_MOCK=false` (see `.env.example`).

Expected REST shape:

```
POST /api/auth/login          { email, password } → { user, token }
POST /api/auth/logout
GET  /api/auth/me             → user
GET  /api/collections         → Collection[]
GET  /api/collections/:id     → Collection
GET  /api/collections/:id/photos → Photo[]
```

`Photo` includes `id`, `src`, `width`, `height`, and optional `title` / `takenAt` so a future backend can return CDN URLs and EXIF-style metadata without a UI rewrite.

## Commands

```bash
npm install --prefix crockis/web
npm run dev --prefix crockis/web        # http://localhost:5174
npm run build --prefix crockis/web      # → crockis/web/dist/
npm run preview --prefix crockis/web
```

In local dev, Vite proxies `/api` to `http://localhost:8787` (same convention as the rundtisch demo).
