# JWT on Cloudflare Workers: crate choice, crypto/RNG, and hexagonal ports

**Date:** 2026-09-21 (decisions locked 2026-09-22)  
**Status:** decided plan — JWT/ports not implemented yet; schema in `001` updated to `auth_sessions` + unique `public_id`  
**Related:** [auth-flow-diagram.md](./auth-flow-diagram.md), [20260708-150107-RustCloudflareWorkerAuthenticationDesign.md](./20260708-150107-RustCloudflareWorkerAuthenticationDesign.md)

This note records the crate survey, WASM probes, and the **locked v1 auth design**. The July 2026 snippet (`jsonwebtoken 9` + `ring` + `getrandom 0.2` `js`) is outdated and must not be copied.

Compile probes used **rustc 1.98.1** (`stable`) and `--target wasm32-unknown-unknown`.

---

## 0. Locked v1 decisions

| Topic | Decision |
|-------|----------|
| Access token | Symmetric **HS256** JWT via **`jwt-compact` 0.8** (`default-features = false, features = ["std"]`). No RSA/EdDSA. |
| Access claims | **`sub`, `exp`, `role` only** (`role` is `User` / `Admin`). `sub` is `auth_users.public_id` (UUIDv4 string), never the integer PK. No `iat` / `iss` / `jti` / email in the access token. |
| Email activation | **Stateless JWT**, different secret (`JWT_VERIFY_SECRET`). Claims bind `sub` + `email` + `exp` (and a type tag so it cannot be used as access). No activation table. |
| Session / refresh | Opaque 32-byte token in an **HttpOnly, Secure, `SameSite=Strict`** cookie (SPA and API share a domain). Row in **`auth_sessions`**. Rotate by updating `token_hash` on the same row. |
| Session hash | **HMAC-SHA-256(`HASH_PEPPER`, raw token)** (`hmac` + `sha2` + `subtle`). Not Argon2. Same on Worker and native. Pepper from `SecretStore`. |
| Passwords | **Same Argon2id on Worker and native** (OWASP [Password Storage](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html): `m=19456,t=2,p=1`). **Unique CSPRNG salt per password** (16+ bytes from `RandomSource`, stored **inside** the PHC string — no extra salt column). **Keyed with `HASH_PEPPER`**. Full PHC in `auth_users.password_hash`. Never a global/static salt, never username/email as salt, never unsalted SHA-256. Worker = native security; upgrade to Paid on Error 1102 rather than weakening. |
| Password policy | OWASP [Authentication](https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html) / NIST SP 800-63B: min **15** chars (no MFA in v1), max **256** (NIST floor for the max is 64), any Unicode, **no** composition rules, **no** silent truncation, **no** periodic rotation. Reject a bundled common-password list on register/change. Constant-time verify; dummy Argon2id on unknown user. |
| Platform ports | **`RandomSource` + `Clock` on `Platform` with default methods**, so native (and the Worker, for clock) need not implement them. WASM overrides `random()` only if the default Worker adapter is not already selected by `cfg`. **`SecretStore`** for JWT secrets and `HASH_PEPPER`. |
| Schema change | **Edit migration `001`** (`001_auth_create_users_and_token_tables`). Adds `auth_sessions` and `auth_users.public_id`. Nothing is in production; do not add `002`. |
| User identifiers | Integer **`id`** is SQLite rowid / FK target (`auth_sessions.user_id`; later OAuth/WebAuthn). Unique **`public_id` UUIDv4** (CSPRNG) is the external id: JWT `sub`, `/api/auth/users/{public_id}`, WebAuthn `userHandle`. Never serialize `id` in JSON; never accept `public_id` from clients. Do **not** use UUID as the PK (D1/SQLite FKs stay on integer rowid). |
| Secrets | `SecretStore.get(name)` (Worker secrets / native env). v1 names: `JWT_ACCESS_SECRET`, `JWT_VERIFY_SECRET`, `HASH_PEPPER`. Each ≥ 32 bytes. Access vs verify stay separate. |

---

## 1. What auth actually needs from crypto

From the existing flow ([auth-flow-diagram.md](./auth-flow-diagram.md)) and schema (`auth_users`; session rows planned as `auth_sessions.token_hash`):

| Operation | Algorithm | Needs CSPRNG? | Platform-specific? |
|-----------|-----------|---------------|--------------------|
| Sign / verify **access JWT** | HMAC-SHA-256 (recommended for v1) | **No** — deterministic given secret + payload | No, if we stay on pure-Rust HMAC |
| Sign / verify **email-activation JWT** (optional, stateless) | HMAC-SHA-256, **different secret** | No | No |
| **Refresh / session token** (opaque, HttpOnly cookie) | 32+ random bytes, store **HMAC-SHA-256(`HASH_PEPPER`, token)** | **Yes** (the token) | Entropy source differs (OS vs Workers `crypto.getRandomValues`); pepper from `SecretStore` |
| **`public_id` UUIDv4** | 16 CSPRNG bytes, RFC 4122 v4 | **Yes** | Same entropy as refresh; stored as unique string on `auth_users` |
| OAuth `state`, WebAuthn challenge (later) | random bytes | **Yes** | Same as refresh |
| Hash stored session token | **HMAC-SHA-256 + pepper**, not Argon2 | No | Pepper is a secret; hash is portable Rust |
| **Password** | **Argon2id** PHC: **unique salt** + shared pepper, same params on Worker and native | **Yes — new salt every hash** | Salt is public (in PHC); pepper is a `SecretStore` secret; Free CPU may force a **Paid** upgrade, not a weaker hash |

So: **JWT HMAC and session HMAC are portable library problems. Random bytes and secrets are platform problems. Argon2id is the same library and parameters on every host.** Do not put session tokens and passwords through the same hash. Do not give the Worker a weaker password path.

ECDSA (`ES256`) signing needs a per-signature nonce (CSPRNG). RSA-PSS needs blinding. We should not pick those for v1 on Workers.

---

## 2. Cloudflare Workers crypto surface

The Workers runtime implements [Web Crypto](https://developers.cloudflare.com/workers/runtime-apis/web-crypto/) including:

- **`crypto.getRandomValues`** — synchronous CSPRNG. This is the right entropy source on the Worker.
- **`crypto.randomUUID`** — UUID v4 helper.
- **`crypto.subtle`** — HMAC, SHA-2, ECDSA, Ed25519, RSA, AES, HKDF, PBKDF2. `sign` / `verify` / `digest` are **async**.
- **`crypto.subtle.timingSafeEqual`** — non-standard, useful later for comparing hashes.
- `worker` 0.8.x (`worker::crypto`) only wraps **`DigestStream`**. There is **no** first-class `getRandomValues` or HMAC helper in `workers-rs` today.

Using `subtle.sign("HMAC", …)` for JWTs would force the token API to be async and would couple the domain to JS `Promise`s (`SendFuture` on WASM). For ~200-byte JWT payloads, pure-Rust HMAC-SHA-256 in WASM is cheap enough that **Web Crypto HMAC is not worth a port**. Keep Web Crypto for **entropy** (and later maybe password KDF if we decide the JS implementation is faster/safer).

`std::time::SystemTime::now()` still panics on `wasm32-unknown-unknown`. This repo already solved wall-clock time with `time` + `time/wasm-bindgen` on the `d1` feature. JWT crates that call `SystemTime` or `chrono::Utc::now()` without a WASM clock backend will compile and then **panic at `exp` check**.

---

## 3. `getrandom` on `wasm32-unknown-unknown`

There is no OS entropy on this target. Backends:

| `getrandom` | WASM feature | Extra rustc cfg (historical) | Notes |
|-------------|--------------|------------------------------|--------|
| **0.2** | `js` | none | What `jsonwebtoken` 9/11 and `ring` 0.17 still pull. Enabling the feature on a **direct** dependency applies workspace-wide for that 0.x line. |
| **0.3.3** | `wasm_js` | **also** `RUSTFLAGS=--cfg getrandom_backend="wasm_js"` | Easy to get wrong; `workers-rs` issues [#736](https://github.com/cloudflare/workers-rs/issues/736), [#812](https://github.com/cloudflare/workers-rs/issues/812). |
| **0.3.4+ / 0.4** | `wasm_js` | cfg **not** required if the feature is on | Feature enables `Crypto.getRandomValues` via `wasm-bindgen`. |

**A Random port does not make a transitive `getrandom` compile.** If any JWT crate links `getrandom` on WASM, the **leaf crate** (`demo/worker` or `rundtisch` with `d1`) must still enable the matching feature (and, for 0.3.3, rustflags). Two major lines (`0.2` and `0.3`/`0.4`) can coexist; **each** needs its own feature. That is the usual Workers-rs footgun.

**Recommendation:** **`jwt-compact` HMAC-only** (locked; see §0). Session tokens and password salts will use the Random port (`crypto.getRandomValues` on the Worker). **`public_id` generation** currently calls `getrandom::fill` and sets RFC 4122 v4 bits (`uuid` without the `v4` feature — that feature does not compile on `wasm32-unknown-unknown`). The `d1` feature enables `getrandom/wasm_js` (same `Crypto.getRandomValues` backend). Switch that to `RandomSource` when the port lands so there is one entropy path. Do not add `.cargo/config.toml` rustflags unless a future crate forces `getrandom` 0.3.3. Pin **0.3.4+**.

---

## 4. Crate survey

### 4.1 Disqualified or poor fit

| Crate | Why not |
|-------|---------|
| **`josekit`** | OpenSSL. Cannot link into WASM Workers. |
| **`jsonwebtoken` `aws_lc_rs`** (v10/v11 default-ish backend) | Native AWS-LC. Probe **failed** on WASM at `getrandom` 0.2 before the C library; not a WASM backend. |
| **`biscuit`** (lawliet89 JOSE, not Biscuit tokens) | Old `jsonwebtoken` fork; not WASM-first. |
| **RustCrypto `jose-jwt`** | Split JOSE workspace, still early as a batteries-included JWT app crate. |
| **Rolling our own JWT** | Tempting given HMAC + base64 is small, but we would re-implement `alg` pinning, `exp`/`nbf` leeway, and base64url. Not worth it. |

PASETO (`pasetors`) is a better *design* than JWT if we controlled every client. The product docs and SPA already assume JWT access tokens; stay on JWT unless we explicitly change that.

### 4.2 Contenders (HS256, WASM compile-checked)

Probes: `cargo check --target wasm32-unknown-unknown`, then `cdylib` release with an exported `issue()` so the encoder is actually linked (rustc 1.98.1, 2026-09-21).

| Setup | WASM check | `getrandom` on WASM tree | Isolated cdylib `.wasm` |
|-------|------------|--------------------------|-------------------------|
| `hmac` + `sha2` only (baseline) | pass | none | **37 KiB** |
| **`jwt-compact` 0.8** `default-features=false, features=["std"]` | pass | **none** | **57 KiB** |
| `jwt-compact` 0.8 default (chrono clock + ciborium) | pass | none | (not sized; clock is unsafe on WASM — see below) |
| **`jwt-simple` 0.13** `default-features=false, features=["pure-rust"]` | pass | 0.4 via `ed25519-compact` (`wasm_js` enabled transitively) | **596 KiB** |
| `jwt-simple` 0.13 default | pass on WASM (native default pulls **`boring`**) | same | — |
| **`jsonwebtoken` 11** `rust_crypto` **without** extra `getrandom` | **fail** (`getrandom` 0.2 `compile_error!`) | 0.2 required | — |
| `jsonwebtoken` 11 `rust_crypto` + `getrandom 0.2/js` | pass | 0.2 | **1.4 MiB** (RSA/P-256/P-384/Ed25519 always in the `rust_crypto` feature) |
| `jsonwebtoken` 9 + `ring` + `getrandom 0.2/js` (July doc) | pass | 0.2 via `ring` | **694 KiB** |

Sizes are un-`wasm-opt`’d isolated cdylibs. In the real Worker, `wasm-bindgen` is already present via `worker`, so the interesting delta is **extra crypto**, not the JS shim. `jsonwebtoken` 11’s `rust_crypto` feature still **compiles rsa + p256 + p384 + ed25519-dalek + rand** even if we only call HS256.

### 4.3 Behaviour notes (not just compile)

**`jwt-compact` 0.8**

- HS256/384/512 via `hmac` + `sha2`. RSA feature is the one that needs `getrandom`.
- Header has **no public `alg` field**; algorithm is a type (`Hs256`). This blocks algorithm-confusion (`alg=none` / RS vs HS) by construction.
- `Hs256Key::new(bytes)` / `generate(rng)` — generate takes **`CryptoRng + RngCore`**, so it can use our Random port. Production keys come from secrets, not generate.
- `TimeOptions::new(leeway, clock_fn)` injects a clock. **`TimeOptions::default()` uses `chrono::Utc::now()`** and requires feature `clock`. Chrono’s `wasmbind` feature is **not** enabled by jwt-compact, so **default clock panics on WASM**. We must disable `clock` and pass our Clock port.
- Always depends on **`chrono`**, even with `clock` off. This repo uses **`time`**. Convert at the adapter (`OffsetDateTime` → unix timestamp or `chrono::DateTime<Utc>`).
- `no_std` + WASM tests exist. Latest **stable is 0.8.0 (2024-09)**; `0.9.0-beta.1` is the same vintage. Maintenance is the main risk versus `jsonwebtoken` 11.

**`jsonwebtoken` 11.1**

- Current ecosystem default. MSRV **1.88** (CI stable is fine).
- Must choose **exactly one** of `rust_crypto` or `aws_lc_rs`, or install a custom `CryptoProvider`.
- WASM timestamps: `get_current_timestamp()` uses `js_sys::Date` on `wasm32`. **No hook to inject a test clock** except turning `validate_exp` off and checking claims ourselves.
- Unconditional WASM deps: `js-sys`, `getrandom` **0.2 without `js`**. Consumers must add `getrandom = { version = "0.2", features = ["js"] }`.
- HMAC implementation (`hmac` + `sha2`) is solid; it is bundled with a large backend.
- Custom HMAC-only `CryptoProvider` is possible (HMAC is ~80 lines in-tree) if we want the jsonwebtoken API without RSA. Still need `getrandom` 0.2 `js` because the crate lists it for all WASM builds.

**`jwt-simple` 0.13**

- Designed for WASM/WASI (Fastly Compute). HMAC via compact `hmac-sha256`.
- Wall clock via **`coarsetime`**, which **does** bind `Date.now()` on WASM — runtime clock works.
- `HS256Key::from_bytes` / `authenticate` / `verify_token` is ergonomic; verification options are opinionated (15 min leeway by default).
- Default native feature **`optimal` enables `boring`**. WASM uses `superboring`. We would need `default-features = false, features = ["pure-rust"]` on **both** targets.
- No algorithm feature flags: Ed25519, P-256, P-384, K-256, RSA, ML-DSA, JWE all compile. WASM ~10× jwt-compact.
- `HS256Key::generate()` calls `rand::rng()` (`rand` 0.10 → `getrandom` 0.4). Compile succeeded because **`ed25519-compact` enabled `getrandom/wasm_js` for us** — a transitive accident we should not rely on.
- Author explicitly prefers JS WebCrypto JWT **in browsers**; we are a Worker, so that warning is weaker, but the binary cost remains.

### 4.4 Choice: `jwt-compact` 0.8 HMAC-only

**Locked.** Use `jwt-compact` 0.8 in `rundtisch`:

```toml
jwt-compact = { version = "0.8", default-features = false, features = ["std"] }
```

Do **not** enable `clock`, `ciborium`, `rsa`, or EdDSA features for v1.

Why this over `jsonwebtoken` 11:

1. HS256 path has **no `getrandom`**, so WASM builds do not depend on `js` / `wasm_js` feature soup.
2. Smallest extra WASM (~20 KiB over raw HMAC).
3. Typed algorithm + injectable clock match hexagonal ports.
4. RFC 7518 key-length wrappers (`StrongKey`) for HS256.

Why not `jwt-simple`: clock works, but the graph is huge and native `boring` is a trap; HMAC-only is not selectable.

**Fallback** if jwt-compact looks unmaintained when we implement: `jsonwebtoken` 11 + `rust_crypto` + explicit `getrandom` 0.2 `js` on the WASM target, and check `exp` ourselves via the Clock port. Not the v1 path.

---

## 5. Hexagonal ports

### 5.1 Random — **yes, add `RandomSource`**

JWT HMAC does not need it. **Auth still does** (refresh tokens now; OAuth/WebAuthn later). Entropy is the one crypto primitive that **cannot** be the same code on native vs Workers without `getrandom` WASM features.

Fit the existing `Platform` + `AppState` pattern (`DatabaseExecutor` today; `SecretStore` already commented).

```rust
// crates/rundtisch/src/traits/random.rs
pub trait RandomSource: Send + Sync {
    /// Fill `dest` with cryptographically secure random bytes.
    fn fill_bytes(&self, dest: &mut [u8]) -> Result<(), RandomError>;
}

impl dyn RandomSource {
    fn bytes(&self, n: usize) -> Result<Vec<u8>, RandomError> { /* fill */ }
}
```

Keep it **synchronous**. Both `getrandom` on native and `crypto.getRandomValues` on Workers are sync. Do not use `crypto.subtle` here.

**Default implementations (native platforms should not implement these).** Associated type defaults are still awkward on stable Rust, so `random` / `clock` are **default trait methods** returning trait objects. `NativePlatform` keeps only `Database`. Tests override the methods.

```rust
pub trait Platform: 'static {
    type Database: DatabaseExecutor + Send + Sync;
    fn database(&self) -> Arc<Self::Database>;

    fn random(&self) -> Arc<dyn RandomSource> {
        Arc::new(DefaultRandom)
    }

    fn clock(&self) -> Arc<dyn Clock> {
        Arc::new(SystemClock)
    }
}
```

| Adapter | Backend | Who uses it |
|---------|---------|-------------|
| `OsRandom` | `getrandom::fill` | `DefaultRandom` on **non-WASM** |
| `WorkerRandom` | `crypto.getRandomValues` via `js_sys` (not `getrandom`) | `DefaultRandom` on **`wasm32`** |
| `SystemClock` | `time::OffsetDateTime::now_utc()` | Default on **both** (enable `time/wasm-bindgen` on `d1`) |
| `ReplayRandom` / `FrozenClock` | deterministic | tests override `fn random` / `fn clock` |

`CloudflarePlatform` does **not** need to mention Random or Clock unless WorkerRandom later needs `Env` (it should not: `crypto` is global). `AppState` holds `database`, `random`, `clock`, and later `secrets` filled by `from_platform`.

**Domain use:** 32-byte session token → URL-safe base64 (no padding) → HMAC-SHA-256(`HASH_PEPPER`, token) → `auth_sessions.token_hash`. Same `RandomSource` supplies the 16-byte Argon2id salt (do **not** enable `argon2`’s `getrandom` feature on WASM) and the 16 random bytes for `public_id` UUIDv4. Until that port exists, `new_public_id()` uses `getrandom::fill` directly.

Error type: small `RandomError` (`Unavailable` / `Backend(String)`), not `traits::db::Error`.

### 5.2 Clock — default `SystemClock`

Handlers today call `time::OffsetDateTime::now_utc()` (`auth/models.rs`). JWT `exp` and session `expires_at` / `last_used_at` need the same clock. Tests freeze it via `fn clock`.

```rust
pub trait Clock: Send + Sync {
    fn now_utc(&self) -> time::OffsetDateTime;
}
```

`SystemClock` is the default on native **and** WASM (`d1` already enables `time/wasm-bindgen`). jwt-compact’s `TimeOptions::default()` (`chrono::Utc::now()`) must **not** be used; pass `TimeOptions::new(leeway, || clock.now_utc()…)` at the adapter boundary.

### 5.3 Secrets — `SecretStore` (JWT keys + hash pepper)

Commented `SecretStore` on `Platform` is the v1 secrets port. **Signing, session HMAC, and Argon2id must not hard-code secrets.** Minimum API: `get(name) -> Result<String>` (bytes via UTF-8 / hex as we standardize in the adapter).

| Name | Use | Length |
|------|-----|--------|
| `JWT_ACCESS_SECRET` | HS256 access JWTs | ≥ 32 bytes (`StrongKey`) |
| `JWT_VERIFY_SECRET` | HS256 email-activation JWTs (never the access key) | ≥ 32 bytes |
| `HASH_PEPPER` | Session `token_hash` = HMAC-SHA-256(pepper, raw token); Argon2id **keyed secret** (`Argon2::new_with_secret`) | **32 bytes** (fits argon2’s max secret length) |

Worker: Wrangler/dashboard secrets (`wrangler secret put`). Native: environment variables. Tests: an in-memory map.

`HASH_PEPPER` is not stored in D1. It is not in the Argon2 PHC string. A DB dump without the pepper cannot verify passwords or mint a session cookie from `token_hash`. Rotating `HASH_PEPPER` invalidates every password verify and every session lookup — treat it like a signing key (not v1: dual-pepper window).

Do not reuse `JWT_ACCESS_SECRET` as the pepper (different compromise story; JWT leak would otherwise become an offline-hash oracle).

`SecretStore` is a real `Platform` port (Worker `env.secret` vs native environment variables). Clock and Random stay default methods; secrets do not, because the backends differ.

### 5.4 JWT issuer — **library adapter, not a Platform port**

Signing is CPU-only and the same on native and WASM if we stay on hmac+sha2. Making `TokenIssuer` a `Platform` associated type would imply a Worker WebCrypto adapter we do not want for v1.

Put a small **in-crate service** next to auth (e.g. `auth/jwt.rs`) that:

- takes `&dyn Clock` and the HS256 secret bytes;
- encodes/decodes **access** claims `{ sub, exp, role }` with `Hs256`;
- encodes/decodes **email-activation** JWTs with a **different** secret and claims (`sub`, `email`, `exp`, plus a type tag);
- maps crate errors to a domain `TokenError`.

Handler tests can use a real HS256 key and `FrozenClock`.

### 5.5 Password hasher — **yes, a port; not the same as session hashing**

See [§11](#11-hashing-sessions-vs-passwords-worker--native) and [§12](#12-password-management--authentication-owasp--nist). Session HMAC is a library call (`hmac` + `sha2`) using `HASH_PEPPER`. Passwords use one **`Argon2idHasher`**: unique salt + shared pepper + OWASP params on Worker and native. Keep a `PasswordHasher` trait so tests can inject a fake; do not ship a weaker Worker adapter.

### 5.6 What we will **not** port

- SHA-256 / HMAC-SHA-256 (`sha2` + `hmac` + `subtle`) — pure Rust, same on native and WASM.
- Email sending — HTTP to Resend; its own port later.

---

## 6. Tokens, cookie, and `auth_sessions`

### Access JWT (Bearer, 5–15 min, memory on the client)

```json
{
  "sub": "550e8400-e29b-41d4-a716-446655440000",
  "role": "User",
  "exp": 0
}
```

`sub` is `auth_users.public_id` (hyphenated UUIDv4), not the integer PK. Verify with typed `Hs256` only. Check `exp` via `Clock` + small leeway. Role changes take effect when the access token expires.

### Email activation JWT (stateless)

Separate secret. Claims: `sub` (= `public_id`), `email`, `exp`, plus a type tag that the access-token verifier rejects. Activation:

```sql
UPDATE auth_users SET email_verified_at = ? WHERE public_id = ? AND email = ?;
```

No extra table.

### `auth_users` identifiers (edit migration `001`)

OWASP [Authentication](https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html) recommends **random user IDs** so sequential integers are not leaked in URLs, JWTs, or WebAuthn. Sequential `id` values enumerate accounts, enable IDOR guessing, and correlate JWT `sub` with row count / signup order.

**Locked:** dual key. Integer PK stays for SQLite/D1 FKs; the public identifier is a unique UUIDv4.

| Column | Type | Notes |
|--------|------|--------|
| `id` | integer PK AUTOINCREMENT | Rowid. FK target for `auth_sessions.user_id`. **`#[serde(skip_serializing)]`** — never in JSON. |
| `public_id` | string UNIQUE NOT NULL | UUIDv4, assigned server-side (`new_public_id()`). JWT `sub`, `/api/auth/users/{public_id}`, later WebAuthn `userHandle`. Clients cannot set it (`skip_deserializing` on `NewUser`). |

Do **not** make `public_id` the primary key: D1/SQLite foreign keys and `last_insert_rowid()` stay on integer `id`. Do **not** expose `id` as a second public identifier.

### Session cookie

HttpOnly, Secure, **`SameSite=Strict`** (frontend and API are the same site). Value: 32 CSPRNG bytes, URL-safe base64 without padding. Never persist the raw value.

### `auth_sessions` (edit migration `001`, no `002`)

SQLite for the demo is in `demo/migrations/001_auth_create_users_and_token_tables.sql` (regenerate with `generate_auth_migrations`). Nothing is in production, so this rewrite is the whole schema change.

| Column | Type | Notes |
|--------|------|--------|
| `id` | integer PK | Stable across token rotation |
| `user_id` | integer FK → `auth_users.id` ON DELETE CASCADE | Internal PK, not `public_id` |
| `token_hash` | string UNIQUE | HMAC-SHA-256(`HASH_PEPPER`, cookie), hex |
| `created_at` | RFC 3339 string | |
| `last_used_at` | RFC 3339 string | Update **only** on `/auth/refresh` |
| `expires_at` | RFC 3339 string | Absolute lifetime |
| `revoked_at` | RFC 3339 string NULL | Active = `revoked_at IS NULL AND expires_at > now` |
| `user_agent` | string NULL | Display only; cap ~512 in application code |

On refresh: look up by `token_hash`, reject if revoked/expired, generate a new raw token, **UPDATE** `token_hash` + `last_used_at` on the same `id`. Logout sets `revoked_at`. Logout-all updates every row for that `user_id`. Reuse detection (`previous_token_hash`) is deferred.

---

## 7. Implementation sequence

1. **Ports:** `RandomSource` + `Clock` with default methods; `SecretStore` (Worker secrets / native env, including `HASH_PEPPER`); `AppState` fields; tests with `ReplayRandom` / `FrozenClock` / in-memory secrets. Native and Cloudflare platforms still do not implement Random or Clock.
2. **JWT helper:** `jwt-compact` HS256, access claims `{sub, exp, role}`, activation JWT tests. `cargo check -p rundtisch --features d1 --target wasm32-unknown-unknown`.
3. **Secrets + sessions + login:** `SecretStore`, same salted+peppered Argon2id hasher on Worker and native, generic login errors + dummy hash on unknown user, issue access JWT, persist HMAC’d `auth_sessions`, Strict cookie, Bearer extractor.

CI should keep compiling the Worker target on every PR that touches crypto.

---

## 8. Remaining (not blocking ports / JWT helper)

- Concrete access-token TTL (5 vs 15 min) and session TTL (days).
- If register/login hits Error 1102 on Free, switch the **same** Worker to Paid (same hasher). No param fork.
- Later: Have I Been Pwned k-anonymity check; MFA; forgot-password JWT; dual-pepper window; login rate-limit binding.

---

## 9. Corrections to the July 2026 JWT snippet

The published snippet is still directionally right (HS256, explicit algorithm, separate verify secret) but the dependency block is wrong for 2026:

```toml
# Do not copy this for new code
jsonwebtoken = "9.3"
getrandom = { version = "0.2", features = ["js"] }
```

- `jsonwebtoken` 9 used **`ring`**, which compiles on WASM with `getrandom` 0.2 `js` but is a larger, clang-flavoured stack (~694 KiB isolated).
- `jsonwebtoken` 11 needs **`features = ["rust_crypto"]`** (or a custom provider). `aws_lc_rs` is not for Workers.
- `getrandom` 0.2 `js` is only required if something in the graph still depends on 0.2. hmac-only `jwt-compact` does not.
- `SystemTime` in that snippet is the old WASM panic; use the Clock port / `time` crate.

---

## 10. Probe appendix

Commands (scratch workspace, not in this repo): `cargo check -p <probe> --target wasm32-unknown-unknown` and `cargo build --release --target wasm32-unknown-unknown` with `crate-type = ["cdylib"]` plus `#[unsafe(no_mangle)] extern "C" fn wasm_issue()`.

| Probe | Result |
|-------|--------|
| jsonwebtoken 11 `rust_crypto` bare | fail: getrandom 0.2 WASM `compile_error!` |
| jsonwebtoken 11 `rust_crypto` + getrandom 0.2 `js` | pass, ~1.4 MiB cdylib |
| jsonwebtoken 11 `aws_lc_rs` | fail: same getrandom 0.2 error (native backend unused) |
| jwt-compact 0.8 default | pass, no getrandom |
| jwt-compact 0.8 hmac-only | pass, no getrandom, ~57 KiB cdylib |
| jwt-simple 0.13 default / `pure-rust` | pass; getrandom 0.4 via ed25519-compact `wasm_js` |
| hmac + sha2 | pass, ~37 KiB |
| jsonwebtoken 9 + ring + getrandom 0.2 `js` | pass, ~694 KiB |
| getrandom 0.3 `wasm_js` | pass on 0.3.4 without extra rustc cfg |
| `argon2` 0.6 `default-features=false, features=["alloc"]` | pass, no getrandom |
| `argon2` 0.6 `alloc` + `password-hash` (no `getrandom` feature) | pass, no getrandom |
| `sha2` 0.10 + `subtle` 2 | pass, no getrandom |

Default `argon2 = "0.6"` enables `getrandom` and **will** break a WASM Worker unless that crate’s `wasm_js`/`js` feature is also on. Always disable default features on the Worker graph.

---

## 11. Hashing: sessions vs passwords (Worker = native)

Two different jobs. Mixing them is the usual mistake. The Worker demo and the native binary **use the same algorithms, parameters, and `HASH_PEPPER`**. Security parity is the constraint; the Free 10 ms CPU cap is not allowed to fork a weaker Worker path. If register/login exceeds Free CPU, **upgrade that Worker to Paid**.

### 11.1 Session / refresh `token_hash` — HMAC-SHA-256 + pepper, no port

The cookie value is 32 CSPRNG bytes. Store a **peppered HMAC**, not bare SHA-256 and not Argon2:

```
token_hash = hex(HMAC-SHA-256(HASH_PEPPER, raw_token))
```

- A D1 dump without `HASH_PEPPER` cannot be used to mint cookies.
- `/auth/refresh` stays cheap (milliseconds), so it does not become a CPU DoS.
- **Do not use Argon2 here.** OWASP Argon2id is ~50–100 ms in WASM; doing that on every refresh buys nothing.

`HASH_PEPPER` comes from `SecretStore` (same port as `JWT_ACCESS_SECRET`). Compare with `subtle::ConstantTimeEq`, not `==`.

WASM deps (already compile-checked):

```toml
hmac = "0.12"
sha2 = "0.10"
subtle = "2"
```

### 11.2 Passwords — Argon2id + pepper, **same hasher on Worker and native**

Argon2id is the OWASP first choice. RustCrypto **`argon2` 0.6** compiles to `wasm32-unknown-unknown` without `getrandom` if we supply the salt.

```toml
# Worker and native — never enable crate defaults
argon2 = { version = "0.6", default-features = false, features = ["alloc", "password-hash", "zeroize"] }
```

| Feature | Use on WASM? |
|---------|----------------|
| `alloc` + `password-hash` | **Yes** — PHC strings (`$argon2id$v=19$m=…`) |
| `zeroize` | Yes |
| **`getrandom`** (in default features) | **No** — salt from `RandomSource` |
| `parallel` / `rayon` | **No** — `p=1` on every platform |

**Salt vs pepper (both required):**

| | Salt | Pepper (`HASH_PEPPER`) |
|---|------|------------------------|
| Unique? | **Yes — new 16-byte CSPRNG value for every hash** (register, password change, rehash) | No — one secret for the deployment |
| Source | `RandomSource::fill_bytes` (never `getrandom` crate defaults on WASM) | `SecretStore` |
| Stored in D1? | **Yes**, encoded inside the PHC string (`$argon2id$v=19$m=19456,t=2,p=1$<salt>$<digest>`) | **No** |
| Purpose | Stop rainbow tables and “crack once, all users with that password” | Stop offline cracking if D1 leaks without Worker/native secrets |

Do **not** add a `password_salt` column. Duplicating the PHC salt is how hashes and salts get out of sync. Do **not** use a static salt, email, user id, or `HASH_PEPPER` as the salt.

Example stored value (illustrative):

```
$argon2id$v=19$m=19456,t=2,p=1$<16-byte-salt-b64>$<digest-b64>
```

**Construction (identical on Worker and native):**

- Params: OWASP **`m=19456, t=2, p=1`**. Native must **not** use a heavier preset so verify cost stays the same everywhere.
- Salt: **16 bytes (128 bits)** from `RandomSource` → `password_hash::SaltString` (RFC 9106 / OWASP minimum). A new salt on every `hash()` call, including password change.
- Pepper: `Argon2::new_with_secret(HASH_PEPPER)` — 32-byte key, **not** serialized into the PHC string. Verify must load the same pepper from `SecretStore`.
- Store the **full PHC string** in `auth_users.password_hash`. Verify with `Argon2::verify_password` using the salt and params **in the stored string**, in constant time (`password-hash` crate).
- After hashing, **zeroize** the plaintext password buffer (`zeroize` feature). Never log the password or the PHC.
- On successful login, if stored params are below the current OWASP preset, **rehash with a fresh salt** and update the row (upgrade path without a migration).

Login/register only. Never on the Bearer or refresh path. Session tokens are **not** salted Argon2 (high-entropy secrets; HMAC-SHA-256 + pepper is correct).

### 11.3 Worker CPU: keep the hasher, pay for CPU if needed

| Plan | CPU per request | Memory per isolate |
|------|-----------------|--------------------|
| Workers **Free** (current demo) | **10 ms** | 128 MB |
| Workers **Paid** | 30 s default (up to 5 min) | 128 MB |

[Limits](https://developers.cloudflare.com/workers/platform/limits/): CPU is *execution* time. D1 wait does not count; Argon2id **does**. Exceeding the cap is Error 1102 (`exceededCpu`).

| Job | ~CPU | On the Worker demo |
|-----|------|--------------------|
| HMAC-SHA-256 JWT / session HMAC | << 10 ms | **Yes, in-process** |
| Argon2id OWASP `m=19456,t=2,p=1` in WASM | ~100 ms | **Yes, in-process — same as native.** Likely over Free 10 ms. |

**Locked policy:** do not weaken Argon2, do not switch the Worker to PBKDF2-100k, magic-link-only auth, or `password_hash = NULL` in order to stay on Free. When Free kills register/login, move the **same** script to Workers Paid. A dedicated hasher Worker + service binding is an optional later split of the **same** Argon2id+pepper code, not a different algorithm.

**Rejected as crate defaults**

- SHA-256 / HMAC of the password as the only KDF (pepper or not). Fast hashes are for session tokens.
- Web Crypto PBKDF2 at 100k (below OWASP 600k; runtime throws above 100k).
- Client-side hashing (hash becomes the password without OPAQUE/SRP).
- A weaker `WorkerArgon2` preset so Free stays green.
- Skipping password hashing on the Worker demo.

### 11.4 `PasswordHasher` (one production adapter)

Policy object built at boot from `RandomSource` + `HASH_PEPPER` + the shared params. Not a `Platform` associated type — Worker and native run the same code.

```rust
pub trait PasswordHasher: Send + Sync {
    fn hash(&self, password: &str) -> Result<String, PasswordHashError>;
    fn verify(&self, password: &str, password_hash: &str) -> Result<bool, PasswordHashError>;
}
```

| Adapter | Behaviour |
|---------|-----------|
| `Argon2idHasher` | RustCrypto Argon2id, OWASP params, salt from `RandomSource`, secret = `HASH_PEPPER`. **Worker demo and native.** |
| `TestPasswordHasher` | Fast, deterministic (never production) |

Wire as a field on `AppState` constructed in `from_platform`. Do **not** merge it with `RandomSource` or a generic “Hasher” that also does session HMAC.

`hash` / `verify` stay **sync** (CPU-bound; async does not yield the isolate). If we later offload to a Paid hasher Worker, the trait becomes `impl Future + Send` like `DatabaseExecutor`.

### 11.5 WASM dependency / port map (sessions + passwords)

| Need | Mechanism | WASM issue |
|------|-----------|------------|
| `public_id` UUIDv4 | 16 CSPRNG bytes, RFC 4122 version/variant bits | `getrandom` until `RandomSource`; `d1` enables `wasm_js`. Do **not** enable uuid `v4` (WASM compile_error). |
| Session token bytes | **`RandomSource` port** | `crypto.getRandomValues` vs OS |
| Session `token_hash` | HMAC-SHA-256(`HASH_PEPPER`, token) | Pepper from `SecretStore`; hash is cheap |
| JWT access / activation | `jwt-compact` HS256 | Secrets from `SecretStore` |
| JWT `exp`, `expires_at` | **`Clock` port** | `time` + `wasm-bindgen` already on `d1` |
| `JWT_ACCESS_SECRET`, `JWT_VERIFY_SECRET`, `HASH_PEPPER` | **`SecretStore` port** | Worker secrets vs env |
| Password salt (unique, 16 bytes) | **`RandomSource`**, embedded in PHC | Must be CSPRNG; never a column of its own |
| Password hash / verify | **`Argon2idHasher`** (salt + `HASH_PEPPER`, same as native) | ~100 ms CPU; upgrade to Paid if Free 1102 |

No extra port for “hashing in general.”

---

## 12. Password management & authentication (OWASP / NIST)

v1 follows current [OWASP Password Storage](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html), [OWASP Authentication](https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html), and NIST SP 800-63B. The Worker demo does not get a lighter interpretation.

### 12.1 Store verifiers, not passwords

- Never store plaintext, reversible encryption, or a fast hash (SHA-256, HMAC-only, MD5, SHA-1) of a password.
- One PHC string per user: **algorithm + params + unique salt + digest**. Salt is mandatory and per-password ([§11.2](#112-passwords--argon2id--pepper-same-hasher-on-worker-and-native)).
- Pepper is extra defense-in-depth, stored only in `SecretStore`, never in D1.
- Compare with the library verifier (constant time). Dummy-verify a hard-coded PHC when the email is unknown so login timing does not enumerate accounts.

### 12.2 Password rules (v1)

| Rule | v1 |
|------|----|
| Minimum length | **15** (no MFA yet; NIST/OWASP treat shorter than 15 as weak without MFA) |
| Maximum length | **256** (floor is 64; cap avoids Argon2 CPU DoS on huge bodies) |
| Composition / complexity | **None** (no “must include a digit”) |
| Periodic rotation | **No** |
| Silent truncation | **Forbidden** (reject over-max; Argon2 has no bcrypt 72-byte trap) |
| Allowed characters | Any Unicode; allow paste |
| Common passwords | Reject against a **bundled** top-password list on register and change |
| Breached-password API (HIBP) | **Later** (k-anonymity range query) |
| Change password | Require the current password; new hash = **new salt** + same pepper |

### 12.3 Authentication mechanism (v1)

| Practice | v1 |
|----------|----|
| Short-lived access JWT | HS256, `sub` (= `public_id`) / `exp` / `role`, memory on the client |
| Opaque user identifiers | Integer PK internally; UUIDv4 `public_id` in JWT `sub`, API paths, WebAuthn `userHandle`; never serialize integer `id` |
| Server-side session | Rotating opaque cookie, HMAC + pepper, `auth_sessions`, revoke on logout |
| Cookie flags | HttpOnly, Secure, `SameSite=Strict`, `__Host-` prefix when served on HTTPS |
| Email proof | Stateless activation JWT; login requires `email_verified_at` |
| Login errors | Single generic message (`invalid_credentials`); same for bad password and unknown email |
| Transport | HTTPS only in deploy (Secure cookie is meaningless on cleartext) |
| Secrets | Distinct `JWT_ACCESS_SECRET`, `JWT_VERIFY_SECRET`, `HASH_PEPPER` |
| Rate limit | Register / login / activate / refresh (platform limiter; exact binding later) |
| MFA / WebAuthn / OAuth | **Later** (planned flows already exist as docs) |
| Forgot password | **Later**, same JWT shape as activation, then **new salt** on reset |

### 12.4 Explicitly not SOTA — do not ship

- Global or per-app salt, username/email as salt, empty salt
- Unsalted or fast hashes for passwords
- Weaker Worker KDF, PBKDF2-100k, client-side hashing
- Revealing “email not registered” vs “wrong password” on login
- Logging passwords, cookies, or PHC strings
- Storing the raw session token
- `alg=none` / algorithm confusion (typed `Hs256` in jwt-compact)
- Long-lived access tokens instead of rotating sessions
