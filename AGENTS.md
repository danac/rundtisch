# AGENTS.md

## Cursor Cloud specific instructions

This repo is an artist portfolio site (**anikaelsa**) deployed as a single Cloudflare Worker: a React 19 + Vite 6 + TypeScript + Tailwind CSS v4 SPA in `frontend/`, plus a Rust (Axum) WASM API in `backend/`. The frontend is not wired to the API yet; data still comes from bundled mocks in `frontend/src/data/`.

Standard commands:

- **Install:** `npm install` at the repo root (installs `concurrently` + `wrangler`); `npm install --prefix frontend` for frontend deps. The cloud-agent install script runs root `npm install` — a root `package.json` must exist.
- **Run (dev):** `npm run dev` from the repo root starts Vite on `http://localhost:5173` and Wrangler on `http://localhost:8787` (API proxied via Vite). Requires `rustup target add wasm32-unknown-unknown`; on Linux, `libssl-dev` and `pkg-config` for `worker-build`.
- **Build:** `npm run build --prefix frontend` runs `tsc -b` then `vite build` into `frontend/dist/`. `npm run preview --prefix frontend` serves the build on port 4173.
- **Lint is non-functional:** the `lint` script runs `eslint .`, but ESLint is not declared in `devDependencies` and no ESLint config exists, so `npm run lint` fails with `eslint: not found`. This is a pre-existing repo state, not an environment problem.
- **Optional API:** setting `VITE_API_BASE_URL` (see `frontend/.env.example`) switches the data layer in `frontend/src/services/` from local mocks to a REST API. Leave it unset to use bundled mock data.
- **Deploy:** Cloudflare Workers config is at the repo root (`wrangler.jsonc`, `wrangler.dev.jsonc`). Run `npm run build --prefix frontend` then `npx wrangler deploy` for a local production deploy.
- Static images served from `frontend/public/images/` are present and load correctly.
