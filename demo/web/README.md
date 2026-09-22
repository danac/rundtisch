# Rundtisch — Demo frontend

Small React SPA used to exercise the [demo API](../api/README.md) while the [`rundtisch`](../../crates/rundtisch/README.md) crate is being built. Deployed as static assets on the same Cloudflare Worker (`demo/web/dist/`).

## Tech stack

| Component | Role |
|-----------|------|
| Vite 6 | Dev server, HMR, production build |
| React 19 + TypeScript | UI |
| Tailwind CSS v4 | Styling (`@tailwindcss/vite`) |

## Current state

Landing page with a **register / login / session** panel (activation token is returned in the register JSON until a mailer exists) and a **users** CRUD panel against `/api/auth/users`.

## Commands

From the repository root (recommended — also starts the Wrangler API):

```bash
npm install --prefix demo            # concurrently + wrangler
npm install --prefix demo/web
npm run dev --prefix demo            # Vite :5173 + Wrangler :8787
```

Frontend only:

```bash
npm install --prefix demo/web
npm run dev --prefix demo/web        # http://localhost:5173
npm run build --prefix demo/web      # → demo/web/dist/
npm run preview --prefix demo/web    # serve the build on :4173
```

## API proxy

In local dev, Vite proxies `/api` to `http://localhost:8787` (see `vite.config.ts`). Same-origin requests like `fetch('/api/health')` reach the Rust worker without CORS setup.

## See also

- [Root README](../../README.md) — monorepo architecture and combined `npm run dev --prefix demo`
- [Demo API README](../api/README.md) — Axum routes and WASM build
