# Crockis — API

Axum app for the Crockis photo library. Auth routes come from [`rundtisch`](../../crates/rundtisch/README.md). `src/bin/native.rs` serves them as `crockis-native`. `src/bin/migrate.rs` applies pending migrations, then seeds the library, as `crockis-migrate`.

Collections and pictures are SeaORM tables. Image bytes are files in `DATA_DIR` (default `/data`, the Wasmer volume `data`).

## Tech stack

| Component | Role |
|-----------|------|
| Rust (stable) | Source language |
| Axum 0.8 | HTTP router and handlers |
| rundtisch | `AppState`, auth handlers |
| SeaORM 2 | `DatabaseConnection`, migrations |

## Project structure

```
crockis/api/
  Cargo.toml
  src/
    lib.rs
    routes.rs
    handlers.rs
    native_platform.rs    # Database::connect(DATABASE_URL)
    migrator/             # auth migrator, then crockis_migrations
    library/              # entities, seed catalog, file paths, HTTP handlers
    listen.rs             # BIND_ADDR/PORT, default 0.0.0.0:8788
    static_files.rs       # optional SPA from /app/web or STATIC_DIR
    bin/native.rs         # listen; serve web/dist when the static dir exists
    bin/migrate.rs        # migrate, seed /data, then optional bootstrap Admin upsert
    bin/auth_link.rs      # mint invitation and recovery links
```

Binary names are `crockis-native`, `crockis-migrate`, and `crockis-auth-link` so they do not collide with the demo crate in the same Cargo workspace. Wasmer command names stay `native` and `migrate`.

## Endpoints

| Method | Path | Response |
|--------|------|----------|
| GET | `/api/health` | `{ "status": "ok", "headers": [...] }` |
| GET | `/api/collections` | Signed-in library. Each item has `cover` and `photoCount` |
| GET | `/api/collections/{id}` | One collection, or 404 |
| GET | `/api/collections/{id}/photos` | Pictures in mosaic order |
| GET | `/api/photos/{id}/file` | Stored image bytes (`image/*`), same session cookie |
| POST | `/api/auth/register/invitation` | `204` when the invitation token is still open |
| POST | `/api/auth/register/password`, `/api/auth/register_with_token` | Consume an invitation and set a password |
| POST | `/api/auth/register/passkey/options`, `/api/auth/register/passkey` | Invitation passkey ceremony |
| POST | `/api/auth/login` | Password login; `session` cookie and opaque bearer |
| POST | `/api/auth/passkeys/login/options`, `/api/auth/passkeys/login` | Passkey login |
| POST | `/api/auth/step-up/login` | Step-up with the account password |
| POST | `/api/auth/step-up/passkeys/login/options`, `/api/auth/step-up/passkeys/login` | Step-up with a passkey |
| POST | `/api/auth/logout` | Revoke this session, clear cookie |
| POST | `/api/auth/logout_all` | Revoke every session for the account |
| GET | `/api/auth/me` | Cookie or `Authorization: Bearer` |
| PUT | `/api/auth/alias` | Update the signed-in user's display alias |
| GET | `/api/auth/sessions` | List active sessions |
| POST | `/api/auth/sessions` | Mint a session for the CLI authorize flow |
| DELETE | `/api/auth/sessions/{public_id}` | Revoke one session |
| PUT | `/api/auth/password` | Set a password (step-up) |
| DELETE | `/api/auth/password` | Remove the password when a passkey remains (step-up) |
| POST | `/api/auth/request_reset` | `202` always; recovery mail is sent in the background when the account exists |
| POST | `/api/auth/reset` | Password recovery |
| POST | `/api/auth/reset/passkey/options`, `/api/auth/reset/passkey` | Passkey recovery |
| GET | `/api/auth/passkeys` | List passkeys |
| POST | `/api/auth/passkeys/register/options`, `/api/auth/passkeys/register` | Add a passkey |
| DELETE | `/api/auth/passkeys/{public_id}` | Remove a passkey by opaque UUID, unless it is the last credential |

## Run

`DATABASE_URL` selects the backend (`sqlite://`, `mysql://`, or `postgres://`). Apply migrations before starting the server. The server does not migrate or seed on startup or per request. On Wasmer Edge, `app.yaml` runs the `migrate` command once per deploy as a `post-deployment` job. That command applies auth migrations (`rundtisch_migrations_auth`) and library migrations (`crockis_migrations`), downloads missing seed images into `DATA_DIR` (Edge: `/data`), then upserts a bootstrap admin when `RUNDTISCH_BOOTSTRAP_ADMIN_EMAIL` and `RUNDTISCH_BOOTSTRAP_ADMIN_PASSWORD` are both set: insert a verified Admin if the email is absent, or reset the password on the existing row (`public_id`, role, alias, and verification stay the same).

Library rows:

| Table | Purpose |
|-------|---------|
| `collections` | `public_id` slug, name, description, `created_at`, `sort_order`, nullable `thumbnail_picture_id` (no FK; set after pictures exist) |
| `pictures` | `public_id`, `collection_id` FK, pixel `width`/`height` of the stored file, `original_filename`, `captured_at`, unique `storage_filename` (`{uuid}.{ext}`), `alt`, `title`, `sort_order` |

Files are `{DATA_DIR}/{storage_filename}`. The thumbnail column is not a foreign key, so the picture → collection FK can cascade without a cycle. `CROCKIS_SEED=0` skips downloads. A second migrate leaves existing files in place and only fills gaps.

Local MySQL uses the same account as the demo, and a separate database name:

```bash
export DATABASE_URL=mysql://demo:demo@127.0.0.1:3306/crockis_dev
export DATA_DIR=crockis/data
export AUTH_HASH_PEPPER=cccccccccccccccccccccccccccccccc
export AUTH_WEBAUTHN_RP_ID=localhost
export AUTH_WEBAUTHN_RP_ORIGIN=http://localhost:5174
export AUTH_WEBAUTHN_RP_NAME=crockis-photos
# SMTP STARTTLS (required for the native binary to start, and for recovery mail):
export AUTH_SMTP_URL=localhost
export AUTH_SMTP_USERNAME=crockis
export AUTH_SMTP_PASSWORD=crockis
export AUTH_MAIL_FROM=crockis-photos@localhost

cargo run -p crockis --bin crockis-migrate
cargo run -p crockis --bin crockis-native
curl -i http://localhost:8788/api/health
```

The library's WebAuthn defaults are `localhost`, `http://localhost:5173`, and `rundtisch`. Crockis overrides them with the variables above so passkeys match Vite on port 5174. `npm run dev --prefix crockis` sets `DATA_DIR` to `crockis/data`; export the auth and SMTP variables above as well.

`PORT` and `BIND_ADDR` override the listen address (Wasmer Edge uses `80` / `127.0.0.1`). When `/app/web` exists — the `[fs]` mount in `crockis/wasmer.toml` — or `STATIC_DIR` points at `crockis/web/dist`, the same process serves the built SPA. Leave both unset for Vite split-dev.

| Secret | Length |
|--------|--------|
| `AUTH_HASH_PEPPER` | exactly 32 bytes |
| `AUTH_WEBAUTHN_RP_ID` | dev script sets `localhost`; on Edge set the secret `crockis-photos.wasmer.app` |
| `AUTH_WEBAUTHN_RP_ORIGIN` | dev script sets `http://localhost:5174`; on Edge set the secret `https://crockis-photos.wasmer.app` |
| `AUTH_WEBAUTHN_RP_NAME` | `crockis-photos` (dev script locally, app secret on Edge) |
| `AUTH_SMTP_URL` | SMTP host or `smtp://host[:port]` (STARTTLS) |
| `AUTH_SMTP_USERNAME` | SMTP auth username |
| `AUTH_SMTP_PASSWORD` | SMTP auth password |
| `AUTH_MAIL_FROM` | optional; dev script uses `crockis-photos@localhost` |
| `RUNDTISCH_BOOTSTRAP_ADMIN_EMAIL` | optional; skip seed if unset |
| `RUNDTISCH_BOOTSTRAP_ADMIN_PASSWORD` | optional; must be set with the email; upserts the password |
| `RUNDTISCH_BOOTSTRAP_ADMIN_ALIAS` | optional; defaults to the email local-part |

`POST /api/auth/request_reset` always returns `202` and runs recovery in the background. When the account exists, `rundtisch::SmtpEmailSender` (crate feature `smtp`) sends mail over SMTP STARTTLS. Set `AUTH_SMTP_*` as app secrets on Wasmer Edge. Locally, if SMTP cannot deliver, mint a link with `crockis-auth-link -- recover` instead.

`crockis-auth-link` prints an invitation or recovery URL. `recover` exits non-zero when the email has no user. The raw token is not stored.

```bash
cargo run -p crockis --bin crockis-auth-link -- invite \
  --email user@example.com [--ttl-hours 24] [--base-url http://localhost:5174]
cargo run -p crockis --bin crockis-auth-link -- recover \
  --email user@example.com [--ttl-hours 1] [--base-url http://localhost:5174]
```

Open `http://localhost:5174/?invite=…` or `/?recover=…`. The SPA keeps that query on the login screen.

## Check

```bash
cargo test -p crockis
cargo check -p crockis
```

## See also

- [Root README](../../README.md)
- [rundtisch crate](../../crates/rundtisch/README.md)
- [Frontend README](../web/README.md)
