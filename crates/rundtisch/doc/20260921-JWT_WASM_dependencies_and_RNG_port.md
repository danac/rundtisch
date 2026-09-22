# JWT on Cloudflare Workers: crate choice, crypto/RNG, and hexagonal ports

**Date:** 2026-09-21 (decisions locked 2026-09-22)  
**Status:** decided plan — JWT/ports not implemented yet; schema in `001` updated to `auth_sessions`  
**Related:** [auth-flow-diagram.md](./auth-flow-diagram.md), [20260708-150107-RustCloudflareWorkerAuthenticationDesign.md](./20260708-150107-RustCloudflareWorkerAuthenticationDesign.md)

This note records the crate survey, WASM probes, and the **locked v1 auth design**. The July 2026 snippet (`jsonwebtoken 9` + `ring` + `getrandom 0.2` `js`) is outdated and must not be copied.

Compile probes used **rustc 1.98.1** (`stable`) and `--target wasm32-unknown-unknown`.

---

## 0. Locked v1 decisions

| Topic | Decision |
|-------|----------|
| Access token | Symmetric **HS256** JWT via **`jwt-compact` 0.8** (`default-features = false, features = ["std"]`). No RSA/EdDSA. |
| Access claims | **`sub`, `exp`, `role` only** (`role` is `User` / `Admin`). No `iat` / `iss` / `jti` / email in the access token. |
| Email activation | **Stateless JWT**, different secret (`JWT_VERIFY_SECRET`). Claims bind `sub` + `email` + `exp` (and a type tag so it cannot be used as access). No activation table. |
| Session / refresh | Opaque 32-byte token in an **HttpOnly, Secure, `SameSite=Strict`** cookie (SPA and API share a domain). Row in **`auth_sessions`**. Rotate by updating `token_hash` on the same row. |
| Session hash | **SHA-256** of the raw token (`sha2` + `subtle`). Not Argon2. Fine on Free Workers. Optional HMAC pepper later; not required for v1. |
| Passwords | **Argon2id** is still the algorithm, but **not executed on Free Workers** (10 ms CPU). Native: OWASP `m=19456,t=2,p=1` via `PasswordHasher`. Worker v1: **no password KDF** — magic-link / activation JWT (`password_hash` stays NULL). Do not ship PBKDF2-100k or a weakened Argon2 as the crate default. Paid hasher Worker via service binding is a later upgrade. |
| Platform ports | **`RandomSource` + `Clock` on `Platform` with default methods**, so native (and the Worker, for clock) need not implement them. WASM overrides `random()` only if the default Worker adapter is not already selected by `cfg`. |
| Schema change | **Edit migration `001`** (`001_auth_create_users_and_token_tables`). Nothing is in production; do not add `002`. |
| Secrets | Still a later `SecretStore`. Access vs verify secrets stay separate. HS256 secret ≥ 32 bytes. |

---

## 1. What auth actually needs from crypto

From the existing flow ([auth-flow-diagram.md](./auth-flow-diagram.md)) and schema (`auth_users`; session rows planned as `auth_sessions.token_hash`):

| Operation | Algorithm | Needs CSPRNG? | Platform-specific? |
|-----------|-----------|---------------|--------------------|
| Sign / verify **access JWT** | HMAC-SHA-256 (recommended for v1) | **No** — deterministic given secret + payload | No, if we stay on pure-Rust HMAC |
| Sign / verify **email-activation JWT** (optional, stateless) | HMAC-SHA-256, **different secret** | No | No |
| **Refresh / session token** (opaque, HttpOnly cookie) | 32+ random bytes, store **SHA-256** (or HMAC-SHA-256 + pepper) | **Yes** (the token) | Entropy source differs (OS vs Workers `crypto.getRandomValues`) |
| OAuth `state`, WebAuthn challenge (later) | random bytes | **Yes** | Same as refresh |
| Hash stored session token | **SHA-256**, not Argon2 | No | No — high-entropy secret |
| **Password** | **Argon2id** (PHC string) on native / Paid | **Yes** (per-password salt) | Algorithm is portable Rust; **CPU budget is not** — Free Workers cannot run it |

So: **JWT HMAC and SHA-256 are portable library problems. Random bytes are a platform problem. Argon2id is a portable library that the Free Worker cannot afford.** Do not put session tokens and passwords through the same hash.

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

**Recommendation:** **`jwt-compact` HMAC-only** (locked; see §0). Worker entropy for tokens/salts is `crypto.getRandomValues` via the Random port default, not `getrandom`. Do not add `.cargo/config.toml` rustflags unless a future crate forces `getrandom` 0.3.3.

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

`CloudflarePlatform` does **not** need to mention Random or Clock unless WorkerRandom later needs `Env` (it should not: `crypto` is global). `AppState` holds `database`, `random`, and `clock` filled by `from_platform`.

**Domain use:** 32-byte session token → URL-safe base64 (no padding) → SHA-256 → `auth_sessions.token_hash`. Same `RandomSource` supplies the 16-byte Argon2id salt (do **not** enable `argon2`’s `getrandom` feature on WASM).

Error type: small `RandomError` (`Unavailable` / `Backend(String)`), not `traits::db::Error`.

### 5.2 Clock — default `SystemClock`

Handlers today call `time::OffsetDateTime::now_utc()` (`auth/models.rs`). JWT `exp` and session `expires_at` / `last_used_at` need the same clock. Tests freeze it via `fn clock`.

```rust
pub trait Clock: Send + Sync {
    fn now_utc(&self) -> time::OffsetDateTime;
}
```

`SystemClock` is the default on native **and** WASM (`d1` already enables `time/wasm-bindgen`). jwt-compact’s `TimeOptions::default()` (`chrono::Utc::now()`) must **not** be used; pass `TimeOptions::new(leeway, || clock.now_utc()…)` at the adapter boundary.

### 5.3 Secrets — already planned; needed before signing

Commented `SecretStore` on `Platform` is the right place for `JWT_ACCESS_SECRET` and `JWT_VERIFY_SECRET` (separate keys; see July design). Implementation can stay a follow-up, but **signing must not hard-code secrets**. Minimum v1: `get(name) -> Result<String>` from Worker secrets / env vars.

HS256 secret: **≥ 256 bits** (32 bytes). jwt-compact can enforce this via `StrongKey`.

### 5.4 JWT issuer — **library adapter, not a Platform port**

Signing is CPU-only and the same on native and WASM if we stay on hmac+sha2. Making `TokenIssuer` a `Platform` associated type would imply a Worker WebCrypto adapter we do not want for v1.

Put a small **in-crate service** next to auth (e.g. `auth/jwt.rs`) that:

- takes `&dyn Clock` and the HS256 secret bytes;
- encodes/decodes **access** claims `{ sub, exp, role }` with `Hs256`;
- encodes/decodes **email-activation** JWTs with a **different** secret and claims (`sub`, `email`, `exp`, plus a type tag);
- maps crate errors to a domain `TokenError`.

Handler tests can use a real HS256 key and `FrozenClock`.

### 5.5 Password hasher — **yes, a port; not the same as session hashing**

See [§11](#11-hashing-on-wasm-sessions-vs-passwords). Session `token_hash` stays a library call (`sha2`). Passwords get a `PasswordHasher` port so **native Argon2id**, **Worker Free (no KDF)**, tests, and a later Paid hasher Worker can differ without touching handlers.

### 5.6 What we will **not** port

- SHA-256 / HMAC-SHA-256 (`sha2` + `hmac` + `subtle`) — pure Rust, same on native and WASM.
- Email sending — HTTP to Resend; its own port later.

---

## 6. Tokens, cookie, and `auth_sessions`

### Access JWT (Bearer, 5–15 min, memory on the client)

```json
{
  "sub": "123",
  "role": "User",
  "exp": 0
}
```

Verify with typed `Hs256` only. Check `exp` via `Clock` + small leeway. Role changes take effect when the access token expires.

### Email activation JWT (stateless)

Separate secret. Claims: `sub`, `email`, `exp`, plus a type tag that the access-token verifier rejects. Activation:

```sql
UPDATE auth_users SET email_verified_at = ? WHERE id = ? AND email = ?;
```

No extra table.

### Session cookie

HttpOnly, Secure, **`SameSite=Strict`** (frontend and API are the same site). Value: 32 CSPRNG bytes, URL-safe base64 without padding. Never persist the raw value.

### `auth_sessions` (edit migration `001`, no `002`)

SQLite for the demo is in `demo/migrations/001_auth_create_users_and_token_tables.sql` (regenerate with `generate_auth_migrations`). Nothing is in production, so this rewrite is the whole schema change.

| Column | Type | Notes |
|--------|------|--------|
| `id` | integer PK | Stable across token rotation |
| `user_id` | integer FK → `auth_users` ON DELETE CASCADE | |
| `token_hash` | string UNIQUE | SHA-256 (hex) of the cookie |
| `created_at` | RFC 3339 string | |
| `last_used_at` | RFC 3339 string | Update **only** on `/auth/refresh` |
| `expires_at` | RFC 3339 string | Absolute lifetime |
| `revoked_at` | RFC 3339 string NULL | Active = `revoked_at IS NULL AND expires_at > now` |
| `user_agent` | string NULL | Display only; cap ~512 in application code |

On refresh: look up by `token_hash`, reject if revoked/expired, generate a new raw token, **UPDATE** `token_hash` + `last_used_at` on the same `id`. Logout sets `revoked_at`. Logout-all updates every row for that `user_id`. Reuse detection (`previous_token_hash`) is deferred.

---

## 7. Implementation sequence

1. **Ports:** `RandomSource` + `Clock` with default methods; `AppState` fields; tests with `ReplayRandom` / `FrozenClock`. Native and Cloudflare platforms stay Database-only.
2. **JWT helper:** `jwt-compact` HS256, access claims `{sub, exp, role}`, activation JWT tests. `cargo check -p rundtisch --features d1 --target wasm32-unknown-unknown`.
3. **Secrets + sessions + login:** `SecretStore`, native Argon2id `PasswordHasher`, Worker magic-link (reuse activation JWT), issue access JWT, persist `auth_sessions`, Strict cookie, Bearer extractor.

CI should keep compiling the Worker target on every PR that touches crypto.

---

## 8. Remaining (not blocking ports / JWT helper)

- Optional `SESSION_PEPPER` HMAC instead of bare SHA-256.
- Concrete access-token TTL (5 vs 15 min) and session TTL (days).
- Later: Paid hasher Worker + service binding if password login must run on the edge.

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

## 11. Hashing on WASM: sessions vs passwords

Two different jobs. Mixing them is the usual mistake.

### 11.1 Session / refresh `token_hash` — SHA-256, no port

The cookie value is 32 CSPRNG bytes (256 bits of entropy). A fast, unsalted (or peppered) hash is correct:

- Attacker who steals the D1 row still cannot mint the cookie without inverting SHA-256 of a 256-bit secret (infeasible).
- `/auth/refresh` must stay cheap so it does not become a CPU DoS.

**Do not use Argon2 here.** OWASP-minimum Argon2id is ~50–100 ms CPU in WASM; doing that on every refresh buys nothing and hurts the edge.

WASM deps (already compile-checked):

```toml
sha2 = "0.10"
subtle = "2"          # constant-time compare of hashes
# optional pepper:
hmac = "0.12"
```

`sha2` is pure Rust, no `getrandom`. Compare with `subtle::ConstantTimeEq`, not `==`.

Optional later: store `HMAC-SHA-256(SESSION_PEPPER, raw_token)` instead of bare SHA-256 so a DB dump is useless without the Worker secret. Not v1.

### 11.2 Passwords — Argon2id, **do** add a port

Argon2id is the right algorithm (OWASP first choice) **on native**. The RustCrypto **`argon2` 0.6** crate is `no_std`, compiles to `wasm32-unknown-unknown`, and does **not** need `getrandom` if we generate the salt ourselves. Compiling it on WASM is not the same as **running** it on Free Workers — see [§11.3](#113-free-workers-no-password-kdf-on-the-isolate).

```toml
# Native hasher (and a later Paid hasher Worker) — never enable crate defaults
argon2 = { version = "0.6", default-features = false, features = ["alloc", "password-hash", "zeroize"] }
```

| Feature | Use on WASM? |
|---------|----------------|
| `alloc` + `password-hash` | **Yes** — PHC strings (`$argon2id$v=19$m=…`) — compile-checked; **do not call this on Free** |
| `zeroize` | Yes |
| **`getrandom`** (in default features) | **No** — would reintroduce the WASM entropy footgun. Pass 16 salt bytes from `RandomSource`. |
| `parallel` / `rayon` | **No** — Workers have no threads. `p=1`. |

Hash API: `Argon2::hash_password(password, &salt)` → store the **full PHC string** in `auth_users.password_hash` (params travel with the hash, so we can raise costs later). Verify with `Argon2::verify_password` using the params **in the stored string**, not the current defaults.

Salt: `RandomSource::fill_bytes` → 16 bytes → `password_hash::SaltString`. This is why Random is a platform port and Argon2 itself is not.

### 11.3 Free Workers: no password KDF on the isolate

We are targeting **Workers Free**. That is a hard CPU ceiling, not a soft preference.

| Plan | CPU per request | Memory per isolate |
|------|-----------------|--------------------|
| Workers **Free** (this project) | **10 ms** | 128 MB |
| Workers **Paid** | 30 s default (up to 5 min) | 128 MB |

[Limits](https://developers.cloudflare.com/workers/platform/limits/): CPU is *execution* time. `fetch` / D1 / service-binding wait does **not** count; hashing **does**. Exceeding the cap is Error 1102 (`exceededCpu`). Cloudflare notes the isolate has some infrequent-overage slack, then starts killing invocations that stay over the limit.

| Job | Fits Free 10 ms? |
|-----|------------------|
| HMAC-SHA-256 JWT sign/verify (~200 bytes) | **Yes** |
| SHA-256 of a 32-byte session token | **Yes** |
| Web Crypto `PBKDF2-SHA-256` at the production cap (**100,000** iterations) | **No** — ~40 ms in published Worker measurements; [OWASP wants 600,000](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html); workerd **throws** above 100k rather than clamping ([Jul 2026 write-up](https://dxdev.com/blog/2026-07-09_cf-pbkdf2-workers-limit/), [workerd#1346](https://github.com/cloudflare/workerd/issues/1346)) |
| Argon2id OWASP `m=19456,t=2,p=1` in WASM | **No** — ~100 ms ([lucia + Rust hasher tutorial](https://mli.puffinsystems.com/blog/lucia-auth-cloudflare-argon2)) |
| Weakened Argon2 / bcrypt-cost-4 to squeeze under 10 ms | Fits the clock; **not an acceptable crate default** (GPU-cheap, below OWASP) |

**A second Worker via service binding does not help on Free.** The hasher isolate also has 10 ms. The lucia/argon2-cloudflare pattern needs **Paid** on the hasher. Durable Objects on Free inherit the same Worker CPU plan; do not treat them as an Argon2 escape hatch.

**Recommended approach (locked):**

1. **Session `token_hash`:** SHA-256 in-process on every platform, including Free. Do not Argon2 the cookie.
2. **Passwords:** Argon2id PHC strings, **only where CPU allows**.
   - **Native** (`cargo run -p rundtisch-demo --features native`): in-process Argon2id, OWASP `m=19456,t=2,p=1` (native can go heavier).
   - **Free Worker:** **do not hash or verify passwords**. Register/login on the Worker is **magic-link** — reuse the stateless activation JWT (`JWT_VERIFY_SECRET`). `auth_users.password_hash` stays `NULL` (already nullable for OAuth-only users).
3. **Upgrade path:** if password login must run on the edge later, add a **Paid** hasher Worker and call it over a service binding. Waiting is I/O on the main Worker; the hasher needs the Paid CPU budget. Keep that behind `PasswordHasher`.

**Rejected as crate defaults**

- SHA-256 / HMAC-SHA-256 of the password (even with a pepper). Fast hashes belong on session tokens, not passwords.
- Web Crypto PBKDF2 at 100k. Below OWASP, still over 10 ms, platform-capped.
- Client-side hashing. The hash becomes the password equivalent without OPAQUE/SRP (out of scope).
- Lowering Argon2 memory/time until it fits 10 ms, then shipping that as `WorkerArgon2`.

A pepper (`HMAC(PEPPER, password)` before Argon2, or HMAC of the session token) is still useful later as defense-in-depth. It is **not** a substitute for a slow KDF.

### 11.4 `PasswordHasher` port (sketch)

Not a `Platform` resource like D1; a policy object built at boot from `Random` + optional pepper secret + params.

```rust
pub trait PasswordHasher: Send + Sync {
    fn hash(&self, password: &str) -> Result<String, PasswordHashError>;
    fn verify(&self, password: &str, password_hash: &str) -> Result<bool, PasswordHashError>;
}
```

| Adapter | Behaviour |
|---------|-----------|
| `NativeArgon2` | RustCrypto Argon2id, PHC string, salt from `RandomSource`, OWASP (or heavier) params |
| `UnsupportedOnWorker` | `hash`/`verify` return a domain error — Free Worker uses magic-link instead |
| `PaidWorkerArgon2` | Later: same crate, OWASP params, behind a Paid service binding (`impl Future`) |
| `TestPasswordHasher` | Fast, deterministic (never production) |

Wire as a field on `AppState` constructed in `from_platform`. Do **not** merge it with `RandomSource` or a generic “Hasher” that also does SHA-256.

On native, `hash` / `verify` can stay **sync**. If we later call a Paid hasher Worker, the trait methods become `impl Future + Send` like `DatabaseExecutor`.

### 11.5 WASM dependency / port map (sessions + passwords)

| Need | Mechanism | WASM / Free issue |
|------|-----------|-------------------|
| Session token bytes | **`RandomSource` port** | `crypto.getRandomValues` vs OS |
| Session `token_hash` | `sha2` (+ optional `hmac` pepper) | None — fits 10 ms |
| JWT access token | `jwt-compact` HS256 | None if HMAC-only — fits 10 ms |
| JWT `exp`, `expires_at` | **`Clock` port** | `time` + `wasm-bindgen` already on `d1` |
| Signing / pepper secrets | **`SecretStore` port** (already stubbed) | Worker secrets vs env |
| Password salt | **`RandomSource`** (native hasher only) | Same as session token |
| Password hash / verify | **`PasswordHasher` port** → native Argon2id; **no KDF on Free Worker** | 10 ms CPU; magic-link instead |

No extra port for “hashing in general.”
