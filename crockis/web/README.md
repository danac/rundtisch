# Crockis — frontend

React SPA for a private photo library. Sign-in, collections, and photos use the [Crockis API](../api/README.md).

## Tech stack

| Component | Role |
|-----------|------|
| Vite 8 | Dev server, HMR, production build |
| React 19 + TypeScript | UI |
| React Router 7 | Login, library, settings, and CLI authorize routes |
| TanStack Query | Cache for collection / photo fetches |
| Tailwind CSS v4 | Styling (`@tailwindcss/vite`) |
| react-photo-album | Justified photo mosaic |
| yet-another-react-lightbox | Full-size viewer (download + pinch zoom) |

## Pages

| Route | Purpose |
|-------|---------|
| `/login` | Password or passkey sign-in, invitation registration, account recovery |
| `/authorize` | Let a signed-in user approve a command-line session |
| `/authorize/done` | Result of that approval |
| `/collections` | Tile list of collections |
| `/collections/:collectionId` | Photo mosaic for one collection |
| `/settings` | Alias, password, passkeys, and sessions |

`/?invite=` and `/?recover=` stay on the login screen. Unauthenticated visits to the library redirect to `/login`.

## Data layer

Auth calls `/api/auth/*` with `credentials: 'include'` (cookie session). Collection and photo reads use the same cookie against `/api/collections` and `/api/photos/{id}/file`.

## Commands

From the repository root (also starts the API):

```bash
npm install --prefix crockis
npm install --prefix crockis/web
npm run dev --prefix crockis          # Vite :5174 + API :8788
```

Frontend only:

```bash
npm install --prefix crockis/web
npm run dev --prefix crockis/web      # http://localhost:5174
npm run build --prefix crockis/web    # → crockis/web/dist/
npm run preview --prefix crockis/web
```

## API proxy

Vite proxies `/api` to `http://localhost:8788` (`vite.config.ts`). Auth cookies need `credentials: 'include'`.

## See also

- [Crockis API](../api/README.md)
- [Root README](../../README.md)
