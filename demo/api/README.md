# Rundtisch — Demo API

Small Axum app that exercises [`rundtisch`](../../crates/rundtisch/README.md). Routes live here. `src/bin/native.rs` serves them. `src/bin/migrate.rs` applies pending migrations.

## Tech stack

| Component | Role |
|-----------|------|
| Rust (stable) | Source language |
| Axum 0.8 | HTTP router and handlers |
| rundtisch | `AppState`, auth handlers |
| SeaORM 2 | `DatabaseConnection`, migrations |

## Project structure

```
demo/api/
  Cargo.toml
  src/
    lib.rs
    routes.rs
    handlers.rs
    email.rs              # SendmailEmailSender (rundtisch::EmailSender)
    native_platform.rs    # Database::connect(DATABASE_URL)
    migrator.rs           # re-exports rundtisch::auth::Migrator
    listen.rs             # BIND_ADDR/PORT, default 0.0.0.0:8787
    static_files.rs       # optional SPA from /app/web or STATIC_DIR
    bin/native.rs         # listen; serve web/dist when the static dir exists
    bin/migrate.rs        # Migrator::up, then optional bootstrap Admin upsert
    bin/auth_link.rs      # mint invitation and recovery links
```

## Endpoints

| Method | Path | Response |
|--------|------|----------|
| GET | `/api/health` | `{ "status": "ok", "headers": [...] }` |
| POST | `/api/auth/register/password`, `/api/auth/register_with_token` | Consume an invitation and set a password |
| POST | `/api/auth/register/passkey/options`, `/api/auth/register/passkey` | Invitation passkey ceremony |
| POST | `/api/auth/login` | Password login; `session` cookie and opaque bearer |
| POST | `/api/auth/passkeys/login/options`, `/api/auth/passkeys/login` | Passkey login |
| POST | `/api/auth/logout` | Revoke this session, clear cookie |
| POST | `/api/auth/logout_all` | Revoke every session for the account |
| GET | `/api/auth/me` | Cookie or `Authorization: Bearer` |
| PUT | `/api/auth/alias` | Update the signed-in user's display alias |
| POST | `/api/auth/request_reset` | `202` always; recovery mail is sent in the background when the account exists |
| POST | `/api/auth/reset` | Password recovery |
| POST | `/api/auth/reset/passkey/options`, `/api/auth/reset/passkey` | Passkey recovery |
| GET | `/api/auth/passkeys` | List passkeys |
| POST | `/api/auth/passkeys/register/options`, `/api/auth/passkeys/register` | Add a passkey |
| DELETE | `/api/auth/passkeys/{public_id}` | Remove a passkey by opaque UUID, unless it is the last credential |

## Run

`DATABASE_URL` selects the backend (`sqlite://`, `mysql://`, or `postgres://`). Apply migrations before starting the server. The server does not migrate on startup or per request. On Wasmer Edge, `app.yaml` runs the `migrate` command once per deploy as a `post-deployment` job. After `Migrator::up`, that command upserts a bootstrap admin when `RUNDTISCH_BOOTSTRAP_ADMIN_EMAIL` and `RUNDTISCH_BOOTSTRAP_ADMIN_PASSWORD` are both set: insert a verified Admin if the email is absent, or reset the password on the existing row (`public_id`, role, alias, and verification stay the same).

```bash
export DATABASE_URL=sqlite://rundtisch.sqlite?mode=rwc
export AUTH_HASH_PEPPER=cccccccccccccccccccccccccccccccc
# optional WebAuthn overrides (defaults suit Vite on localhost:5173):
# export AUTH_WEBAUTHN_RP_ID=localhost
# export AUTH_WEBAUTHN_RP_ORIGIN=http://localhost:5173
# export AUTH_WEBAUTHN_RP_NAME=rundtisch
# optional From address for outbound mail (default rundtisch@localhost):
# export AUTH_MAIL_FROM=rundtisch@localhost
# optional first admin (insert if absent, or reset the password):
# export RUNDTISCH_BOOTSTRAP_ADMIN_EMAIL=admin@example.com
# export RUNDTISCH_BOOTSTRAP_ADMIN_PASSWORD=unique-passphrase-ok

cargo run -p rundtisch-demo --bin migrate
cargo run -p rundtisch-demo --bin native
curl -i http://localhost:8787/api/health
```

`PORT` and `BIND_ADDR` override the listen address (Wasmer Edge uses `80` / `127.0.0.1`). When `/app/web` exists — the `[fs]` mount in `demo/wasmer.toml` — or `STATIC_DIR` points at `demo/web/dist`, the same process serves the built SPA. Leave both unset for Vite split-dev.

| Secret | Length |
|--------|--------|
| `AUTH_HASH_PEPPER` | exactly 32 bytes |
| `AUTH_WEBAUTHN_RP_ID` | optional; default `localhost` |
| `AUTH_WEBAUTHN_RP_ORIGIN` | optional; default `http://localhost:5173` |
| `AUTH_WEBAUTHN_RP_NAME` | optional; default `rundtisch` |
| `AUTH_MAIL_FROM` | optional; default `rundtisch@localhost` |
| `RUNDTISCH_BOOTSTRAP_ADMIN_EMAIL` | optional; skip seed if unset |
| `RUNDTISCH_BOOTSTRAP_ADMIN_PASSWORD` | optional; must be set with the email; upserts the password |
| `RUNDTISCH_BOOTSTRAP_ADMIN_ALIAS` | optional; defaults to the email local-part |

`POST /api/auth/request_reset` always returns `202` and runs recovery in the background. When the account exists, `SendmailEmailSender` writes the message to a temporary file and passes that file to `sendmail -t` (WASIX does not deliver EOF on a subprocess pipe). On Wasmer Edge, that needs `enable_email: true` in `app.yaml` and the `sendmail/sendmail` package. Locally, if sendmail is unavailable, mint a link with `auth-link -- recover` instead.

`auth-link` prints an invitation or recovery URL. `recover` exits non-zero when the email has no user. The raw token is not stored.

```bash
cargo run -p rundtisch-demo --bin auth-link -- invite \
  --email user@example.com [--ttl-hours 24] [--base-url http://localhost:5173]
cargo run -p rundtisch-demo --bin auth-link -- recover \
  --email user@example.com [--ttl-hours 1] [--base-url http://localhost:5173]
```

## Check

```bash
cargo test
cargo check -p rundtisch-demo
```

## Adding a route

1. Add a handler in `src/handlers.rs` (or call one from `rundtisch::auth`).
2. Register it in `src/routes.rs` under `/api/...`.
3. `curl http://localhost:8787/api/<path>`.

## See also

- [Root README](../../README.md)
- [rundtisch crate](../../crates/rundtisch/README.md)
- [Frontend README](../web/README.md)
