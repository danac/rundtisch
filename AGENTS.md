# AGENTS.md

## Cursor Cloud specific instructions

This repo is **rundtisch** — a monorepo with a reusable Rust lib crate (`crates/rundtisch`) and a demo website: React 19 + Vite + TypeScript + Tailwind CSS v4 SPA in `demo/web/`, plus a small Axum API in `demo/api/`. Persistence is SeaORM on a `DatabaseConnection`. The process entry point opens `DATABASE_URL` (sqlite, mysql, or postgres). Schema changes are applied by `cargo run -p rundtisch-demo --bin migrate`, not on server startup.

Standard commands:

- **Install:** `npm install --prefix demo` (installs `concurrently`); `npm install --prefix demo/web` for frontend deps.
- **Run (dev):** `npm run dev --prefix demo` starts Vite on `http://localhost:5173` and the native API on `http://localhost:8787` (API proxied via Vite). The script migrates a local SQLite file first. Auth routes also need `AUTH_JWT_ACCESS_SECRET`, `AUTH_JWT_VERIFY_SECRET` (each at least 32 bytes), and `AUTH_HASH_PEPPER` (exactly 32 bytes).
- **Build:** `npm run build --prefix demo/web` runs `tsc -b` then `vite build` into `demo/web/dist/`. `npm run preview --prefix demo/web` serves the build on port 4173.
- **API:** `DATABASE_URL=sqlite://rundtisch.sqlite?mode=rwc cargo run -p rundtisch-demo --bin native` serves the demo API on `http://localhost:8787`.
- **Migrate:** `DATABASE_URL=... cargo run -p rundtisch-demo --bin migrate`.
- **Wasmer:** `demo/wasmer.toml` and `demo/app.yaml` package the WASIX binaries. `[fs]` mounts `demo/web/dist` at `/app/web`; the native binary serves that SPA when the directory exists. Leave `STATIC_DIR` unset so Vite split-dev still owns the frontend. `.github/workflows/ci.yml` runs on pushes to `main` and `cursor/wasmer-edge-demo-6266`. It builds a WASIX release (after swapping in `Cargo.wasix.lock` and building the frontend) and deploys from `demo/` using the `Wasmer` GitHub environment (`WASMER_TOKEN`, `WASMER_OWNER`).
- **Test:** `cargo test` from the repository root.
