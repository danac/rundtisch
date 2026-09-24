# Drop Cloudflare D1 and move persistence to SeaORM

Status: **plan only**. This document does not change runtime code. Database engine, migration apply in CI, and Wasmer packaging are decided below and are not open questions.

Synced to GitHub `main` at `0b13339` (`Implement v1 JWT auth: ports, Argon2id, sessions, demo login (#23)`).

## Why

Cloudflare Workers and D1 are no longer a deployment target. The custom database port exists so one SeaQuery statement can run on SQLite (`sqlx`) and on D1 (JSON rows, type-directed decode). That port, the D1 adapter, the Worker secret store, and the `Platform` trait are the cost of that split.

[wasmer-sqlx-demo](https://github.com/danac/wasmer-sqlx-demo) already runs sqlx 0.9 + SeaORM 2.0.3 + Tokio on Wasmer Edge against managed MySQL, including a `bool` column that round-trips as JSON `true` / `false`. Datetime is a first-class SeaORM/sqlx MySQL mapping (`DATETIME` / `TIMESTAMP`), not a D1 JSON-string problem.

Auth behavior stays: users, sessions, Argon2id, jwt-compact HS256, the same HTTP routes. Only the host abstractions and the persistence layer change.

## Target shape

```text
demo/api/src/bin/native.rs
    │
    ▼
native_platform.rs
    Database::connect(DATABASE_URL) -> sea_orm::DatabaseConnection
    (sqlite, mysql, or postgres — chosen only here)
    │
    ▼
AppState { db }                         secret(name) -> std::env::var
    │
    ▼
Axum handlers (no Platform type parameter)
    │
    ▼
SeaORM entities + auth::migrations()    demo Migrator lists those migrations
                                        demo bin `migrate` calls Migrator::up
```

The library never selects a backend. `sea_orm::DatabaseConnection` already wraps whichever sqlx pool it was built from. `rundtisch` compiles SeaORM with the sqlite, mysql, and postgres drivers together, and it does **not** grow `sqlite` / `mysql` / `postgres` feature flags. The `d1` and `sqlite` features that exist today are removed. The demo’s `native` and `cloudflare` features are removed with them.

`native.rs` calls into `native_platform.rs` to open the connection and build `AppState`. A `sqlite://`, `mysql://`, or `postgres://` `DATABASE_URL` is the only switch. Handlers, entities, and queries do not branch on the backend.

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

The connection is opened in the demo, not behind a trait. `native_platform.rs` stops implementing `Platform` and becomes the place that builds the connection:

```rust
// demo/api/src/native_platform.rs
pub async fn connect() -> Result<DatabaseConnection, DbErr> {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    sea_orm::Database::connect(&url).await
}
```

`native.rs` calls `connect()`, puts the connection in `AppState`, and serves. `SQLITE_PATH` goes away; SQLite is just another `DATABASE_URL` (`sqlite://…` or `sqlite::memory:` in tests). There is no `sqlite` / `mysql` / `postgres` Cargo feature on `rundtisch` or `rundtisch-demo`.

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
- `demo/worker/` entirely (`CloudflarePlatform`, `#[event(fetch)]`)

`demo/api/src/native_platform.rs` stays. It no longer implements `Platform`. It only opens the `DatabaseConnection` (step 1).

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
| timestamps as RFC 3339 **strings** (`datetime_to_rfc3339` on insert) | real datetime columns via SeaORM `with-time` and `time::OffsetDateTime`. The migration uses SeaQuery schema helpers (`timestamp`, `date_time`) so the same Rust migration maps to each backend. No backend-specific SQL |
| `public_id: Uuid` inserted as text | `Uuid` column through SeaORM, which picks a backend-specific storage type. JSON still emits the hyphenated string on the response DTO |
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

Follow [Writing Migration](https://www.sea-ql.org/SeaORM/docs/migration/writing-migration/). Migrations are Rust files from here on. Each one is a `MigrationTrait` with `up` and `down`, named `mYYYYMMDD_HHMMSS_<name>.rs`. DDL goes through `SchemaManager` and the SeaQuery helpers (`pk_auto`, `string`, `timestamp`, …) so one file runs on SQLite, MySQL, and Postgres. Raw SQL is not used for this migration; the docs note that raw SQL drops multi-backend compatibility.

`auth` owns the migration files and exports the list. It does **not** implement `MigratorTrait`. The demo app does, and its list is the auth list (later modules append their own):

```rust
// crates/rundtisch/src/auth/migrations/mod.rs
pub fn migrations() -> Vec<Box<dyn MigrationTrait>> {
    vec![Box::new(m20260924_000001_create_auth_tables::Migration)]
}

// demo/api/src/migrator.rs
pub struct Migrator;

#[async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        rundtisch::auth::migrations()
    }
}
```

The first migration creates `auth_users` and `auth_sessions` in `up`, and drops `auth_sessions` then `auth_users` in `down`. `DeriveMigrationName` supplies the name SeaORM records in its migration table.

Replace `AuthMigration001` (SeaQuery `Table::create` rendered to SQLite and snapshotted into `demo/migrations/001_auth_create_users_and_token_tables.sql`).

Delete:

- `demo/api/src/bin/generate_auth_migrations.rs`
- `demo/migrations/`
- the test `demo_sqlite_snapshot_matches_up_sql`

Add `demo/api/src/bin/migrate.rs`, a new executable on the demo crate. It is the pre-deployment command. It does not run inside `native`. Per [Running Migration](https://www.sea-ql.org/SeaORM/docs/migration/running-migration/), it applies pending migrations programmatically:

```rust
// demo/api/src/bin/migrate.rs
#[tokio::main]
async fn main() -> Result<(), sea_orm::DbErr> {
    let db = rundtisch_demo::native_platform::connect().await?;
    Migrator::up(&db, None).await?;
    Ok(())
}
```

`Migrator::up(db, None)` applies every pending migration. The same `DATABASE_URL` the server uses is the database this binary migrates. `cargo run -p rundtisch-demo --bin migrate` is the command a later pre-deploy step will call. Wiring that step into CI or a host is not part of this change. The native server still does not migrate on startup. Tests may call `Migrator::up` against whatever `DATABASE_URL` they were given. A CI MySQL service is also later.

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

Historical notes under `crates/rundtisch/doc/` stay, including `20260708-150107-RustCloudflareWorkerAuthenticationDesign.md`, `20260921-JWT_WASM_dependencies_and_RNG_port.md`, and the D1 migration write-ups. They are design history, not build instructions.

Wasmer `app.yaml` / `cargo wasix` is not part of this refactor.

## Step 7 — Dependencies

### Remove from the workspace / `rundtisch`

| Dependency | Why it is unused after the steps above |
| --- | --- |
| `sea-query` as a direct workspace dependency | DML no longer builds SeaQuery statements. Migration DDL uses SeaQuery through `sea-orm-migration`, which re-exports it |
| `worker`, `worker-macros` | D1 and the Worker entry |
| `js-sys`, `wasm-bindgen` | `d1` feature (`getrandom/wasm_js`, `time/wasm-bindgen`) |
| `tower-service` | Only `demo/worker` calls `Service::call` |
| features `d1`, `sqlite`, and any future `mysql` / `postgres` flags | All three sqlx drivers are always compiled in. The entry point picks the backend with `DATABASE_URL` |
| `sqlx` as a direct rundtisch dependency | Pulled in by `sea-orm`’s `sqlx-*` features |

`getrandom` stays if UUIDv4 / session tokens call it directly. `uuid` can enable the `v4` feature instead and drop the hand-rolled `public_id_from_bytes` — only if tests that lock the version nibble still pass. Default: keep the existing UUIDv4 helper and call `getrandom::fill`.

### Add

```toml
sea-orm = { version = "2.0", default-features = false, features = [
  "macros",
  "runtime-tokio-rustls",
  "with-time",
  "sqlx-sqlite",
  "sqlx-mysql",
  "sqlx-postgres",
] }
sea-orm-migration = { version = "2.0", default-features = false, features = [
  "runtime-tokio-rustls",
  "sqlx-sqlite",
  "sqlx-mysql",
  "sqlx-postgres",
] }
```

`sea-orm-migration` is what provides `MigrationTrait`, `MigratorTrait`, `SchemaManager`, and `DeriveMigrationName`. The demo depends on it for `Migrator`. The auth crate depends on it for the migration files and the exported list.

Tokio is already a demo dependency. The library needs it for SeaORM’s runtime feature; make `tokio` a normal dependency of `rundtisch` with `macros` and `rt-multi-thread` (tests and the connection). The demo feature flag `native` disappears: the demo crate always runs the native server, and it has no per-database features either.

`time` stays (formatting, parsing, serde, rfc3339). Do not add `time/wasm-bindgen`.

`chrono` stays only while `jwt-compact` needs it. Do not use it for columns if entities use `time`.

Unchanged: `axum`, `serde`, `serde_json`, `email_address`, `argon2`, `hmac`, `sha2`, `subtle`, `base64`, `zeroize`, `jwt-compact`.

After the edit, `cargo metadata` / a clean `cargo build` should show no `worker`, `sea-query`, or `wasm-bindgen` in the `rundtisch` or `rundtisch-demo` graphs.

## Suggested order of work

The steps are one PR sequence, not seven releases. Compile breaks until 1–5 land together.

1. Add SeaORM (all three sqlx drivers), the auth migration files, `auth::migrations()`, the demo `Migrator`, and the `migrate` binary that calls `Migrator::up(&db, None)`.
2. Switch `auth/queries.rs` and handlers to `DatabaseConnection`. Delete trait bounds.
3. Collapse `AppState` and delete traits/adapters. Leave `native_platform.rs` as the `Database::connect` entry.
4. Delete `demo/worker`, Wrangler, the `cloudflare` feature, and the CI deploy job. Do not add a MySQL service to CI.
5. Dependency cleanup and README/AGENTS rewrite.
6. `cargo test` and a manual pass of register → activate → login → `GET /api/auth/me` → logout against the demo API.

## Out of scope

- New product features, GraphQL, or a shop schema.
- Porting D1 data. The schema stays empty until someone runs the `migrate` binary.
- Publishing to crates.io (`publish` stays `false`).
- Wasmer deploy manifests in this repository.
- Calling `migrate` from CI or a deploy pipeline, and adding a MySQL (or Postgres) service to CI. The binary itself is in scope.

## Decisions

- **All three sqlx backends.** The library takes a `DatabaseConnection`. `native_platform.rs` is the only place that calls `Database::connect`. No `sqlite` / `mysql` / `postgres` feature flags.
- **SeaORM migration files.** `auth::migrations()` returns the `Vec<Box<dyn MigrationTrait>>`. The demo implements `MigratorTrait` and returns that list. DDL uses `SchemaManager`, not raw SQL and not the old SQL snapshot.
- **`migrate` binary.** `demo/api/src/bin/migrate.rs` opens the connection and calls `Migrator::up(&db, None)`. That is the pre-deployment command. The server does not migrate on startup. Invoking the binary from CI or a deploy pipeline, and a CI database container, come later.
- **Docs.** Cloudflare design notes under `crates/rundtisch/doc/` stay. README, `AGENTS.md`, and the demo READMEs are rewritten.
- **Wasmer.** Packaging is a later change.

## Risks

- **Schema helpers must stay portable.** A migration that uses MySQL-only or Postgres-only DDL (`create_type`, raw `AUTO_INCREMENT`) will not run on the other drivers. Stick to `SchemaManager` helpers.
- **SeaORM’s `Uuid` storage differs by backend** (`binary(16)` on MySQL, `uuid` on Postgres, text on SQLite). Response DTOs still serialize the hyphenated string. Do not compare raw column bytes across engines.
- **Datetime storage differs by backend** (`TIMESTAMP` vs `DATETIME` vs text). Entities use `time::OffsetDateTime` and let SeaORM encode them. Do not keep writing RFC 3339 strings by hand.
- **Env secrets in tests.** `MapSecretStore` could be constructed per test. `std::env::var` cannot. Auth handler tests that need `AUTH_HASH_PEPPER` must set and restore env vars without overlapping `cargo test` threads, or those tests become single-threaded.
- **Argon2id on every login** still dominates CPU. Building the hasher from the pepper per request is cheap next to that and matches “`AppState` holds only the connection”.
- **CI** loses the Workers preview. The workflow must not keep failing on `wasm32-unknown-unknown` or missing `CLOUDFLARE_API_TOKEN`. Tests that need a live database keep using SQLite in-memory until a later CI change adds MySQL or Postgres.
- **Atomic migrations.** SeaORM runs Postgres migrations inside a transaction. MySQL and SQLite do not. `up` should still be safe to retry (`if_not_exists`, or `has_column` before `ALTER`).
