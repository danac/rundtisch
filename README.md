# Rundtisch (WORK IN PROGRESS)

Micro web framework with CMS features.

The repository is a **Cargo workspace** plus two frontends: a **demo website** (React SPA + Rust Axum API) and **Crockis**, a photo-library SPA that will later sit on the same `rundtisch` crate. The reusable framework lives in `crates/rundtisch`. See the component READMEs for implementation detail:

| Document | Scope |
|----------|-------|
| [crates/rundtisch/README.md](crates/rundtisch/README.md) | Library crate — `AppState`, SeaORM auth |
| [demo/web/README.md](demo/web/README.md) | React SPA — landing page and auth panels |
| [demo/api/README.md](demo/api/README.md) | Demo Axum app — `/api/*` routes on top of `rundtisch` |
| [crockis/web/README.md](crockis/web/README.md) | Crockis SPA — collections, photo mosaic, login (frontend only) |

## Architecture

Application routes depend on `rundtisch` and a `sea_orm::DatabaseConnection`. The demo entry point opens that connection from `DATABASE_URL` (`sqlite://`, `mysql://`, or `postgres://`).

```
Browser (localhost:5173)
    │
    ├─ /, /assets/...  →  Vite dev server (HMR)
    │
    └─ /api/*          →  Vite proxy  →  native Axum (localhost:8787)
                                            └─ AppState { db, email }
```

`npm run dev --prefix demo` starts both processes. The dev script applies pending migrations to a local SQLite file, then starts the API. The server itself does not migrate on startup.

### Repository layout

```
.
├── Cargo.toml                 # workspace
├── crates/
│   └── rundtisch/             # lib (AppState, auth, SeaORM migrations)
├── demo/
│   ├── api/                   # demo app crate: routes, native + migrate bins
│   │   └── src/bin/native.rs  # listens on 0.0.0.0:8787
│   ├── web/                   # React SPA
│   ├── wasmer.toml            # Wasmer package (WASIX binaries + web/dist)
│   ├── app.yaml               # Wasmer Edge app rundtisch
│   └── package.json           # concurrently; `npm run dev`
└── crockis/
    ├── web/                   # photo library SPA (frontend first)
    ├── wasmer.toml            # Wasmer package (static-web-server + web/dist)
    ├── settings/              # static-web-server config (SPA 404 + cache)
    └── app.yaml               # Wasmer Edge app crockis
```

## Local development

### First-time setup

```bash
npm install --prefix demo
npm install --prefix demo/web
```

### Start

```bash
npm run dev --prefix demo
```

| Process | Label | URL | Role |
|---------|-------|-----|------|
| Vite | `fe` | http://localhost:5173 | SPA with HMR — **open this in the browser** |
| API | `api` | http://localhost:8787 | Native Axum server |

Auth routes need `AUTH_HASH_PEPPER` (exactly 32 bytes). The dev script sets `DATABASE_URL` and a local pepper. WebAuthn uses RP id `localhost`, origin `http://localhost:5173`, and name `rundtisch` unless `AUTH_WEBAUTHN_RP_ID`, `AUTH_WEBAUTHN_RP_ORIGIN`, or `AUTH_WEBAUTHN_RP_NAME` is set. Passkey registration requires a discoverable credential. Login does not ask for an email: the browser fills it from the passkey. Credentials created before that requirement must be registered again.

| Name | Length |
|------|--------|
| `AUTH_HASH_PEPPER` | exactly 32 bytes; HMAC for session, invitation, and recovery tokens |
| `AUTH_WEBAUTHN_RP_ID` | optional; default `localhost` |
| `AUTH_WEBAUTHN_RP_ORIGIN` | optional; default `http://localhost:5173` |
| `AUTH_WEBAUTHN_RP_NAME` | optional; default `rundtisch` |
| `RUNDTISCH_BOOTSTRAP_ADMIN_EMAIL` | optional migrate seed; skip if unset |
| `RUNDTISCH_BOOTSTRAP_ADMIN_PASSWORD` | optional; required with the email; upserts the password for that email |
| `RUNDTISCH_BOOTSTRAP_ADMIN_ALIAS` | optional; defaults to the email local-part |

Invitation and recovery links are opaque database tokens. The SPA never prints a recovery link. Mint one with the demo CLI (same pepper and database as the API):

```bash
cargo run -p rundtisch-demo --bin auth-link -- invite --email user@example.com
cargo run -p rundtisch-demo --bin auth-link -- recover --email user@example.com
```

### Verify

```bash
curl http://localhost:8787/api/health
curl http://localhost:5173/api/health
```

### API and migrations without the frontend

```bash
export DATABASE_URL=sqlite://rundtisch.sqlite?mode=rwc
cargo run -p rundtisch-demo --bin migrate
cargo run -p rundtisch-demo --bin native
curl http://localhost:8787/api/health
```

## Build and test

```bash
cargo test --manifest-path crates/rundtisch/Cargo.toml
cargo test
npm run build --prefix demo/web
```

The library is outside the Cargo workspace, so its tests use `crates/rundtisch/Cargo.lock`. Root `cargo test` runs the demo. CI (`.github/workflows/ci.yml`) runs both, then the frontend build.

### Crockis

Workflow: `.github/workflows/deploy-crockis.yml`

| Trigger | Action |
|---------|--------|
| Push | Build `crockis/web` → `wasmer deploy` to Edge app `crockis` |
| **workflow_dispatch** | Same as production deploy |

There is no Rust/WASM build; Wasmer Edge serves the Vite SPA from `crockis/web/dist/` via `wasmer/static-web-server` (`crockis/wasmer.toml`, `crockis/app.yaml`).

Requires `WASMER_TOKEN` (secret) and `WASMER_OWNER` (variable or secret) in the GitHub **Wasmer** environment.

```bash
npm install --prefix crockis/web
npm run build --prefix crockis/web
# from crockis/; pass --owner if app.yaml has no owner
wasmer deploy --non-interactive --bump --no-persist-id --publish-package
```

## Related documentation

- [crates/rundtisch/README.md](crates/rundtisch/README.md) — library API
- [demo/web/README.md](demo/web/README.md) — SPA and Vite proxy
- [demo/api/README.md](demo/api/README.md) — demo routes and the migrate binary
- [crockis/web/README.md](crockis/web/README.md) — Crockis photo library SPA
- [AGENTS.md](AGENTS.md) — Cursor Cloud agent environment notes
