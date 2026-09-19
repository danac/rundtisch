# AGENTS.md

## Cursor Cloud specific instructions

This repo is **rundtisch** — a monorepo with a reusable Rust lib crate (`crates/rundtisch`), a demo website (React 19 + Vite + TypeScript + Tailwind CSS v4 SPA in `demo/web/`, plus a small Axum API in `demo/api/` compiled to WASM and deployed as a single Cloudflare Worker), and **Crockis**, a photo-library frontend in `crockis/web/` that will later use the same crate. The demo frontend is currently a landing-page skeleton; an API playground (buttons that call `/api/*` and show responses) is planned next.

Standard commands:

- **Install:** `npm install --prefix demo` (installs `concurrently` + `wrangler`); `npm install --prefix demo/web` for the demo frontend; `npm install --prefix crockis/web` for the Crockis photo SPA. The cloud-agent install script should run `npm install --prefix demo`.
- **Run (dev):** `npm run dev --prefix demo` starts Vite on `http://localhost:5173` and Wrangler on `http://localhost:8787` (API proxied via Vite). `predev` creates `demo/web/dist` so Wrangler can start without a prior frontend build. Requires `rustup target add wasm32-unknown-unknown`; on Linux, `libssl-dev` and `pkg-config` for `worker-build`.
- **Build:** `npm run build --prefix demo/web` runs `tsc -b` then `vite build` into `demo/web/dist/`. `npm run preview --prefix demo/web` serves the build on port 4173. Crockis: `npm run dev --prefix crockis/web` on `http://localhost:5174`; `npm run build --prefix crockis/web`.
- **Native API:** `cargo run -p rundtisch-demo --features native --bin native` serves the demo API on `http://localhost:8080`.
- **Deploy (demo):** Cloudflare Workers config lives in `demo/` (`wrangler.jsonc`, `wrangler.dev.jsonc`). Run `npm run build --prefix demo/web` then `npx wrangler deploy --config demo/wrangler.jsonc`. Workflow `.github/workflows/deploy.yml` runs on `main` and pull requests.
- **Deploy (Crockis):** Assets-only Worker `crockis` (`crockis/wrangler.jsonc`). Build with `npm run build --prefix crockis/web`, then `npx wrangler deploy --config crockis/wrangler.jsonc`. Workflow `.github/workflows/deploy-crockis.yml` runs on `main`, pull requests (preview alias `pr-<N>`), and `workflow_dispatch`. No Rust/WASM step.
