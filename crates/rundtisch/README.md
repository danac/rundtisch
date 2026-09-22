# rundtisch

Thin platform-agnostic layer for [Axum](https://github.com/tokio-rs/axum) apps that deploy on **Cloudflare Workers** and as a **native container** without changing route or handler code.

This crate is the reusable core of the [rundtisch](https://github.com/danac/rundtisch) monorepo. A small demo website lives in `demo/` of that repository. Publishing to crates.io is planned; `publish` is currently `false`.

## What the crate owns

| Module | Role |
|--------|------|
| `traits` | `Platform` and database traits (`DatabaseExecutor`, `DbRecord`, migrations) |
| `app` | `AppState<P>` |
| `adapters::db` | SQLite / D1 adapters |
| `adapters::platform` | `NativePlatform`, `CloudflarePlatform` |
| `auth` | Auth models and schema migrations (WIP) |

Application crates define routes, handlers, and HTTP serving. They stay generic over `P: Platform`. Native `serve` lives in `demo/api/src/bin/native.rs`; the Workers `fetch` entry lives in `demo/worker`.

## Features

| Feature | Enables |
|---------|---------|
| *(none)* | Traits, `AppState`, auth models/migrations |
| `native` | SQLite adapter |
| `cloudflare` | Workers `Env` platform, D1 executor stub |

Default features are empty so a WASM build does not pull Tokio.

Row types are `#[derive(Serialize, Deserialize)]` structs (`DbRecord`). The same model is used on SQLite and D1; adapters coerce cells from the requested field type (see `traits::db::DbRecord`).

## Native

```rust
use rundtisch::adapters::platform::native::NativePlatform;
use rundtisch::AppState;

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
use rundtisch::adapters::platform::cloudflare::CloudflarePlatform;
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
cargo test -p rundtisch --features native
```

## See also

- [Root README](../../README.md) — monorepo layout, demo Worker, CI
- [Demo API](../../demo/api/README.md) — example routes using this crate
- [JWT / WASM / RNG port plan](doc/20260921-JWT_WASM_dependencies_and_RNG_port.md) — locked v1: jwt-compact HS256, default Clock/Random, `SecretStore` + `HASH_PEPPER`, same Argon2id on Worker and native, `auth_sessions`
