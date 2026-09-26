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
    native_platform.rs    # Database::connect(DATABASE_URL)
    migrator.rs           # MigratorTrait → rundtisch::auth::migrations()
    listen.rs             # BIND_ADDR/PORT, default 0.0.0.0:8787
    static_files.rs       # optional SPA from /app/web or STATIC_DIR
    bin/native.rs         # listen; serve web/dist when the static dir exists
    bin/migrate.rs        # Migrator::up(&db, None)
```

## Endpoints

| Method | Path | Response |
|--------|------|----------|
| GET | `/api/health` | `{ "status": "ok", "headers": [...] }` |
| GET / POST | `/api/auth/users` | Playground list / create |
| PATCH / DELETE | `/api/auth/users/{public_id}` | Alias update / delete |
| POST | `/api/auth/register` | Creates user, returns `activation_token` |
| POST | `/api/auth/activate` | Email-verify JWT |
| POST | `/api/auth/login` | Access JWT + `session` cookie |
| POST | `/api/auth/refresh` | Rotate session cookie, new access JWT |
| POST | `/api/auth/logout` | Revoke session, clear cookie |
| GET | `/api/auth/me` | Bearer access JWT |

## Run

`DATABASE_URL` selects the backend (`sqlite://`, `mysql://`, or `postgres://`). Apply migrations before starting the server. The server does not migrate on startup.

```bash
export DATABASE_URL=sqlite://rundtisch.sqlite?mode=rwc
export AUTH_JWT_ACCESS_SECRET=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
export AUTH_JWT_VERIFY_SECRET=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
export AUTH_HASH_PEPPER=cccccccccccccccccccccccccccccccc

cargo run -p rundtisch-demo --bin migrate
cargo run -p rundtisch-demo --bin native
curl -i http://localhost:8787/api/health
```

`PORT` and `BIND_ADDR` override the listen address (Wasmer Edge uses `80` / `127.0.0.1`). When `/app/web` exists — the `[fs]` mount in `demo/wasmer.toml` — or `STATIC_DIR` points at `demo/web/dist`, the same process serves the built SPA. Leave both unset for Vite split-dev.

| Secret | Length |
|--------|--------|
| `AUTH_JWT_ACCESS_SECRET` | ≥ 32 bytes |
| `AUTH_JWT_VERIFY_SECRET` | ≥ 32 bytes |
| `AUTH_HASH_PEPPER` | exactly 32 bytes |

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
