# JWT on Cloudflare Workers: crate choice, crypto/RNG, and hexagonal ports

**Date:** 2026-09-21  
**Status:** investigation only — no implementation in this change  
**Related:** [auth-flow-diagram.md](./auth-flow-diagram.md), [20260708-150107-RustCloudflareWorkerAuthenticationDesign.md](./20260708-150107-RustCloudflareWorkerAuthenticationDesign.md)

This note records a crate and architecture survey before we implement JWT access tokens. The July 2026 design recommended `jsonwebtoken 9` + `ring` + `getrandom 0.2` with the `js` feature. That advice is **out of date**: `jsonwebtoken` 11 dropped `ring`, WASM entropy configuration changed across `getrandom` 0.2/0.3/0.4, and HMAC-only signing does not need a CSPRNG at all.

Compile probes below used **rustc 1.98.1** (current `stable`, matching CI `dtolnay/rust-toolchain@stable`) and `--target wasm32-unknown-unknown`.

---

## 1. What auth actually needs from crypto

From the existing flow ([auth-flow-diagram.md](./auth-flow-diagram.md)) and schema (`auth_users`, `auth_refresh_tokens.token_hash`):

| Operation | Algorithm | Needs CSPRNG? | Platform-specific? |
|-----------|-----------|---------------|--------------------|
| Sign / verify **access JWT** | HMAC-SHA-256 (recommended for v1) | **No** — deterministic given secret + payload | No, if we stay on pure-Rust HMAC |
| Sign / verify **email-activation JWT** (optional, stateless) | HMAC-SHA-256, **different secret** | No | No |
| **Refresh / session token** (opaque, HttpOnly cookie) | 32+ random bytes, store **SHA-256** (or HMAC-SHA-256 + pepper) | **Yes** (the token) | Entropy source differs (OS vs Workers `crypto.getRandomValues`) |
| OAuth `state`, WebAuthn challenge (later) | random bytes | **Yes** | Same as refresh |
| JWT `jti` (optional replay id) | random bytes or UUID | **Yes** if used | Same |
| Hash stored session token | **SHA-256**, not Argon2 | No | No — high-entropy secret |
| **Password** | **Argon2id** (PHC string) | **Yes** (per-password salt) | Algorithm is portable Rust; **CPU/memory params** are platform-specific |

So: **JWT HMAC and SHA-256 are portable library problems. Random bytes are a platform problem. Argon2id is a portable library with a platform-specific budget.** Do not put session tokens and passwords through the same hash.

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

**Recommendation:** pick a JWT crate whose HS256 path has **zero** `getrandom` dependency, and implement Worker entropy ourselves via `crypto.getRandomValues`. Do not add `.cargo/config.toml` rustflags unless a future crate forces `getrandom` 0.3.3.

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

### 4.4 Recommendation

**Use `jwt-compact` 0.8** in `rundtisch`, HMAC-only:

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

**Fallback** if jwt-compact looks unmaintained when we implement: `jsonwebtoken` 11 + `rust_crypto` + explicit `getrandom` 0.2 `js` on the WASM target, and check `exp` ourselves via the Clock port (`Validation { validate_exp: false, .. }` plus manual compare). Optionally an HMAC-only `CryptoProvider` to drop RSA from the binary.

**Do not** follow the July doc’s `jsonwebtoken = "9"` + `ring` setup for new code.

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

**Adapters** (same layering as DB: traits in `rundtisch`, impls in `adapters`, platforms in `demo/` construct them):

| Adapter | Where | Backend |
|---------|--------|---------|
| `OsRandom` | `rundtisch`, `#[cfg(not(target_arch = "wasm32"))]` | `getrandom::fill` (0.2 or 0.3 — native needs no extra feature) |
| `WorkerRandom` | `rundtisch` `d1` feature | `js_sys` call to global `crypto.getRandomValues` (copy into `&mut [u8]`). **Do not** go through `getrandom` so the WASM graph stays free of `wasm_js` unless some other crate pulls it. |
| `ReplayRandom` / seeded ChaCha | tests / `#[cfg(test)]` | Deterministic; never used in production platforms |

Wire through `Platform`:

```rust
pub trait Platform: 'static {
    type Database: DatabaseExecutor + Send + Sync;
    type Random: RandomSource + Send + Sync;
    fn database(&self) -> Arc<Self::Database>;
    fn random(&self) -> Arc<Self::Random>;
}
```

`AppState` grows `random: Arc<P::Random>`. `NativePlatform` / `CloudflarePlatform` / `TestPlatform` supply the matching adapter.

**Domain use (later, not this PR):** 32-byte session token → base64url → SHA-256 (`sha2`, not a port) → store hex/bytes in `auth_sessions.token_hash`. Never log or persist the raw token. Same `RandomSource` later supplies the 16-byte salt for Argon2id (do **not** enable `argon2`’s `getrandom` feature on WASM).

Error type: small `RandomError` (`Unavailable` / `Backend(String)`), not `traits::db::Error`.

### 5.2 Clock — **yes, add `Clock` in the same increment as Random**

Handlers already call `time::OffsetDateTime::now_utc()` (`auth/models.rs`). JWT `iat`/`exp` and refresh `expires_at` need the same clock. Tests cannot freeze `now_utc()`. jwt-compact’s default clock is the wrong crate and panics on WASM.

```rust
pub trait Clock: Send + Sync {
    fn now_utc(&self) -> time::OffsetDateTime;
}
```

- `SystemClock`: `OffsetDateTime::now_utc()` — already correct on WASM when `d1` enables `time/wasm-bindgen`.
- `FrozenClock`: tests.

Not async. JWT adapter converts to unix seconds / `chrono` only at the jwt-compact boundary.

### 5.3 Secrets — already planned; needed before signing

Commented `SecretStore` on `Platform` is the right place for `JWT_ACCESS_SECRET` and `JWT_VERIFY_SECRET` (separate keys; see July design). Implementation can stay a follow-up, but **signing must not hard-code secrets**. Minimum v1: `get(name) -> Result<String>` from Worker secrets / env vars.

HS256 secret: **≥ 256 bits** (32 bytes). jwt-compact can enforce this via `StrongKey`.

### 5.4 JWT issuer — **library adapter, not a Platform port**

Signing is CPU-only and the same on native and WASM if we stay on hmac+sha2. Making `TokenIssuer` a `Platform` associated type would imply a Worker WebCrypto adapter we do not want for v1.

Put a small **in-crate service** next to auth (e.g. `auth/jwt.rs` or `adapters/jwt.rs`) that:

- takes `&dyn Clock` (or `Arc<P::Clock>`) and the secret bytes;
- encodes/decodes typed claims with `Hs256`;
- pins `token_type` / `iss` in claims (`access` vs `email_verification`);
- maps crate errors to a domain `TokenError`.

Handler tests can use a real HS256 key and `FrozenClock`. A `TokenIssuer` trait is optional sugar; do not add it until a second implementation exists.

### 5.5 Password hasher — **yes, a port; not the same as session hashing**

See [§11](#11-hashing-on-wasm-sessions-vs-passwords). Session `token_hash` stays a library call (`sha2`). Passwords get a `PasswordHasher` port so tests, Worker params, and a later service-binding offload can differ without touching handlers.

### 5.6 What we will **not** port

- SHA-256 / HMAC-SHA-256 (`sha2` + `hmac` + `subtle`) — pure Rust, same on native and WASM.
- Email sending — HTTP to Resend; its own port later.

---

## 6. Suggested claims (v1)

Access token (Bearer, 5–15 minutes, memory on the client):

```json
{
  "sub": "123",
  "role": "User",
  "typ": "access",
  "iss": "rundtisch",
  "iat": 0,
  "exp": 0
}
```

Email activation JWT (if we skip an activation-token table): `typ: "email_verification"`, include `email`, sign with **`JWT_VERIFY_SECRET`**, short `exp`. Verify with `UPDATE … WHERE id = ? AND email = ?` as in the July note.

Do not put the refresh token in a JWT. Opaque random + DB hash + rotation stays as designed.

Verification must:

1. Use **only** `Hs256` (typed API).
2. Reject wrong `typ` (activation JWT is not an access token).
3. Check `exp` via Clock + small leeway (jwt-compact default leeway is 60s if we use `TimeOptions`).
4. Not trust a client-supplied `alg` header.

---

## 7. Implementation sequence (when we start coding)

Do not combine all of this with login/refresh in one PR.

1. **Ports only:** `RandomSource` + `Clock`, adapters, `Platform` / `AppState` / native + Worker + test platforms. Replace `now_utc()` in auth models/handlers with `state.clock`. Unit-test `WorkerRandom` shape with a thin JS mock if feasible; always test `OsRandom` + `FrozenClock` + `ReplayRandom`.
2. **JWT helper:** add `jwt-compact` (HMAC-only), claims structs, encode/decode tests (native). `cargo check -p rundtisch --features d1 --target wasm32-unknown-unknown` in CI (already used on some PRs). Still no HTTP.
3. **Secrets + sessions + login:** `SecretStore`, issue access JWT, persist `auth_sessions` row (Random + SHA-256), Argon2id `PasswordHasher` with Worker-safe params, Axum extractor/middleware for `Authorization: Bearer`.

CI should keep compiling the Worker target on every PR that touches crypto features.

---

## 8. Open questions

1. **HS256 vs Ed25519/ES256 for v1?** Recommendation: **HS256**. This process is both issuer and verifier; Worker secrets hold a shared key; no RNG in the signer; smallest WASM. Asymmetric only if another service must verify without the secret.
2. **Access-token claims:** `sub` + `role` only, or also `email` / `alias`? Role in the token avoids a D1 hit on every request but delays revocation of role changes until expiry (5–15 min).
3. **Email activation:** stateless JWT (July follow-up) vs hashed token in DB? Schema today has `email_verified_at` and **no** activation table. JWT fits that.
4. **jwt-compact 0.8 age:** accept 2024 stable, or prefer `jsonwebtoken` 11 + `getrandom` 0.2 `js` for maintenance? This note prefers jwt-compact; easy to reverse in step 2 before HTTP exists.
5. **Clock in the same PR as Random?** Recommendation: **yes** — JWT and `expires_at` need it, and `now_utc()` is already scattered.
6. **Refresh token encoding:** raw 32 bytes as URL-safe base64 (no padding), 256-bit. Cookie flags: `HttpOnly`, `Secure`, `SameSite=Lax` or `Strict`? SPA on the same origin can use Strict; confirm cookie vs JSON body for native API clients.
7. **`jti` on access tokens?** Only useful with a denylist; skip for v1 to keep the API stateless.
8. **Secret rotation / `kid`?** Skip for v1; one access secret, one verify secret.
9. **Workers plan for Argon2id?** OWASP `m=19456,t=2,p=1` needs ~100 ms CPU — **Paid**, not Free (10 ms). Confirm the demo account is Paid before login ships.
10. **Session token hash:** bare SHA-256 vs HMAC-SHA-256 with a `SESSION_PEPPER` secret?

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

Optional hardening: store `HMAC-SHA-256(SESSION_PEPPER, raw_token)` instead of bare SHA-256 so a DB dump is useless without the Worker secret (`SecretStore`). Still not a port.

### 11.2 Passwords — Argon2id, **do** add a port

Argon2id is the right algorithm (OWASP first choice). The RustCrypto **`argon2` 0.6** crate is `no_std`, compiles to `wasm32-unknown-unknown`, and does **not** need `getrandom` if we generate the salt ourselves.

```toml
# Worker / lib — do not use crate defaults
argon2 = { version = "0.6", default-features = false, features = ["alloc", "password-hash", "zeroize"] }
```

| Feature | Use on WASM? |
|---------|----------------|
| `alloc` + `password-hash` | **Yes** — PHC strings (`$argon2id$v=19$m=…`) |
| `zeroize` | Yes |
| **`getrandom`** (in default features) | **No** — would reintroduce the WASM entropy footgun. Pass 16 salt bytes from `RandomSource`. |
| `parallel` / `rayon` | **No** — Workers have no threads. `p=1`. |

Hash API: `Argon2::hash_password(password, &salt)` → store the **full PHC string** in `auth_users.password_hash` (params travel with the hash, so we can raise costs later). Verify with `Argon2::verify_password` using the params **in the stored string**, not the current defaults.

Salt: `RandomSource::fill_bytes` → 16 bytes → `password_hash::SaltString`. This is why Random is a platform port and Argon2 itself is not.

### 11.3 Worker CPU / memory (this is the real constraint)

| Plan | CPU per request | Memory per isolate |
|------|-----------------|--------------------|
| Workers **Free** | **10 ms** | 128 MB |
| Workers **Paid** | 30 s default (up to 5 min) | 128 MB |

OWASP minimum Argon2id: **m=19456 (19 MiB), t=2, p=1**. Public measurements of Rust Argon2id at those params on a CF Worker are on the order of **~100 ms CPU** — fine on Paid, **impossible on Free**. 19 MiB is comfortable inside 128 MB if we are not hashing many passwords concurrently on one isolate.

Web Crypto on Workers offers **PBKDF2**, not Argon2, and production PBKDF2 is capped (historically 100k iterations). Do not use it as the primary password KDF.

Practical policy:

- Target **Paid Workers** for register/login, or accept that Free cannot do real Argon2id.
- Use **p=1**, OWASP `m=19456,t=2` on the Worker; native can use a heavier preset (`m=47104,t=1` or higher) — that difference is why a port exists.
- Login/register only. Never on the Bearer path.

A dedicated hashing Worker + service binding (the lucia/argon2-cloudflare pattern) is a later optimisation if login CPU shows up in metrics. A `PasswordHasher` port lets us move there without rewriting handlers.

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
| `Argon2Hasher` | RustCrypto Argon2id, PHC string, salt from `RandomSource`, optional pepper from `SecretStore` |
| `WorkerArgon2` | Same crate, Worker params (`m=19456,t=2,p=1`) |
| `NativeArgon2` | Same crate, stronger params |
| `TestPasswordHasher` | Fast, deterministic (never production) |

Wire as `type PasswordHasher: …` on `Platform` **or** as a field on `AppState` constructed in `from_platform`. Either is fine; do **not** merge it with `RandomSource` or a generic “Hasher” that also does SHA-256.

`hash` / `verify` can stay **sync**. Argon2 is CPU-bound; making it async does not yield the isolate (no threads). If we later call a service binding, the trait methods become `impl Future + Send` like `DatabaseExecutor`.

### 11.5 WASM dependency / port map (sessions + passwords)

| Need | Mechanism | WASM issue |
|------|-----------|------------|
| Session token bytes | **`RandomSource` port** | `crypto.getRandomValues` vs OS |
| Session `token_hash` | `sha2` (+ optional `hmac` pepper) | None |
| JWT access token | `jwt-compact` HS256 | None if HMAC-only |
| JWT `iat`/`exp`, `expires_at` | **`Clock` port** | `time` + `wasm-bindgen` already on `d1` |
| Signing / pepper secrets | **`SecretStore` port** (already stubbed) | Worker secrets vs env |
| Password salt | **`RandomSource`** again | Same as session token |
| Password hash / verify | **`PasswordHasher` port** → `argon2` 0.6 without `getrandom`/`parallel` | CPU ~100 ms; Paid plan; 19 MiB RAM |

No extra port for “hashing in general.”
