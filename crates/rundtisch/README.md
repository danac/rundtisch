# rundtisch

Thin platform-agnostic layer for [Axum](https://github.com/tokio-rs/axum) apps that deploy on **Cloudflare Workers** and as a **native container** without changing route or handler code.

This crate is the reusable core of the [rundtisch](https://github.com/danac/rundtisch) monorepo. A small demo website lives in `demo/` of that repository. Publishing to crates.io is planned; `publish` is currently `false`.

## What the crate owns

| Module | Role |
|--------|------|
| `traits` | `Platform` (`Database` + `SecretStore`; default `random()` / `clock()`), `DatabaseExecutor`, migrations |
| `app` | `AppState<P>` (`database`, `secrets`, `random`, `clock`, `password_hasher`) |
| `adapters` | SQLite / D1, `OsRandom` / `WorkerRandom`, `SystemClock`, `EnvSecretStore` / `WorkerSecretStore` |
| `auth` | Users + sessions, jwt-compact HS256, Argon2id, HMAC session cookies, register/login handlers |

Application crates own `Platform` impls (`demo/api` native, `demo/worker` Cloudflare). They stay generic over `P: Platform`. Native `serve` lives in `demo/api/src/bin/native.rs`; the Workers `fetch` entry lives in `demo/worker`.

## Features

| Feature | Enables |
|---------|---------|
| *(none)* | Traits, `AppState`, auth (JWT / Argon2id / sessions) |
| `sqlite` | SQLite adapter (`sqlx`) |
| `d1` | Workers D1 executor, `WorkerRandom`, `WorkerSecretStore`, `getrandom/wasm_js` |

Default features are empty so a WASM build does not pull Tokio.

Row types are `#[derive(Serialize, Deserialize)]` structs (`DbRecord`). The same model is used on SQLite and D1; adapters coerce cells from the requested field type (see `traits::db::DbRecord`).

## Native

```rust
use rundtisch::adapters::db::sqlite::SqliteExecutor;
use rundtisch::adapters::secrets::EnvSecretStore;
use rundtisch::{AppState, Platform};

impl Platform for NativePlatform {
    type Database = SqliteExecutor;
    type SecretStore = EnvSecretStore;
    fn database(&self) -> Arc<Self::Database> { self.db.clone() }
    fn secrets(&self) -> Arc<Self::SecretStore> { self.secrets.clone() }
}

#[tokio::main]
async fn main() {
    let platform = NativePlatform::new("rundtisch.sqlite").await;
    let state = AppState::from_platform(&platform);
    serve(build_router(state)).await; // application-owned
}
```

## Cloudflare Workers

Keep a thin `cdylib` with `#[event(fetch)]`. Construct `CloudflarePlatform` from the Worker `env` and dispatch through your Axum router:

```rust
use rundtisch::AppState;
use tower_service::Service;
use worker::{Context, Env, HttpRequest};
use worker_macros::event;

#[event(fetch)]
async fn fetch(
    req: HttpRequest,
    env: Env,
    _ctx: Context,
) -> worker::Result<axum::http::Response<axum::body::Body>> {
    let platform = CloudflarePlatform::new(env, "D1_BINDING");
    let mut router = build_router(AppState::from_platform(&platform));
    Ok(router.call(req).await?)
}
```

`worker-build` must run against the cdylib crate (see `demo/worker` in this repo).

## Development

From the repository root:

```bash
cargo test -p rundtisch --features sqlite
cargo check -p rundtisch --features d1 --target wasm32-unknown-unknown
```

## See also

- [Root README](../../README.md) — monorepo layout, demo Worker, CI
- [Demo API](../../demo/api/README.md) — example routes using this crate
- [JWT / WASM / RNG port plan](doc/20260921-JWT_WASM_dependencies_and_RNG_port.md) — locked v1: jwt-compact HS256, default Clock/Random, `SecretStore` + `HASH_PEPPER`, same Argon2id on Worker and native, `auth_sessions`, opaque `public_id` UUIDv4
