# anikaelsa — Frontend

The public-facing artist portfolio: a modern, image-focused single-page app for **Anika**, who paints watercolor birds, hand lettering, and illustrations for children.

Part of the [anikaelsa monorepo](../README.md). The built output (`dist/`) is served as static assets by the Cloudflare Worker; API routes are handled separately by the [Rust backend](../backend/README.md).

> Copy and sample artwork can be customized in
> `src/i18n/locales/en/translation.json` and the images in `public/images/`.

## Tech stack

| Layer | Choice | Notes |
|-------|--------|-------|
| UI | React 19 | Functional components, hooks |
| Build | Vite 6 | Fast HMR, `@vitejs/plugin-react` |
| Language | TypeScript | Strict mode via `tsconfig` project references |
| Styling | Tailwind CSS v4 | `@tailwindcss/vite` plugin; design tokens in `src/index.css` |
| Routing | react-router-dom 7 | Client-side SPA routes |
| i18n | react-i18next | English + French UI strings |

## Pages

| Route | Page | Notes |
|-------|------|-------|
| `/` | Home | Hero, featured gallery, categories, call to action |
| `/portfolio` | Portfolio | Filterable gallery + click-to-open lightbox |
| `/shop` | Shop | Product grid with external buy links |
| `/about` | About | Artist bio and facts |

## Project structure

```
src/
  components/   # Layout, Navbar, Footer, AsyncImage, Gallery, cards, Lightbox…
  pages/        # Home, Portfolio, Merch, About, NotFound
  services/     # Data access — mocks today, REST when VITE_API_BASE_URL is set
  data/         # Bundled sample data (homepage, portfolio, merch mocks)
  hooks/        # useAsyncData (loading / error / reload), useDocumentMeta
  types/        # HomepageContent, Collection, Artwork, Product
  i18n/         # i18next setup + locale JSON files
  index.css     # Tailwind import + @theme design tokens
public/
  images/       # Artwork, flags, favicon (served as static files)
```

## Design decisions

### Services layer over direct data access

Components never import from `src/data/` directly. Every page goes through `src/services/` (`homepageService`, `portfolioService`, `merchService`, `translationService`). Each service checks `hasBackend` from `apiClient.ts`:

- **`VITE_API_BASE_URL` unset or empty** — resolve bundled mocks (with simulated latency via `withLatency`)
- **`VITE_API_BASE_URL` set** — `fetch` from the REST API via `apiGet`

This means switching from mocks to a live API requires no component changes — only environment configuration and backend route implementation.

### Same-origin API path

In production the API lives on the same Worker origin under `/api/*`. When wiring up the backend, set:

```bash
# .env.local (dev) or build-time env (CI)
VITE_API_BASE_URL=/api
```

Requests then go to `/api/homepage`, `/api/collections`, etc. No CORS headers needed.

### Async image loading

Every image renders through `components/AsyncImage.tsx`:

- Loads when near the viewport (`IntersectionObserver`) plus native `loading="lazy"` / `decoding="async"`
- Shows a shimmer placeholder, fades in on load
- Pass `priority` for above-the-fold images (hero) to load eagerly

### Internationalization split

**UI strings** (nav, labels, errors, meta) live in `src/i18n/locales/<lng>/translation.json` and are served via `react-i18next`.

**Content strings** (collection titles, artwork names, product descriptions) are **not** in translation files — they will come from the backend/CMS later and are currently embedded in mock data.

At dev/build time, locale files are copied to `public/locales/` by the `sync:locales` script (`predev` / `prebuild` hooks). The app can also fetch `GET /translations/<lng>` when `VITE_API_BASE_URL` is set.

**Language switching:** flag links in the footer (GB = English, FR = French), persisted in `localStorage`.

## Connecting a backend

The Rust worker in `../backend/` is deployed but **not yet called by the frontend**. Services still use mocks.

### Expected REST endpoints

When `VITE_API_BASE_URL` is set, services call:

| Service | Method | Path | Returns |
|---------|--------|------|---------|
| `homepageService` | GET | `/homepage` | `HomepageContent` |
| `portfolioService` | GET | `/collections` | `Collection[]` |
| `portfolioService` | GET | `/collections/:slug` | `Collection` |
| `merchService` | GET | `/products` | `Product[]` |
| `merchService` | GET | `/products/:id` | `Product` |
| `translationService` | GET | `/translations/:lng` | translation JSON |

With `VITE_API_BASE_URL=/api`, paths become `/api/homepage`, `/api/collections`, etc. See [.env.example](.env.example) for the full list.

### Enabling the API

```bash
# frontend/.env.local
VITE_API_BASE_URL=/api
```

Restart the dev server after changing env vars. Until matching routes exist in the backend, requests will fail — keep the variable unset to use mocks.

## Development

### Recommended: monorepo dev (frontend + API)

From the **repository root**:

```bash
npm run dev
```

Opens Vite on http://localhost:5173 with `/api` proxied to Wrangler on port 8787. See [root README — Local development](../README.md#local-development).

### Frontend only

```bash
cd frontend
npm install
npm run dev      # http://localhost:5173
```

Use this when working on UI with mock data and no backend needed. The `predev` hook syncs locale files to `public/locales/`.

## Build

```bash
cd frontend
npm run build    # tsc -b && vite build → dist/
npm run preview  # serve dist/ on port 4173
```

The `prebuild` hook runs `sync:locales` before the TypeScript and Vite build.

Output goes to `frontend/dist/` and is uploaded by Wrangler as static assets (see [root README — Deployment](../README.md#deployment)).

## Deployment

The frontend is not deployed independently. CI builds `dist/` and Wrangler uploads it alongside the WASM worker:

```bash
npm run build
npx wrangler deploy --config ../wrangler.jsonc
```

Or from this directory: `npm run deploy` (build + deploy in one step).

Cloudflare config lives at the repo root: `wrangler.jsonc` (production) and `wrangler.dev.jsonc` (local).

## Adding a language

1. Add `src/i18n/locales/<lng>/translation.json` (same key structure as `en`)
2. Register the code in `src/i18n/languages.ts`
3. Add a bundled fallback import in `translationService.ts`
4. Add a flag asset under `public/images/flags/`

## Lint

The `lint` script runs `eslint .`, but ESLint is not in `devDependencies` and no config exists — `npm run lint` will fail. This is a known repo state, not an environment issue.

## See also

- [Root README](../README.md) — monorepo architecture, CI, combined dev workflow
- [Backend README](../backend/README.md) — API routes and WASM build
