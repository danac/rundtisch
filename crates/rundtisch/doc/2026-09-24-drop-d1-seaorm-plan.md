# Drop Cloudflare D1 and move persistence to SeaORM

Status: **plan only**. This document does not change runtime code. Implementation starts only after the open questions at the end are answered.

Synced to GitHub `main` at `0b13339` (`Implement v1 JWT auth: ports, Argon2id, sessions, demo login (#23)`).

## Why

Cloudflare Workers and D1 are no longer a deployment target. The custom database port exists so one SeaQuery statement can run on SQLite (`sqlx`) and on D1 (JSON rows, type-directed decode). That port, the D1 adapter, the Worker secret store, and the `Platform` trait are the cost of that split.

[wasmer-sqlx-demo](https://github.com/danac/wasmer-sqlx-demo) already runs sqlx 0.9 + SeaORM 2.0.3 + Tokio on Wasmer Edge against managed MySQL, including a `bool` column that round-trips as JSON `true` / `false`. Datetime is a first-class SeaORM/sqlx MySQL mapping (`DATETIME` / `TIMESTAMP`), not a D1 JSON-string problem.

Auth behavior stays: users, sessions, Argon2id, jwt-compact HS256, the same HTTP routes. Only the host abstractions and the persistence layer change.

## Target shape

```text
native binary (demo/api/src/bin/native.rs)
    │
    ▼
Database::connect(DATABASE_URL) -> sea_orm::DatabaseConnection
    │
    ▼
AppState { db }
    │  secret(name) -> std::env::var
    ▼
Axum handlers (no Platform type parameter)
    │
    ▼
SeaORM entities in auth/  (no SeaQuery, no DatabaseExecutor)
```

`AppState` holds the connection and nothing else. Secrets are one method:

```rust
impl AppState {
    pub fn secret(&self, name: &str) -> Result<String, SecretError> {
        std::env::var(name).map_err(|_| SecretError::NotFound)
    }
}
```

Names stay `AUTH_JWT_ACCESS_SECRET`, `AUTH_JWT_VERIFY_SECRET`, and `AUTH_HASH_PEPPER`.

These leave `AppState` because they are not host ports anymore:

| Today | After |
| --- | --- |
| `state.clock.now_utc()` | `time::OffsetDateTime::now_utc()` inside auth |
| `state.random.fill_bytes` | `getrandom::fill` inside auth (UUIDv4, session tokens) |
| `state.password_hasher` | `Argon2idHasher` built in the register/login path from `state.secret(AUTH_HASH_PEPPER)` |
| `state.database.fetch_*` / `.insert` / `.execute` | `user::Entity::find()...` on `&state.db` |

`PasswordHasher`, `Clock`, and `RandomSource` traits go away with the platform. Tests that injected `MapSecretStore`, `TestClock`, or a fake RNG call the same functions with `std::env` set for that test, or test the pure helpers (`public_id_from_bytes`, JWT verify) without a store. Process-global env in parallel tests is called out under risks.

## Step 1 — Database

Remove the database port and both adapters. Queries go through SeaORM on `sea_orm::DatabaseConnection`.

Delete:

- `crates/rundtisch/src/traits/db.rs` (`DatabaseExecutor`, `DbRecord`, `Value`, `Dialect`, `Migration`, SeaQuery render helpers)
- `crates/rundtisch/src/adapters/db.rs`
- `crates/rundtisch/src/adapters/db/sqlite.rs`
- `crates/rundtisch/src/adapters/db/d1.rs`
- `crates/rundtisch/src/adapters/db/row_de.rs`

Stop building SQL with SeaQuery. `auth/queries.rs` today returns `SelectStatement` / `InsertStatement` / `UpdateStatement` / `DeleteStatement` (`user_list_query`, `user_insert_query`, `session_rotate_query`, …). Those functions become async functions that take `&DatabaseConnection` and return models or `DbErr`.

The connection is opened in the demo binary, not behind a trait:

```rust
let db = sea_orm::Database::connect(&database_url).await?;
let state = AppState { db };
```

`SQLITE_PATH` and `NativePlatform` go away with this step (wiring finishes in steps 3 and 6).

## Step 2 — Secrets

Remove secret ports and adapters.

Delete:

- `crates/rundtisch/src/traits/secrets.rs`
- `crates/rundtisch/src/adapters/secrets.rs` (`EnvSecretStore`, `MapSecretStore`, `WorkerSecretStore`)

Keep a small `SecretError` next to `AppState::secret` (or on the auth error type) so handlers can still map a missing key to 500. `WorkerSecretStore` and `demo/.dev.vars.example` exist only for Wrangler; both go.

Handlers that call `secret_bytes(&*state.secrets, AUTH_JWT_ACCESS_SECRET, 32)` call `state.secret(AUTH_JWT_ACCESS_SECRET)` and check the length there.

## Step 3 — Platform

The hexagon is the `Platform` trait plus clock and random ports that existed so Workers and native could share handlers.

Delete:

- `crates/rundtisch/src/traits.rs` (`Platform`)
- `crates/rundtisch/src/traits/clock.rs`, `adapters/clock.rs` (`SystemClock` is `OffsetDateTime::now_utc`)
- `crates/rundtisch/src/traits/random.rs`, `adapters/random.rs` (`OsRandom`, `WorkerRandom`)
- `crates/rundtisch/src/adapters.rs` once it has no modules left
- `demo/api/src/native_platform.rs`
- `demo/worker/` entirely (`CloudflarePlatform`, `#[event(fetch)]`)

`lib.rs` stops exporting `Platform`, `Clock`, `RandomSource`, and `SecretStore`.

## Step 4 — AppState

`AppState<P: Platform>` becomes a concrete struct. Every handler signature drops `<P: Platform>`.

Today, all of these are generic:

- `crates/rundtisch/src/auth/handlers.rs` (`list_users`, `create_user`, `register`, `login`, `refresh`, `logout`, `me`, …)
- `crates/rundtisch/src/app.rs`
- `demo/api/src/routes.rs` (`build_router<P: Platform>`)
- `demo/api/src/handlers.rs` (`health<P: Platform>`)

After:

```rust
pub struct AppState {
    pub db: sea_orm::DatabaseConnection,
}

pub fn build_router(state: AppState) -> Router { /* ... */ }
```

`AppState` is `Clone` because `DatabaseConnection` is. `from_platform` is removed.

Handler tests in `auth/handlers.rs` that build `AppState` by filling trait objects are rewritten to construct `AppState { db }` against the test database (step 5) and to read secrets from the environment.

## Step 5 — Auth

Rewrite entities, queries, and the initial migration in `auth/` to match steps 1–4. HTTP shapes stay.

### Entities

Replace SeaQuery `Iden` enums and serde row structs with SeaORM models.

| Current | SeaORM |
| --- | --- |
| `UserTable` / `User` / `NewUser` in `auth/models.rs` | `auth/entities/user.rs` `DeriveEntityModel`, table `auth_users` |
| `SessionTable` / `Session` | `auth/entities/session.rs`, table `auth_sessions` |
| `Role` as a Rust enum stored via `.as_str()` | `DeriveActiveEnum` stored as a string (`User`, `Admin`), not a MySQL native ENUM, so the column stays a normal `VARCHAR` |
| timestamps as RFC 3339 **strings** (`datetime_to_rfc3339` on insert) | real datetime columns. SeaORM `with-time`, fields `time::OffsetDateTime`. This is the D1 workaround; sqlx MySQL encodes `OffsetDateTime` as `TIMESTAMP` |
| `public_id: Uuid` inserted as text | `Uuid` column. On MySQL, SeaORM’s default uuid mapping is `binary(16)`. JSON still emits the hyphenated string through serde on the response DTO |
| `email: EmailAddress` | store `String`, parse `EmailAddress` at the handler boundary (SeaORM has no `EmailAddress` column type) |
| `password_hash` skipped in JSON | not a column attribute problem: response DTOs omit it, same as `#[serde(skip_serializing)]` today |

`NewUser` / `UpdateUserAlias` remain request DTOs in `models.rs` (or `dto.rs`). They are not entities.

`User` ↔ `Session` is a SeaORM relation (`has_many` / `belongs_to`, `on_delete = Cascade`) matching `fk_session_user_id`.

### Queries

`auth/queries.rs` becomes thin wrappers or disappears into handlers. Illustrative replacements:

- `user_list_query` + `fetch_all` → `user::Entity::find().order_by_asc(user::Column::Id).all(db)`
- `user_get_query(public_id)` → `user::Entity::find().filter(user::Column::PublicId.eq(public_id)).one(db)`
- `user_insert_query` + `insert` returning `i64` → `user::ActiveModel { .. }.insert(db)` and read `model.id`
- `session_get_by_token_hash_query` → filter on `token_hash`
- `session_rotate_query` / `session_revoke_query` → `ActiveModel` update

Unique violations (`email`, `public_id`, `token_hash`) map from `sea_orm::DbErr` / sqlx instead of `traits::db::Error::Conflict`. Handlers already turn conflicts into HTTP 409; that mapping moves to `auth/error.rs`.

Integer `id` stays the foreign key. Public URLs keep `public_id`.

### Migration

Replace `AuthMigration001` (SeaQuery `Table::create`, rendered to SQLite, snapshotted into `demo/migrations/001_auth_create_users_and_token_tables.sql`).

Use one SeaORM migration in `auth/migrations/` that creates `auth_users` and `auth_sessions` with the column types above (`if_not_exists` is not a substitute for a versioned migration once the schema changes again). `down` drops `auth_sessions` then `auth_users`.

Delete:

- `demo/api/src/bin/generate_auth_migrations.rs`
- `demo/migrations/`
- the test `demo_sqlite_snapshot_matches_up_sql`

How the migration is applied is an open question (below). The code that generates dialect-specific SQL for Wrangler D1 does not stay.

### What does not get rewritten

JWT (`auth/jwt.rs`), Argon2id (`auth/password.rs`), cookie/session helpers (`auth/session.rs`), TTLs (`auth/config.rs`), and the route table. They lose trait parameters and call `state.db` / `state.secret` / `OffsetDateTime::now_utc` / `getrandom::fill`.

## Step 6 — Deployments

Remove the Cloudflare demo and every operational mention of it.

Delete:

- `demo/worker/` (crate, `Cargo.toml`, `src/lib.rs`, `src/platform.rs`)
- `demo/wrangler.jsonc`, `demo/wrangler.dev.jsonc`
- `demo/.dev.vars.example`
- workspace member `demo/worker` in the root `Cargo.toml`
- `demo/api` feature `cloudflare` (`rundtisch/d1`)
- `.cursor/skills/cloudflare-preview-pr/` (it tells agents to put a Workers preview URL on PRs)

`demo/package.json` today is `concurrently` + `wrangler dev`. Replace `npm run dev` with Vite plus the native API, and drop the `wrangler` dependency. Point `demo/web/vite.config.ts` at the native listen port (the binary binds `0.0.0.0:8787` today; the README says 8080 — pick one port in the doc rewrite and make the proxy match).

GitHub Actions (`.github/workflows/deploy.yml`):

- Remove `wasm32-unknown-unknown`, `libssl-dev` / `pkg-config` for `worker-build`, `cargo check -p rundtisch --features d1`, and both `cloudflare/wrangler-action` steps.
- Remove `environment: Cloudflare Workers` and the `CLOUDFLARE_*` secrets.
- Keep a test job: `cargo test` for `rundtisch` and `rundtisch-demo`. Frontend `npm ci` + `npm run build` can stay as a non-deploy check.

Docs to rewrite so they describe a native Axum demo only:

- `README.md`
- `AGENTS.md`
- `crates/rundtisch/README.md`
- `demo/api/README.md`
- `demo/web/README.md`

Crate description in `crates/rundtisch/Cargo.toml` (“hexagonal architecture”) is updated in step 7.

Historical notes under `crates/rundtisch/doc/` (`20260708-150107-RustCloudflareWorkerAuthenticationDesign.md`, `20260921-JWT_WASM_dependencies_and_RNG_port.md`, D1 migration write-ups) are **not** scrubbed in this plan. They are design history, not build instructions. Deleting them is an open question.

Wasmer `app.yaml` / `cargo wasix` is **not** part of this refactor. `wasmer-sqlx-demo` remains the Edge packaging reference. This repo’s runnable app after the change is the native binary.

## Step 7 — Dependencies

### Remove from the workspace / `rundtisch`

| Dependency | Why it is unused after the steps above |
| --- | --- |
| `sea-query` | Only the database port and auth query builders use it |
| `worker`, `worker-macros` | D1 and the Worker entry |
| `js-sys`, `wasm-bindgen` | `d1` feature (`getrandom/wasm_js`, `time/wasm-bindgen`) |
| `tower-service` | Only `demo/worker` calls `Service::call` |
| features `d1`, `sqlite` | Replaced by the SeaORM backend feature (question 1) |
| `sqlx` as a direct rundtisch dependency | Pulled in by `sea-orm`’s `sqlx-*` feature. Keep an explicit `sqlx` dep only if tests still open a pool themselves |

`getrandom` stays if UUIDv4 / session tokens call it directly. `uuid` can enable the `v4` feature instead and drop the hand-rolled `public_id_from_bytes` — only if tests that lock the version nibble still pass. Default: keep the existing UUIDv4 helper and call `getrandom::fill`.

### Add

```toml
sea-orm = { version = "2.0", default-features = false, features = [
  "macros",
  "runtime-tokio-rustls",
  "with-time",
  # "sqlx-mysql" or "sqlx-sqlite" — question 1
] }
```

Tokio is already a demo dependency. The library needs it for SeaORM’s runtime feature; make `tokio` a normal dependency of `rundtisch` with `macros` and `rt-multi-thread` (tests and the connection). The demo feature flag `native` can disappear: the demo crate always runs the native server.

`time` stays (formatting, parsing, serde, rfc3339). Do not add `time/wasm-bindgen`.

`chrono` stays only while `jwt-compact` needs it. Do not use it for columns if entities use `time`.

Unchanged: `axum`, `serde`, `serde_json`, `email_address`, `argon2`, `hmac`, `sha2`, `subtle`, `base64`, `zeroize`, `jwt-compact`.

After the edit, `cargo metadata` / a clean `cargo build` should show no `worker`, `sea-query`, or `wasm-bindgen` in the `rundtisch` or `rundtisch-demo` graphs.

## Suggested order of work

The steps are one PR sequence, not seven releases. Compile breaks until 1–5 land together.

1. Add SeaORM and the new entities + migration beside the old code; get migration tests green on the chosen backend.
2. Switch `auth/queries.rs` and handlers to `DatabaseConnection`. Delete trait bounds.
3. Collapse `AppState` and delete traits/adapters/platforms.
4. Point `demo/api` at `Database::connect`. Delete `demo/worker`, Wrangler, the `cloudflare` feature, and CI deploy.
5. Dependency cleanup and README/AGENTS rewrite.
6. `cargo test` and a manual pass of register → activate → login → `GET /api/auth/me` → logout against the demo API.

## Out of scope

- New product features, GraphQL, or a shop schema.
- Porting D1 data. The next schema is empty and applied by the new migration.
- Publishing to crates.io (`publish` stays `false`).
- Wasmer deploy manifests in this repository.

## Risks

- **Backend choice changes types.** SQLite SeaORM datetime is often TEXT; MySQL `TIMESTAMP` is binary. Picking both backends recreates the dialect split this refactor removes. One backend only.
- **`binary(16)` UUIDs** on MySQL are not the hyphenated strings stored today. Response DTOs must still serialize `public_id` as a string. A `CHAR(36)` column is the alternative if we want the column readable in the SQL shell.
- **Env secrets in tests.** `MapSecretStore` could be constructed per test. `std::env::var` cannot. Auth handler tests that need `AUTH_HASH_PEPPER` must set and restore env vars without overlapping `cargo test` threads, or those tests become single-threaded.
- **Argon2id on every login** still dominates CPU. Building the hasher from the pepper per request is cheap next to that and matches “`AppState` holds only the connection”.
- **CI** loses the Workers preview. The workflow must not keep failing on `wasm32-unknown-unknown` or missing `CLOUDFLARE_API_TOKEN`.

## Open questions

1. **Which database?** Recommendation: **MySQL only** (`sqlx-mysql`), `DATABASE_URL=mysql://…`, because that is the engine already proven with SeaORM on Wasmer and it is the datetime/bool behavior you want. The alternative is **SQLite only**, which keeps in-memory tests and today’s native file (`SQLITE_PATH`) but is not the Wasmer engine. This plan does not support compiling both.
2. **When does the migration run?** Recommendation: the native binary runs the SeaORM migrator once before `axum::serve`, so `cargo run` on an empty database is enough. Alternative: a `migrate` subcommand only, and the server assumes the schema already exists.
3. **CI database.** If the answer to (1) is MySQL, CI needs a MySQL service container (or the job will skip integration tests). If SQLite, `sqlite::memory:` stays and CI does not.
4. **Historical `crates/rundtisch/doc/*` Cloudflare notes.** Recommendation: leave them. Say if they should be deleted in the implementation PR.
5. **Wasmer packaging.** Recommendation: not in the implementation PR. Say if `app.yaml` / `cargo wasix` should be added in the same change.
