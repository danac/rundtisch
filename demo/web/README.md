# Rundtisch — Demo frontend

React SPA used to exercise the [demo API](../api/README.md).

## Tech stack

| Component | Role |
|-----------|------|
| Vite | Dev server, HMR, production build |
| React 19 + TypeScript | UI |
| Tailwind CSS v4 | Styling (`@tailwindcss/vite`) |

## Current state

Landing page with a **register / login / session** panel (the activation token is returned in the register JSON) and a **users** CRUD panel against `/api/auth/users`.

## Commands

From the repository root (also starts the native API):

```bash
npm install --prefix demo
npm install --prefix demo/web
npm run dev --prefix demo            # Vite :5173 + API :8787
```

Frontend only:

```bash
npm install --prefix demo/web
npm run dev --prefix demo/web        # http://localhost:5173
npm run build --prefix demo/web      # → demo/web/dist/
npm run preview --prefix demo/web    # serve the build on :4173
```

## API proxy

Vite proxies `/api` to `http://localhost:8787` (`vite.config.ts`). `fetch('/api/health')` reaches the native Axum server. Auth cookies need `credentials: 'include'`.

## See also

- [Root README](../../README.md)
- [Demo API README](../api/README.md)
