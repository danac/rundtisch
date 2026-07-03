# anikaelsa

Artist portfolio site for **Anika** — watercolor birds, hand lettering, and illustrations for children.

The repository is a **monorepo** deployed as a **single Cloudflare Worker**: a React SPA for the public site and a Rust (Axum) WASM worker for `/api/*`. See the component READMEs for implementation detail:

| Document | Scope |
|----------|-------|
| [frontend/README.md](frontend/README.md) | React SPA — pages, UI, data layer, i18n |
| [backend/README.md](backend/README.md) | Rust API — routes, WASM build, worker integration |

## Architecture

### Deployment model

```
                    Cloudflare Worker (anikaelsa)
┌──────────────────────────────────────────────────────┐
│  /api/*  ──►  Rust Axum WASM worker  (backend/)      │
│  /*      ──►  Static SPA assets      (frontend/dist/) │
└──────────────────────────────────────────────────────┘
```

| Path | Handler | Built from |
|------|---------|------------|
| `/api/*` | Rust Axum worker (WASM) | `backend/` via `worker-build` |
| `/*` | Static SPA + SPA fallback | `frontend/dist/` |

Wrangler config at the repo root ties both together. The worker script (`backend/build/index.js`) runs first for `/api/*`; all other requests are served from the Vite build output with `not_found_handling: "single-page-application"`.

### Development model

In local dev the browser talks only to Vite; API calls are proxied to Wrangler:

```
Browser (localhost:5173)
    │
    ├─ /, /assets/...  →  Vite dev server (HMR)
    │
    └─ /api/*          →  Vite proxy  →  wrangler dev (localhost:8787)
                                            └─ Rust Axum worker
```

Root `npm run dev` starts both processes via `concurrently`. See [Local development](#local-development).

### Repository layout

```
.
├── frontend/              # Vite + React 19 SPA
├── backend/               # Rust Axum API (WASM)
├── wrangler.jsonc         # Production Worker config (release WASM build)
├── wrangler.dev.jsonc     # Local dev Worker config (debug WASM build)
├── package.json           # Root dev script (concurrently + wrangler)
├── .github/workflows/     # CI: build frontend, compile WASM, deploy
└── README.md              # This file
```

## Design decisions

### Single Worker, two artifacts

**Why:** One Cloudflare Worker keeps deployment, DNS, and preview URLs simple. Static assets and the API share the same origin, so the browser never needs CORS configuration once the frontend calls the API.

**How:** `wrangler.jsonc` sets `main` to the WASM worker shim and `assets.directory` to `frontend/dist/`. `run_worker_first: ["/api/*"]` ensures API routes hit Rust before the asset handler.

### Frontend and backend developed independently

The SPA and API are separate crates/projects with their own READMEs, dependencies, and build steps. They only meet at deploy time (and in local dev via the Vite proxy). This keeps the React bundle free of Rust tooling and lets each side evolve on its own schedule.

### API not wired yet

The backend currently exposes only `/api/health`. The frontend still reads bundled mocks from `frontend/src/data/`. The data layer in `frontend/src/services/` is already structured for a REST backend via `VITE_API_BASE_URL` — see [frontend/README.md — Connecting a backend](frontend/README.md#connecting-a-backend).

When ready, set `VITE_API_BASE_URL=/api` at frontend build time so requests go to same-origin paths like `/api/homepage`, then implement matching routes in `backend/src/lib.rs`.

### Two Wrangler configs

| Config | WASM build | Used by |
|--------|------------|---------|
| `wrangler.jsonc` | `worker-build --release` | CI, manual production deploy |
| `wrangler.dev.jsonc` | `worker-build` (debug, faster) | `npm run dev`, local `wrangler dev` |

Release builds are slower but smaller and faster at runtime; debug builds shorten the edit-compile loop during backend work.

### Root `package.json`

Cloud-agent and local dev environments run `npm install` at the repo root. The root `package.json` installs `concurrently` and `wrangler` and defines `npm run dev`. Frontend dependencies remain in `frontend/package.json`.

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

### Start

From the repository root:

```bash
npm run dev
```

| Process | Label | URL | Role |
|---------|-------|-----|------|
| Vite | `fe` | http://localhost:5173 | SPA with HMR — **open this in the browser** |
| Wrangler | `api` | http://localhost:8787 | Rust worker (rebuilds on `backend/` changes) |

Stop both with **Ctrl+C**.

### Verify

```bash
curl http://localhost:8787/api/health   # direct to Wrangler → HTTP 200
curl http://localhost:5173/api/health   # via Vite proxy → HTTP 200
```

### Frontend-only or backend-only

You can also run each side in a separate terminal — useful when working on only one stack:

```bash
# Terminal 1 — frontend
npm run dev --prefix frontend

# Terminal 2 — backend (requires frontend/dist/ for asset serving in wrangler dev)
npm run build --prefix frontend
npx wrangler dev --config wrangler.dev.jsonc --port 8787
```

## Build

### Frontend

```bash
npm run build --prefix frontend
```

Output: `frontend/dist/` (TypeScript check + Vite production bundle).

### Backend

```bash
cd backend
cargo install -q worker-build@^0.8
worker-build --release    # or omit --release for debug
```

Output: `backend/build/index.js` + `index_bg.wasm` (gitignored; regenerated on every deploy).

Wrangler runs the backend build automatically via `build.command` in `wrangler.jsonc` — you do not need a separate backend build step before `wrangler deploy`.

## Deployment

### CI (GitHub Actions)

Workflow: `.github/workflows/deploy.yml`

| Trigger | Action |
|---------|--------|
| Push to `master` | Build frontend → `wrangler deploy` (production) |
| Pull request to `master` | Build frontend → preview alias `pr-<N>` |

CI steps: checkout → Node.js + Rust toolchains → Cargo cache → `npm ci` + `npm run build` in `frontend/` → Wrangler deploy (which compiles WASM via `build.command`).

Requires `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` in the GitHub **Cloudflare Workers** environment.

Preview URL format: `https://pr-<PR_NUMBER>-anikaelsa.<account>.workers.dev`

### Manual deploy

```bash
npm run build --prefix frontend
npx wrangler deploy
```

Equivalent to `npm run deploy` from `frontend/` (which builds then calls `wrangler deploy --config ../wrangler.jsonc`).

## Related documentation

- [frontend/README.md](frontend/README.md) — SPA structure, pages, services layer, i18n, async images
- [backend/README.md](backend/README.md) — Axum routes, WASM toolchain, extending the API
- [AGENTS.md](AGENTS.md) — Cursor Cloud agent environment notes
