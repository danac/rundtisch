# rundtisch

Axum helpers and v1 auth for apps that talk to SQLite, MySQL, or Postgres through [SeaORM](https://www.sea-ql.org/SeaORM/).

This crate is the reusable core of the [rundtisch](https://github.com/danac/rundtisch) monorepo. A small demo website lives in `demo/`. Publishing to crates.io is planned; `publish` is currently `false`.

The library does not open a database and does not select a backend. Callers pass a `sea_orm::DatabaseConnection`. SeaORM is compiled with the sqlite, mysql, and postgres drivers together.

## What the crate owns

| Module | Role |
|--------|------|
| `app` | `AppState { db, email }` and `secret()` (`std::env::var`). The `storage` feature adds `objects`. |
| `email` | `EmailSender` trait for outbound mail (recovery links, etc.) |
| `auth` | Users, opaque sessions, invitations, recovery, passkeys, Argon2id, SeaORM entities and migrations |
| `storage` | Object store: `storage_objects`, content hashes, and a `FileStorage` port |

`auth` is a default feature. Opt-in mail transports:

| Feature | Type | Notes |
|---------|------|-------|
| `sendmail` | `SendmailEmailSender` | lettre builder + `sendmail -t` |
| `smtp` | `SmtpEmailSender` | lettre async STARTTLS (`tokio1-rustls` + `ring` + `webpki-roots`) |

SMTP env vars (all required except From): `AUTH_SMTP_URL` (host or `smtp://host[:port]`), `AUTH_SMTP_USERNAME`, `AUTH_SMTP_PASSWORD`, optional `AUTH_MAIL_FROM` (default `rundtisch@localhost`).

Object storage is optional. The demo keeps building without it. `AppState::new` installs a disabled backend when the `storage` feature is on; `with_storage` attaches a real one.

| Feature | Role |
|---------|------|
| `storage` | `storage_objects` migration (`rundtisch_migrations_storage`), `ObjectStore`, disabled backend |
| `storage-local` | Filesystem adapter (`STORAGE_BACKEND=local`) and HTTP delivery |
| `storage-memory` | In-memory adapter for tests |
| `storage-fetch` | HTTP body download used by catalog seed |
| `storage-http` | `Delivery::respond` (Range, 304, ETag). Pulled in by `storage-local` |

`stage` inserts a pending row and writes `{prefix/}{uuid}[.{ext}]` while hashing BLAKE3 and SHA-256. `commit` marks that row committed inside the caller's transaction. Reads return committed rows only. `delete` removes the row in the caller's transaction and the file after commit. `cleanup` sweeps stale pending rows and orphan files; do not call it from a request handler. `storage_key` is `varchar(512)` so the unique index fits MySQL's InnoDB limit. `STORAGE_BACKEND=s3` is rejected. The local root is `STORAGE_LOCAL_ROOT`, then `DATA_DIR`, then `/data`.

`cargo check --manifest-path crates/rundtisch/Cargo.toml --no-default-features` builds only `app` / `email` trait support.

`auth::Migrator` applies the auth migrations and records them in `rundtisch_migrations_auth`, separate from a host app's `seaql_migrations` table. The demo binary is `migrate`. After `Migrator::up`, that binary can upsert a verified Admin from `RUNDTISCH_BOOTSTRAP_ADMIN_*` secrets: insert when the email is absent, or reset the password on the existing user.

## Development

The crate is excluded from the repository workspace, so test it on its own lockfile:

```bash
cargo test --manifest-path crates/rundtisch/Cargo.toml
```

Handler tests use an in-memory SQLite connection and apply `auth::Migrator`.

## See also

- [Root README](../../README.md) — monorepo layout and local dev
- [Demo API](../../demo/api/README.md) — routes, `DATABASE_URL`, and the migrate binary
