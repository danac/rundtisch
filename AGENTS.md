# AGENTS.md

## Cursor Cloud specific instructions

This repo is a single client-side SPA (**anikaelsa**, an artist portfolio site) built with React 19 + Vite 6 + TypeScript + Tailwind CSS v4. There is no backend, database, or Docker in the repo; all data comes from bundled mocks in `src/data/`.

Standard commands live in `package.json` (`dev`, `build`, `preview`). Notes:

- **Run (dev):** `npm run dev` serves on `http://localhost:5173` (Vite default). This is the only service needed for full end-to-end testing.
- **Build:** `npm run build` runs `tsc -b` then `vite build` into `dist/`. `npm run preview` serves the build on port 4173.
- **Lint is non-functional:** the `lint` script runs `eslint .`, but ESLint is not declared in `devDependencies` and no ESLint config exists, so `npm run lint` fails with `eslint: not found`. This is a pre-existing repo state, not an environment problem.
- **Optional backend:** setting `VITE_API_BASE_URL` (see `.env.example`) switches the data layer in `src/services/` from local mocks to a REST API. Leave it unset to use bundled mock data (no backend required).
- Static images served from `public/images/` are present and load correctly.
