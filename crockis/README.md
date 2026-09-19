# Crockis

Private photo library for sharing collections with friends. The product will eventually sit on a [rundtisch](../crates/rundtisch/README.md) backend; this folder currently holds the frontend only.

```
crockis/
├── web/              # Vite + React SPA
├── wrangler.jsonc    # Cloudflare Worker (static SPA, name: crockis)
└── package.json
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

## Deploy

The Cloudflare Worker is named `crockis` and serves `web/dist/` as a single-page app (no Rust/WASM worker). GitHub Actions workflow **Deploy Crockis** (`.github/workflows/deploy-crockis.yml`) is **manual only** (`workflow_dispatch`) — it does not run on pushes to `main` or on pull requests.

```bash
# local
npm install --prefix crockis
npm run deploy --prefix crockis

# or from the Actions tab: Run workflow → Deploy Crockis
```

See [web/README.md](web/README.md) for the SPA stack and API contract.
