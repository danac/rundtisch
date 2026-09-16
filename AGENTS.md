# AGENTS.md

## Cursor Cloud specific instructions

This repo is **rundtisch** — a monorepo with a reusable Rust lib crate (`crates/rundtisch`) and a demo website: React 19 + Vite 6 + TypeScript + Tailwind CSS v4 SPA in `demo/web/`, plus a small Axum API in `demo/api/` compiled to WASM and deployed as a single Cloudflare Worker. The frontend is currently a landing-page skeleton; an API playground (buttons that call `/api/*` and show responses) is planned next.

Standard commands:

- **Install:** `npm install --prefix demo` (installs `concurrently` + `wrangler`); `npm install --prefix demo/web` for frontend deps. The cloud-agent install script should run `npm install --prefix demo`.
- **Run (dev):** `npm run dev --prefix demo` starts Vite on `http://localhost:5173` and Wrangler on `http://localhost:8787` (API proxied via Vite). `predev` creates `demo/web/dist` so Wrangler can start without a prior frontend build. Requires `rustup target add wasm32-unknown-unknown`; on Linux, `libssl-dev` and `pkg-config` for `worker-build`.
- **Build:** `npm run build --prefix demo/web` runs `tsc -b` then `vite build` into `demo/web/dist/`. `npm run preview --prefix demo/web` serves the build on port 4173.
- **Native API:** `cargo run -p rundtisch-demo --features native --bin native` serves the demo API on `http://localhost:8080`.
- **Deploy:** Cloudflare Workers config lives in `demo/` (`wrangler.jsonc`, `wrangler.dev.jsonc`). Run `npm run build --prefix demo/web` then `npx wrangler deploy --config demo/wrangler.jsonc`.
