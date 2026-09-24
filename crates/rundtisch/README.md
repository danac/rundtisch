# rundtisch

Axum helpers and v1 auth for apps that talk to SQLite, MySQL, or Postgres through [SeaORM](https://www.sea-ql.org/SeaORM/).

This crate is the reusable core of the [rundtisch](https://github.com/danac/rundtisch) monorepo. A small demo website lives in `demo/`. Publishing to crates.io is planned; `publish` is currently `false`.

The library does not open a database and does not select a backend. Callers pass a `sea_orm::DatabaseConnection`. SeaORM is compiled with the sqlite, mysql, and postgres drivers together.

## What the crate owns

| Module | Role |
|--------|------|
| `app` | `AppState { db }` and `secret()` (`std::env::var`) |
| `auth` | Users, sessions, jwt-compact HS256, Argon2id, SeaORM entities and migrations |

`auth::migrations()` returns the `MigrationTrait` list. The application implements `MigratorTrait` and applies it (the demo binary is `migrate`).

## Development

From the repository root:

```bash
cargo test -p rundtisch
```

Handler tests use an in-memory SQLite connection and apply `auth::migrations()`.

## See also

- [Root README](../../README.md) — monorepo layout and local dev
- [Demo API](../../demo/api/README.md) — routes, `DATABASE_URL`, and the migrate binary
