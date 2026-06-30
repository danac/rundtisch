---
name: Artist Portfolio Website
overview: Build a modern, lively, image-focused SPA for a children's watercolor/lettering artist using React + Vite + TypeScript + Tailwind, with React Router pages (Home, About, Portfolio, Merch), async image loading, a swappable data layer ready for a future REST backend, and react-i18next wired for easy internationalization.
todos:
  - id: scaffold
    content: Scaffold Vite React+TS project, install tailwind v4, react-router-dom, react-i18next/i18next, configure Vite + Tailwind plugin and fonts
    status: completed
  - id: theme
    content: Define watercolor design tokens (colors, fonts, radii, shadows) and base styles in index.css with Tailwind @theme
    status: completed
  - id: i18n
    content: Set up i18n/index.ts, en translation.json with all strings, and LanguageSwitcher component
    status: completed
  - id: datalayer
    content: Create types, mock data, apiClient (VITE_API_BASE_URL), portfolio/merch services, and useAsyncData hook
    status: completed
  - id: shell
    content: Build Layout, responsive Navbar (mobile hamburger), Footer, and wire React Router routes in App/main
    status: completed
  - id: asyncimage
    content: Implement AsyncImage with lazy/decoding-async + IntersectionObserver skeleton/blur and fade-in
    status: completed
  - id: gallery
    content: Build Gallery, ArtworkCard, ProductCard, Lightbox, Section building blocks
    status: completed
  - id: pages
    content: Implement Home, About, Portfolio (filter + lightbox), and Merch pages using services and i18n
    status: completed
  - id: assets
    content: Move reference images into public/images and reference them in mock data and pages
    status: completed
  - id: verify
    content: Run dev server and production build, fix type/responsive issues across mobile and desktop
    status: completed
isProject: false
---

## Overview

A single-page app (client-side routed) with four pages, designed around the reference look: clean white canvas, generous whitespace, a playful hand-lettered logo, simple top nav, and colorful image grids. Portfolio and merch data come from an abstracted service layer (mock JSON now, REST later). All UI text flows through `react-i18next`.

## Tech stack & tooling

- `Vite` + `React` + `TypeScript` (scaffold via `npm create vite@latest . -- --template react-ts`).
- `tailwindcss` (v4 with `@tailwindcss/vite` plugin) for styling.
- `react-router-dom` for SPA routing.
- `react-i18next` + `i18next` + `i18next-browser-languagedetector` for i18n.
- Google Fonts: a rounded playful display (e.g. `Fredoka`/`Quicksand`) for headings, a script accent (e.g. `Caveat`) for the logo, and a clean body sans (e.g. `Nunito Sans`).

## Project structure

```
artist-website/
  public/images/        # reference images moved here (image1.png, etc.)
  src/
    main.tsx            # router + i18n providers
    App.tsx             # <Routes> + <Layout>
    index.css           # tailwind + theme tokens + fonts
    i18n/
      index.ts          # i18next init
      locales/en/translation.json
    components/
      Layout.tsx, Navbar.tsx, Footer.tsx
      AsyncImage.tsx    # lazy/blur-up image loader
      Gallery.tsx, ArtworkCard.tsx, ProductCard.tsx
      Lightbox.tsx, Section.tsx, LanguageSwitcher.tsx
    pages/
      Home.tsx, About.tsx, Portfolio.tsx, Merch.tsx
    data/
      portfolio.mock.ts, merch.mock.ts
    services/
      apiClient.ts      # fetch wrapper + VITE_API_BASE_URL
      portfolioService.ts, merchService.ts
    hooks/
      useAsyncData.ts   # generic loading/error/data hook
    types/
      portfolio.ts, merch.ts
```

## Design system (`index.css` + Tailwind theme)

Derive a watercolor palette from the reference images: fox-orange/coral, teal, sky blue, soft pink, leaf green, on a near-white background. Define CSS variables and Tailwind `@theme` tokens for colors, the three font families, soft rounded radii, and gentle shadows. Add subtle entrance/hover animations (fade-in, image scale-on-hover) for the "lively" feel while keeping content the focus.

## Routing & layout

- `App.tsx`: `<Layout>` wrapping `<Routes>` for `/`, `/about`, `/portfolio`, `/merch`.
- `Navbar.tsx`: script logo + links, responsive hamburger menu on mobile, `LanguageSwitcher` (hidden until >1 locale, but present).
- `Footer.tsx`: social links + copyright, all text via i18n.

## Async images (`AsyncImage.tsx`)

Reusable component: `loading="lazy"` + `decoding="async"`, an `IntersectionObserver`-driven skeleton/blur placeholder, and a fade-in on load. Used everywhere images render so loading is always asynchronous and smooth on mobile.

## Data layer (REST-ready)

- `apiClient.ts`: thin `fetch` wrapper reading `import.meta.env.VITE_API_BASE_URL`.
- `portfolioService.getArtworks()` / `merchService.getProducts()` return `Promise<T[]>`. Now they resolve the local mock data (simulating latency); switching to REST later is changing the body to call `apiClient`, with no page changes.
- `useAsyncData` hook gives `{ data, loading, error }` so pages render loading skeletons and error states. (TanStack Query can be swapped in later if caching is wanted; not added now to keep it lean.)

## Pages

- Home: full-width hero (script headline + lead illustration), a "featured work" image grid pulling the first N portfolio items, a short intro blurb, and CTAs to Portfolio/Merch.
- About: artist portrait + bio (watercolor birds / lettering / children's illustration), all copy from i18n.
- Portfolio: responsive masonry/grid `Gallery` of `ArtworkCard`s with category filter (birds / lettering / children) and a click-to-open `Lightbox`. Data via `portfolioService`.
- Merch: responsive grid of `ProductCard`s (image, title, price, external/buy link). Data via `merchService`.

## Internationalization

- `i18n/index.ts` initializes i18next with the `en` bundle and language detector.
- `locales/en/translation.json` holds all nav, page, and component strings. Adding a language later = adding a `locales/<lang>/translation.json` and a switcher entry; `LanguageSwitcher` is built now.

## Assets

Move the provided reference PNG/JPGs into `public/images/` and reference them in the mock data and Home/About so the site looks real immediately.

## Verification

Run `npm run dev` and confirm all four routes render, images lazy-load with placeholders, layout is responsive (mobile hamburger + grids reflow), and `npm run build` succeeds with no type errors.
