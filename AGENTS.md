# AGENTS.md

## Cursor Cloud specific instructions

This repo is **rundtisch** — a monorepo with a reusable Rust lib crate (`crates/rundtisch`) and a demo website: React 19 + Vite + TypeScript + Tailwind CSS v4 SPA in `demo/web/`, plus a small Axum API in `demo/api/`. Persistence is SeaORM on a `DatabaseConnection`. The process entry point opens `DATABASE_URL` (sqlite, mysql, or postgres). Schema changes are applied by `cargo run -p rundtisch-demo --bin migrate` locally, and by a Wasmer Edge `pre-deployment` job that runs the same `migrate` command once per deploy — not on server startup or per request.

Standard commands:

- **Install:** `npm install --prefix demo` (installs `concurrently`); `npm install --prefix demo/web` for frontend deps.
- **Run (dev):** `npm run dev --prefix demo` starts Vite on `http://localhost:5173` and the native API on `http://localhost:8787` (API proxied via Vite). The script migrates a local SQLite file first. Auth routes need `AUTH_HASH_PEPPER` (exactly 32 bytes), which the dev script sets. WebAuthn defaults are RP id `localhost`, origin `http://localhost:5173`, and name `rundtisch` (`AUTH_WEBAUTHN_RP_ID`, `AUTH_WEBAUTHN_RP_ORIGIN`, `AUTH_WEBAUTHN_RP_NAME`). Mint invitation and recovery links with `cargo run -p rundtisch-demo --bin auth-link -- invite|recover`.
- **Build:** `npm run build --prefix demo/web` runs `tsc -b` then `vite build` into `demo/web/dist/`. `npm run preview --prefix demo/web` serves the build on port 4173.
- **API:** `DATABASE_URL=sqlite://rundtisch.sqlite?mode=rwc cargo run -p rundtisch-demo --bin native` serves the demo API on `http://localhost:8787`.
- **Migrate:** `DATABASE_URL=... cargo run -p rundtisch-demo --bin migrate`. Optional `RUNDTISCH_BOOTSTRAP_ADMIN_EMAIL` / `RUNDTISCH_BOOTSTRAP_ADMIN_PASSWORD` upsert a verified Admin after migrations (insert if the email is absent, otherwise reset the password and keep the same user).
- **Wasmer:** `demo/wasmer.toml` and `demo/app.yaml` package the WASIX binaries. `[fs]` mounts `demo/web/dist` at `/app/web`; the native binary serves that SPA when the directory exists. Leave `STATIC_DIR` unset so Vite split-dev still owns the frontend. `.github/workflows/ci.yml` runs on pushes to `main` and `cursor/wasmer-edge-demo-6266`. It builds a WASIX release (after swapping in `Cargo.wasix.lock` and building the frontend) and deploys from `demo/` using the `Wasmer` GitHub environment (`WASMER_TOKEN`, `WASMER_OWNER`). A commented "Ensure auth secrets" step in that workflow can create missing `AUTH_*` app secrets; leave it commented unless a new Edge app has no secrets yet.
- **Test:** `cargo test` from the repository root.
