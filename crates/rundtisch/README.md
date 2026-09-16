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
| `runtime::native` | `serve(router)` |
| `runtime::cloudflare` | `handle_fetch(req, env, build)` |

Application crates define routes and handlers only. They stay generic over `P: Platform`.

## Features

| Feature | Enables |
|---------|---------|
| *(none)* | Traits, `AppState`, auth models/migrations |
| `native` | Tokio server, SQLite adapter |
| `cloudflare` | workers-rs fetch helper, D1 adapter |

Default features are empty so a WASM build does not pull Tokio.

Row types are `#[derive(Serialize, Deserialize)]` structs (`DbRecord`). The same model is used on SQLite and D1; adapters coerce cells from the requested field type (see `traits::db::DbRecord`).

## Native

```rust
use rundtisch::adapters::platform::native::NativePlatform;
use rundtisch::runtime::native::serve;
use rundtisch::AppState;

#[tokio::main]
async fn main() {
    let platform = NativePlatform::new();
    let state = AppState::from_platform(&platform);
    serve(build_router(state)).await;
}
```

## Cloudflare Workers

Keep a thin `cdylib` with `#[event(fetch)]` and delegate to the crate:

```rust
use rundtisch::runtime::cloudflare::handle_fetch;
use worker::{Context, Env, HttpRequest};
use worker_macros::event;

#[event(fetch)]
async fn fetch(
    req: HttpRequest,
    env: Env,
    _ctx: Context,
) -> worker::Result<axum::http::Response<axum::body::Body>> {
    handle_fetch(req, env, |platform| {
        build_router(rundtisch::AppState::from_platform(platform))
    })
    .await
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
