# anikaelsa

Artist portfolio site for **Anika** — watercolor birds, hand lettering, and illustrations for children. Deployed as a single Cloudflare Worker serving a React SPA and a Rust (Axum) API.

## Architecture

### Deployment model

Everything runs as **one Cloudflare Worker**:

```
                    Cloudflare Worker
┌──────────────────────────────────────────────────────┐
│  /api/*  ──►  Rust Axum WASM worker  (backend/)      │
│  /*      ──►  Static SPA assets      (frontend/dist/) │
└──────────────────────────────────────────────────────┘
```

| Path | Handler | Source |
|------|---------|--------|
| `/api/*` | Rust Axum worker (WASM) | `backend/` |
| `/*` | Static SPA assets | `frontend/dist/` |

### Repository layout

| Path | Purpose |
|------|---------|
| `frontend/` | Vite + React 19 SPA (TypeScript, Tailwind CSS v4) |
| `backend/` | Rust Axum API compiled to WASM for Cloudflare Workers |
| `wrangler.jsonc` | Production Worker config (release WASM build) |
| `wrangler.dev.jsonc` | Local dev Worker config (debug WASM build) |
| `package.json` | Root dev script (`npm run dev` runs Vite + Wrangler) |
| `.github/workflows/deploy.yml` | CI: build frontend + deploy Worker on push/PR |

### Frontend

- **Stack:** React 19, Vite 6, TypeScript, Tailwind CSS v4, react-router-dom, react-i18next
- **Key dirs:** `frontend/src/components/`, `pages/`, `services/`, `data/`, `i18n/`
- **Data:** Bundled mocks in `frontend/src/data/`; optional REST via `VITE_API_BASE_URL` (see `frontend/.env.example`)
- **Build output:** `frontend/dist/`

The frontend is **not wired to the backend yet** — services still use local mock data. API paths are relative (`/api/...`), so no CORS is needed once connected.

### Backend

- **Stack:** Rust, [Axum](https://github.com/tokio-rs/axum), [workers-rs](https://github.com/cloudflare/workers-rs), [worker-build](https://github.com/cloudflare/workers-rs/tree/main/crates/worker-build)
- **Entry:** `backend/src/lib.rs` — Axum router with `/api/health`
- **Build output:** `backend/build/index.js` + `index_bg.wasm` (generated, not committed)

### Production vs development

**Production:** Browser → Cloudflare Worker → `/api/*` to WASM, everything else to static assets.

**Development:**

```
Browser (localhost:5173)
    │
    ├─ /, /assets/...  →  Vite dev server (HMR)
    │
    └─ /api/*          →  Vite proxy  →  wrangler dev (localhost:8787)
                                            └─ Rust Axum worker
```

## Local development

### First-time setup

```bash
npm install                          # root: concurrently + wrangler
npm install --prefix frontend        # frontend dependencies
rustup target add wasm32-unknown-unknown
```

On Linux, if `worker-build` fails with OpenSSL errors:

```bash
sudo apt-get install libssl-dev pkg-config
```

### Start dev servers

From the repository root:

```bash
npm run dev
```

This starts:

- **Vite** (`fe`) on http://localhost:5173 — SPA with hot module replacement
- **Wrangler** (`api`) on http://localhost:8787 — Rust worker with live rebuild

Open http://localhost:5173 in your browser. API requests to `/api/*` are proxied to Wrangler automatically.

Stop both with **Ctrl+C**.

### Verify

```bash
curl http://localhost:8787/api/health   # direct to Wrangler → HTTP 200
curl http://localhost:5173/api/health   # via Vite proxy → HTTP 200
```

### Wrangler configs

| Config | Build | Used by |
|--------|-------|---------|
| `wrangler.jsonc` | `worker-build --release` | CI, manual production deploy |
| `wrangler.dev.jsonc` | `worker-build` (debug) | `npm run dev`, local `wrangler dev` |

### Two-terminal alternative

```bash
# Terminal 1
npm run dev --prefix frontend

# Terminal 2
npx wrangler dev --config wrangler.dev.jsonc --port 8787
```

## Deployment

### CI

- **Push to `master`:** builds frontend, compiles WASM, deploys to production
- **Pull requests:** preview deploy with alias `pr-<PR_NUMBER>` (e.g. `https://pr-42-anikaelsa.<account>.workers.dev`)

Requires `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` in the GitHub **Cloudflare Workers** environment.

### Manual deploy

```bash
npm run build --prefix frontend
npx wrangler deploy
```

Wrangler runs the `build.command` from `wrangler.jsonc` to compile the Rust backend before upload.
