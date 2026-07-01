# anikaelsa — Artist Portfolio Website

A modern, lively, image-focused website for **Anika**, a visual artist who paints
watercolor birds, hand lettering, and illustrations for children.

Built as a client-side single-page app with **React + Vite + TypeScript +
Tailwind CSS v4**, with async image loading, a data layer ready for a future
REST backend, and internationalization wired through `react-i18next`.

> Copy and sample artwork can be customized in
> `src/i18n/locales/en/translation.json` and the images in `public/images/`.

## Tech stack

- **React 19** + **Vite 6** + **TypeScript**
- **Tailwind CSS v4** via `@tailwindcss/vite`
- **react-router-dom** for SPA routing
- **react-i18next** for internationalization

## Getting started

```bash
cd frontend
npm install
npm run dev      # start the dev server (http://localhost:5173)
npm run build    # type-check + production build into dist/
npm run preview  # preview the production build
```

Configuration for Cloudflare deployment is in `wrangler.jsonc` at the repository root.

## Pages

| Route        | Page      | Notes                                              |
| ------------ | --------- | -------------------------------------------------- |
| `/`          | Home      | Hero, featured gallery, categories, call to action |
| `/portfolio` | Portfolio | Filterable gallery + click-to-open lightbox        |
| `/shop`      | Shop      | Product grid with external buy links               |
| `/about`     | About     | Artist bio and facts                               |

## Project structure

```
src/
  components/   # Layout, Navbar, Footer, AsyncImage, Gallery, cards, Lightbox…
  pages/        # Home, Portfolio, Merch, About, NotFound
  services/     # apiClient + portfolio/merch services (mock now, REST later)
  data/         # local sample data (portfolio.mock, merch.mock)
  hooks/        # useAsyncData (loading / error / reload)
  types/        # Artwork, Product types
  i18n/         # i18next setup + locale files
  index.css     # Tailwind import + design tokens (@theme)
```

## Connecting a backend later

The UI never talks to data sources directly — it goes through
`src/services`. Today those services resolve the bundled sample data in
`src/data`. To switch to a live REST API, set an environment variable:

```bash
# .env.local
VITE_API_BASE_URL=https://api.example.com
```

When set, the services fetch `GET /artworks` and `GET /products` (returning
`Artwork[]` and `Product[]`) instead of the local mocks — no component
changes required. See [.env.example](.env.example).

## Async image loading

Every image renders through `components/AsyncImage.tsx`, which:

- only begins loading when the image is near the viewport
  (`IntersectionObserver`), plus native `loading="lazy"` / `decoding="async"`
- shows a soft shimmer placeholder, then fades the image in once loaded

Pass `priority` for above-the-fold images (e.g. the hero) to load eagerly.

## Internationalization

Static UI copy is translated for **English** (`en`) and **French** (`fr`). Every
shell string (nav, pages, footer, errors, meta tags) flows through `react-i18next`
and the `t()` helper.

**Source of truth:** `src/i18n/locales/<lng>/translation.json`

At dev/build time these files are copied to `public/locales/` and served as
static JSON. The app loads them via `src/services/translationService.ts`, which
can also fetch `GET /translations/<lng>` when `VITE_API_BASE_URL` is set.

**Portfolio and shop content** (collection titles, artwork names, product
descriptions) is **not** in translation files — that content will come from the
CMS/backend later.

**Switching language:** flag links in the footer (GB = English, FR = French).
The choice is persisted in `localStorage`.

To add another language later:

1. Add `src/i18n/locales/<lng>/translation.json` (same key structure as `en`)
2. Register the code in `src/i18n/languages.ts` and add a bundled fallback in
   `translationService.ts`
3. Add a flag asset under `public/images/flags/`
