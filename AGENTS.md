# AGENTS.md

## Cursor Cloud specific instructions

This repo is **rundtisch** — a monorepo deployed as a single Cloudflare Worker: a React 19 + Vite 6 + TypeScript + Tailwind CSS v4 SPA in `frontend/`, plus a Rust (Axum) WASM API in `backend/`. The frontend is currently a landing-page skeleton; an API playground (buttons that call `/api/*` and show responses) is planned next.

Standard commands:

- **Install:** `npm install` at the repo root (installs `concurrently` + `wrangler`); `npm install --prefix frontend` for frontend deps. The cloud-agent install script runs root `npm install` — a root `package.json` must exist.
- **Run (dev):** `npm run dev` from the repo root starts Vite on `http://localhost:5173` and Wrangler on `http://localhost:8787` (API proxied via Vite). Requires `rustup target add wasm32-unknown-unknown`; on Linux, `libssl-dev` and `pkg-config` for `worker-build`.
- **Build:** `npm run build --prefix frontend` runs `tsc -b` then `vite build` into `frontend/dist/`. `npm run preview --prefix frontend` serves the build on port 4173.
- **Deploy:** Cloudflare Workers config is at the repo root (`wrangler.jsonc`, `wrangler.dev.jsonc`). Run `npm run build --prefix frontend` then `npx wrangler deploy` for a local production deploy.
