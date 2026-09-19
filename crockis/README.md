# Crockis

Private photo library for sharing collections with friends. The product will eventually sit on a [rundtisch](../crates/rundtisch/README.md) backend; this folder currently holds the frontend only.

```
crockis/
└── web/          # Vite + React SPA
```

## Current state

Frontend-only. Collections, photos, and login are served from a typed API client that reads placeholder data today and can switch to REST (`GET /api/collections`, `GET /api/collections/:id/photos`, `POST /api/auth/login`) without changing the UI.

## Commands

```bash
npm install --prefix crockis/web
npm run dev --prefix crockis          # or: npm run dev --prefix crockis/web
npm run build --prefix crockis
```

Dev server: http://localhost:5174 (5173 is reserved for the rundtisch demo).

See [web/README.md](web/README.md) for the SPA stack and API contract.
