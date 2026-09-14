# Rundtisch (WORK IN PROGRESS)

Platform-agnostic micro web framework with CMS features.

The repository is a **Cargo workspace** plus a **demo website** deployed as a **single Cloudflare Worker**: a small React SPA (landing page / API test harness) and a Rust (Axum) WASM worker for `/api/*`. The reusable framework lives in `crates/rundtisch` (to be published on crates.io). See the component READMEs for implementation detail:

| Document | Scope |
|----------|-------|
| [crates/rundtisch/README.md](crates/rundtisch/README.md) | Library crate — `Platform`, `AppState`, db, auth, native/Cloudflare runtimes |
| [demo/web/README.md](demo/web/README.md) | React SPA — landing page, planned API playground |
| [demo/api/README.md](demo/api/README.md) | Demo Axum app — a few `/api/*` routes on top of `rundtisch` |

## Architecture

### Crate split

Application code (routes and handlers) depends on `rundtisch` and stays the same for both deployments. Runtime crates call one helper:

| Target | Entry | Helper |
|--------|-------|--------|
| Native container | `demo/api/src/bin/native.rs` | `rundtisch::runtime::native::serve(router)` |
| Cloudflare Worker | `demo/api/worker` cdylib | `rundtisch::runtime::cloudflare::handle_fetch(req, env, build_router)` |

### Deployment model (demo)

```
                    Cloudflare Worker (rundtisch)
┌──────────────────────────────────────────────────────┐
│  /api/*  ──►  Rust Axum WASM worker  (demo/api/)     │
│  /*      ──►  Static SPA assets      (demo/web/dist/)│
└──────────────────────────────────────────────────────┘
```

| Path | Handler | Built from |
|------|---------|------------|
| `/api/*` | Rust Axum worker (WASM) | `demo/api/worker` via `worker-build` |
| `/*` | Static SPA + SPA fallback | `demo/web/dist/` |

Wrangler config at the repo root ties both together. The worker script (`demo/api/worker/build/index.js`) runs first for `/api/*`; all other requests are served from the Vite build output with `not_found_handling: "single-page-application"`.

The same demo API can run as a native binary (`cargo run -p rundtisch-demo --features native --bin native`) with no route/handler changes.

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
├── Cargo.toml                 # workspace
├── crates/
│   └── rundtisch/             # published lib (Platform, db, auth, runtime)
├── demo/
│   ├── api/                   # demo app crate: routes + handlers
│   │   ├── src/bin/native.rs  # native container entry
│   │   └── worker/            # cdylib for wrangler / worker-build
│   └── web/                   # React SPA
├── wrangler.jsonc             # demo Worker (assets + /api/*)
├── wrangler.dev.jsonc
└── package.json
```

## Design decisions

### Library vs demo

**Why split:** `rundtisch` is the reusable layer (platform trait, database adapters, auth, `serve` / `handle_fetch`). The demo is a small website used to debug that layer: a few Axum routes and the current landing page. Other projects can depend on the lib without taking demo routes.

**How:** A root Cargo workspace with `crates/rundtisch` (publishable) and `demo/api` + `demo/api/worker` (`publish = false`). Feature flags `native` and `cloudflare` stay on the lib; the demo crate forwards them.

### Single Worker, two artifacts

**Why:** One Cloudflare Worker keeps deployment, DNS, and preview URLs simple. Static assets and the API share the same origin, so the browser never needs CORS configuration once the frontend calls the API.

**How:** `wrangler.jsonc` sets `main` to the WASM worker shim and `assets.directory` to `demo/web/dist/`. `run_worker_first: ["/api/*"]` ensures API routes hit Rust before the asset handler.

### Frontend and API developed independently

The SPA and demo API are separate projects with their own READMEs, dependencies, and build steps. They only meet at deploy time (and in local dev via the Vite proxy). This keeps the React bundle free of Rust tooling and lets each side evolve on its own schedule.

### Frontend is a test harness

The demo API currently exposes `/api/health`. The SPA is a landing-page skeleton; a button list that calls `/api/*` and displays JSON responses is planned next. In local dev, Vite proxies `/api` to Wrangler so same-origin `fetch('/api/...')` works without CORS.

### Two Wrangler configs

| Config | WASM build | Used by |
|--------|------------|---------|
| `wrangler.jsonc` | `worker-build --release` | CI, manual production deploy |
| `wrangler.dev.jsonc` | `worker-build` (debug, faster) | `npm run dev`, local `wrangler dev` |

Release builds are slower but smaller and faster at runtime; debug builds shorten the edit-compile loop during API work.

### Root `package.json`

Cloud-agent and local dev environments run `npm install` at the repo root. The root `package.json` installs `concurrently` and `wrangler` and defines `npm run dev`. Frontend dependencies remain in `demo/web/package.json`.

## Local development

### First-time setup

```bash
npm install                          # root: concurrently + wrangler
npm install --prefix demo/web        # frontend dependencies
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
| Wrangler | `api` | http://localhost:8787 | Rust worker (rebuilds on crate/demo API changes) |

Stop both with **Ctrl+C**.

### Verify

```bash
curl http://localhost:8787/api/health   # direct to Wrangler → HTTP 200
curl http://localhost:5173/api/health   # via Vite proxy → HTTP 200
```

### Native API (no Wrangler)

```bash
cargo run -p rundtisch-demo --features native --bin native
curl http://localhost:8080/api/health
```

### Frontend-only or API-only

You can also run each side in a separate terminal — useful when working on only one stack:

```bash
# Terminal 1 — frontend
npm run dev --prefix demo/web

# Terminal 2 — Worker API (requires demo/web/dist/ for asset serving in wrangler dev)
npm run build --prefix demo/web
npx wrangler dev --config wrangler.dev.jsonc --port 8787
```

## Build

### Frontend

```bash
npm run build --prefix demo/web
```

Output: `demo/web/dist/` (TypeScript check + Vite production bundle).

### Library and demo API

```bash
cargo test -p rundtisch --features native
cargo check -p rundtisch-demo --features native
```

### Worker (WASM)

```bash
cd demo/api/worker
cargo install -q worker-build@^0.8
worker-build --release    # or omit --release for debug
```

Output: `demo/api/worker/build/index.js` + `index_bg.wasm` (gitignored; regenerated on every deploy).

Wrangler runs the Worker build automatically via `build.command` in `wrangler.jsonc` — you do not need a separate WASM build step before `wrangler deploy`.

## Deployment

### CI (GitHub Actions)

Workflow: `.github/workflows/deploy.yml`

| Trigger | Action |
|---------|--------|
| Push to `main` | Test lib → build frontend → `wrangler deploy` (production) |
| Pull request to `main` | Test lib → build frontend → preview alias `pr-<N>` |

CI steps: checkout → Node.js + Rust toolchains → Cargo cache → `cargo test -p rundtisch --features native` → `npm ci` + `npm run build` in `demo/web/` → Wrangler deploy (which compiles WASM via `build.command`).

Requires `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` in the GitHub **Cloudflare Workers** environment.

Preview URL format: `https://pr-<PR_NUMBER>-rundtisch.<account>.workers.dev`

### Manual deploy

```bash
npm run build --prefix demo/web
npx wrangler deploy
```

## Related documentation

- [crates/rundtisch/README.md](crates/rundtisch/README.md) — library API, features, native vs Cloudflare runtime
- [demo/web/README.md](demo/web/README.md) — SPA landing page, Vite proxy, planned API playground
- [demo/api/README.md](demo/api/README.md) — demo routes, WASM toolchain, extending the API
- [AGENTS.md](AGENTS.md) — Cursor Cloud agent environment notes
