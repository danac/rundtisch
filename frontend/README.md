# Rundtisch — Frontend

Small React SPA used to exercise the [Rust API](../backend/README.md) while it is being built. Deployed as static assets on the same Cloudflare Worker (`frontend/dist/`).

## Tech stack

| Component | Role |
|-----------|------|
| Vite 6 | Dev server, HMR, production build |
| React 19 + TypeScript | UI |
| Tailwind CSS v4 | Styling (`@tailwindcss/vite`) |

## Current state

Landing page only — wordmark **rundtisch** on a paper/ink canvas with concentric rings.

**Next:** an API playground (button list that calls `/api/*` and shows the JSON response).

## Commands

From the repository root (recommended — also starts the Wrangler API):

```bash
npm install                          # root: concurrently + wrangler
npm install --prefix frontend
npm run dev                          # Vite :5173 + Wrangler :8787
```

Frontend only:

```bash
npm install --prefix frontend
npm run dev --prefix frontend        # http://localhost:5173
npm run build --prefix frontend      # → frontend/dist/
npm run preview --prefix frontend    # serve the build on :4173
```

## API proxy

In local dev, Vite proxies `/api` to `http://localhost:8787` (see `vite.config.ts`). Same-origin requests like `fetch('/api/health')` reach the Rust worker without CORS setup.

## See also

- [Root README](../README.md) — monorepo architecture and combined `npm run dev`
- [Backend README](../backend/README.md) — Axum routes and WASM build
