# Crockis

Private photo library for sharing collections with friends. The product will eventually sit on a [rundtisch](../crates/rundtisch/README.md) backend; this folder currently holds the frontend only.

```
crockis/
├── web/              # Vite + React SPA
├── wasmer.toml       # Wasmer package (static-web-server + web/dist)
├── settings/         # static-web-server config (SPA 404 + cache headers)
├── app.yaml          # Wasmer Edge app (name: crockis)
├── wrangler.jsonc    # unused by CI (Cloudflare Worker config)
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

Wasmer Edge serves `web/dist/` as a static SPA (`wasmer.toml` + `app.yaml`, app name `crockis`). GitHub Actions workflow **Deploy Crockis on Wasmer Edge** (`.github/workflows/deploy-crockis.yml`) runs on pushes to `main`, pull requests to `main`, and manual `workflow_dispatch`. It uses the **Wasmer** GitHub environment (`WASMER_TOKEN` secret, `WASMER_OWNER` variable). There is no Rust/WASM build.

```bash
# local (Wasmer CLI + `wasmer login`; pass --owner if app.yaml has no owner)
npm run build --prefix crockis/web
npm run deploy --prefix crockis
```

See [web/README.md](web/README.md) for the SPA stack and API contract.
