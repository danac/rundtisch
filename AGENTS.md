# AGENTS.md

## Cursor Cloud specific instructions

This repo is a single client-side SPA (**anikaelsa**, an artist portfolio site) built with React 19 + Vite 6 + TypeScript + Tailwind CSS v4. The frontend app lives in `frontend/`. There is no backend, database, or Docker in the repo; all data comes from bundled mocks in `frontend/src/data/`.

Standard commands live in `frontend/package.json` (`dev`, `build`, `preview`). Notes:

- **Run (dev):** `cd frontend && npm run dev` serves on `http://localhost:5173` (Vite default). This is the only service needed for full end-to-end testing.
- **Build:** `cd frontend && npm run build` runs `tsc -b` then `vite build` into `frontend/dist/`. `npm run preview` serves the build on port 4173.
- **Lint is non-functional:** the `lint` script runs `eslint .`, but ESLint is not declared in `devDependencies` and no ESLint config exists, so `npm run lint` fails with `eslint: not found`. This is a pre-existing repo state, not an environment problem.
- **Optional backend:** setting `VITE_API_BASE_URL` (see `frontend/.env.example`) switches the data layer in `frontend/src/services/` from local mocks to a REST API. Leave it unset to use bundled mock data (no backend required).
- **Deploy:** Cloudflare Workers config is at the repo root in `wrangler.jsonc`. Run `cd frontend && npm run deploy` for a local production deploy.
- Static images served from `frontend/public/images/` are present and load correctly.
