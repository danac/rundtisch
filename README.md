# Rundtisch (WORK IN PROGRESS)

Micro web framework with CMS features.

The repository is a **Cargo workspace** plus a **demo website**: a small React SPA and a Rust (Axum) API. The reusable framework lives in `crates/rundtisch`. See the component READMEs for implementation detail:

| Document | Scope |
|----------|-------|
| [crates/rundtisch/README.md](crates/rundtisch/README.md) | Library crate — `AppState`, SeaORM auth |
| [demo/web/README.md](demo/web/README.md) | React SPA — landing page and auth panels |
| [demo/api/README.md](demo/api/README.md) | Demo Axum app — `/api/*` routes on top of `rundtisch` |

## Architecture

Application routes depend on `rundtisch` and a `sea_orm::DatabaseConnection`. The demo entry point opens that connection from `DATABASE_URL` (`sqlite://`, `mysql://`, or `postgres://`).

```
Browser (localhost:5173)
    │
    ├─ /, /assets/...  →  Vite dev server (HMR)
    │
    └─ /api/*          →  Vite proxy  →  native Axum (localhost:8787)
                                            └─ AppState { db }
```

`npm run dev --prefix demo` starts both processes. The dev script applies pending migrations to a local SQLite file, then starts the API. The server itself does not migrate on startup.

### Repository layout

```
.
├── Cargo.toml                 # workspace
├── crates/
│   └── rundtisch/             # lib (AppState, auth, SeaORM migrations)
└── demo/
    ├── api/                   # demo app crate: routes, native + migrate bins
    │   └── src/bin/native.rs  # listens on 0.0.0.0:8787
    ├── web/                   # React SPA
    └── package.json           # concurrently; `npm run dev`
```

## Local development

### First-time setup

```bash
npm install --prefix demo
npm install --prefix demo/web
```

### Start

```bash
npm run dev --prefix demo
```

| Process | Label | URL | Role |
|---------|-------|-----|------|
| Vite | `fe` | http://localhost:5173 | SPA with HMR — **open this in the browser** |
| API | `api` | http://localhost:8787 | Native Axum server |

Auth routes need these environment variables (the dev script sets only `DATABASE_URL`):

| Name | Length |
|------|--------|
| `AUTH_JWT_ACCESS_SECRET` | ≥ 32 bytes |
| `AUTH_JWT_VERIFY_SECRET` | ≥ 32 bytes |
| `AUTH_HASH_PEPPER` | exactly 32 bytes |

### Verify

```bash
curl http://localhost:8787/api/health
curl http://localhost:5173/api/health
```

### API and migrations without the frontend

```bash
export DATABASE_URL=sqlite://rundtisch.sqlite?mode=rwc
cargo run -p rundtisch-demo --bin migrate
cargo run -p rundtisch-demo --bin native
curl http://localhost:8787/api/health
```

## Build and test

```bash
cargo test
npm run build --prefix demo/web
```

CI (`.github/workflows/deploy.yml`) runs `cargo test` and the frontend build.

## Related documentation

- [crates/rundtisch/README.md](crates/rundtisch/README.md) — library API
- [demo/web/README.md](demo/web/README.md) — SPA and Vite proxy
- [demo/api/README.md](demo/api/README.md) — demo routes and the migrate binary
- [AGENTS.md](AGENTS.md) — Cursor Cloud agent environment notes
