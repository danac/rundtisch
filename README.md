# Rundtisch (WORK IN PROGRESS)

Micro web framework with CMS features.

The repository is a **Cargo workspace** plus two apps: a **demo website** (React SPA + Rust Axum API) and **Crockis**, a photo library with the same auth API and a React SPA. The reusable framework lives in `crates/rundtisch`. See the component READMEs for implementation detail:

| Document | Scope |
|----------|-------|
| [crates/rundtisch/README.md](crates/rundtisch/README.md) | Library crate — `AppState`, SeaORM auth |
| [demo/web/README.md](demo/web/README.md) | React SPA — landing page and auth panels |
| [demo/api/README.md](demo/api/README.md) | Demo Axum app — `/api/*` routes on top of `rundtisch` |
| [crockis/web/README.md](crockis/web/README.md) | Crockis SPA — library, login, and account |
| [crockis/api/README.md](crockis/api/README.md) | Crockis Axum app — auth routes on top of `rundtisch` |

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
    ├── api/                   # Crockis app crate: auth routes, native + migrate bins
    │   └── src/bin/native.rs  # listens on 0.0.0.0:8788
    ├── web/                   # photo library SPA
    ├── wasmer.toml            # Wasmer package (WASIX binaries + web/dist)
    └── app.yaml               # Wasmer Edge app crockis-photos
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

The library is outside the Cargo workspace, so its tests use `crates/rundtisch/Cargo.lock`. Root `cargo test` runs the demo (the default workspace member). `cargo test -p crockis` runs the Crockis API. CI (`.github/workflows/ci.yml`) runs both, then both frontend builds.

### Crockis

`npm run dev --prefix crockis` starts Vite on http://localhost:5174 and the API on http://localhost:8788. MySQL is `mysql://demo:demo@127.0.0.1:3306/crockis_dev`. See [crockis/README.md](crockis/README.md).

Workflow: `.github/workflows/deploy-crockis.yml`

| Trigger | Action |
|---------|--------|
| Push | WASIX release of `crockis` → `wasmer deploy` to Edge app `crockis-photos` |
| **workflow_dispatch** | Same as production deploy |

Wasmer Edge serves the Axum API and the built SPA (`crockis/wasmer.toml`, `crockis/app.yaml`), with managed MySQL in `fr-roub1`, a `data` volume at `/data`, and a post-deploy migrate job that also seeds pictures. Hostname: `crockis-photos.wasmer.app`. The next deploy creates the volume; it is not attached yet.

Requires `WASMER_TOKEN` (secret) and `WASMER_OWNER` (variable or secret) in the GitHub **Wasmer** environment.

## Related documentation

- [crates/rundtisch/README.md](crates/rundtisch/README.md) — library API
- [demo/web/README.md](demo/web/README.md) — SPA and Vite proxy
- [demo/api/README.md](demo/api/README.md) — demo routes and the migrate binary
- [crockis/web/README.md](crockis/web/README.md) — Crockis SPA and Vite proxy
- [crockis/api/README.md](crockis/api/README.md) — Crockis auth routes and the migrate binary
- [AGENTS.md](AGENTS.md) — Cursor Cloud agent environment notes
