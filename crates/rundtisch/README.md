# rundtisch

Axum helpers and v1 auth for apps that talk to SQLite, MySQL, or Postgres through [SeaORM](https://www.sea-ql.org/SeaORM/).

This crate is the reusable core of the [rundtisch](https://github.com/danac/rundtisch) monorepo. A small demo website lives in `demo/`. Publishing to crates.io is planned; `publish` is currently `false`.

The library does not open a database and does not select a backend. Callers pass a `sea_orm::DatabaseConnection`. SeaORM is compiled with the sqlite, mysql, and postgres drivers together.

## What the crate owns

| Module | Role |
|--------|------|
| `app` | `AppState { db, email }` and `secret()` (`std::env::var`) |
| `email` | `EmailSender` trait for outbound mail (recovery links, etc.) |
| `auth` | Users, opaque sessions, invitations, recovery, passkeys, Argon2id, SeaORM entities and migrations |

`auth` is a default feature. Opt-in mail transports:

| Feature | Type | Notes |
|---------|------|-------|
| `sendmail` | `SendmailEmailSender` | lettre builder + `sendmail -t` |
| `smtp` | `SmtpEmailSender` | lettre async STARTTLS (`tokio1-rustls` + `ring` + `webpki-roots`) |

SMTP env vars (all required except From): `AUTH_SMTP_URL` (host or `smtp://host[:port]`), `AUTH_SMTP_USERNAME`, `AUTH_SMTP_PASSWORD`, optional `AUTH_MAIL_FROM` (default `rundtisch@localhost`).

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
