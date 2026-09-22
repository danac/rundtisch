# Registration & Authentication Flow

Web frontend ↔ REST API with **stateless email-activation JWTs**, **HS256 access JWTs** (`sub` / `exp` / `role`), and **rotating session cookies** stored as hashed rows in `auth_sessions`.

## Legend

| Symbol | Meaning |
|--------|---------|
| `──>` | Request |
| `<──` | Response |
| `...` | Internal / DB operation |
| `[401]` | Unauthorized (expired access token) |
| `[403]` | Forbidden (email not verified) |

## Actors

```
  User          Frontend        REST API        Email           Database
 (Browser)      (Web app)       (Backend)      (Mailer)    (auth_users + auth_sessions)
    |               |               |              |               |
```

---

## Phase 1 — Registration

```
User          Frontend        REST API        Email           Database
  |               |               |              |               |
  |-- register -->|               |              |               |
  |  (email,pwd)  |               |              |               |
  |               |-- POST /auth/register ----->|               |
  |               |               |              |               |
  |               |               | ... create user ------------>|
  |               |               |     Argon2id hash,           |
  |               |               |     email_verified_at = NULL |
  |               |               | ... sign activation JWT      |
  |               |               |     (stateless; no DB row)   |
  |               |               |              |               |
  |               |               |-- send activation email ---->|
  |               |               |   (link + activation JWT)    |
  |               |               |              |               |
  |               |<-- [201] Created ------------|               |
  |               |    (no session tokens)        |               |
  |               |               |              |               |
  |<-- "Check your email to activate" ----------|               |
  |               |               |              |               |
```

---

## Phase 2 — Email activation

```
User          Frontend        REST API        Email           Database
  |               |               |              |               |
  |-- open link from inbox ---------------------->|               |
  |               |               |              |               |
  |-- open /activate?token=... -->|               |              |               |
  |               |               |              |               |
  |               |-- POST /auth/activate ----->|               |
  |               |   { token }   |              |               |
  |               |               |              |               |
  |               |               | ... verify activation JWT -->|
  |               |               |     UPDATE email_verified_at |
  |               |               |     (no token table to clear)|
  |               |               |              |               |
  |               |<-- [200] Activated ----------|               |
  |               |    (optional auto-login)      |               |
  |               |               |              |               |
  |               |    +--[ auto-login ]--------+               |
  |               |    |  ... insert auth_sessions ------------>|
  |               |    |  access JWT + Set-Cookie session      |
  |               |    +-- enter app (authenticated)            |
  |               |    |                                          |
  |               |    +--[ login required ]------+               |
  |<-- "Account activated — please log in" -------|               |
  |               |               |              |               |
```

---

## Phase 3 — Login

```
User          Frontend        REST API        Email           Database
  |               |               |              |               |
  |-- credentials->|               |              |               |
  |               |               |              |               |
  |               |-- POST /auth/login --------->|               |
  |               |               |              |               |
  |               |               | ... verify password -------->|
  |               |               |     check email_verified     |
  |               |               |              |               |
  |               |    +--[ email NOT verified ]--+               |
  |               |    |  <-- [403] Email not verified             |
  |<-- resend activation / check email ---------|               |
  |               |    |                                          |
  |               |    +--[ email verified ]------+               |
  |               |       ... insert auth_sessions --------------->|
  |               |       sign access JWT {sub,exp,role}          |
  |               |<-- [200] access JWT + Set-Cookie session -----|
  |               |    (HttpOnly, Secure, SameSite=Strict)        |
  |               |               |              |               |
```

---

## Phase 4 — Authenticated API usage

```
User          Frontend        REST API        Email           Database
  |               |               |              |               |
  |               |-- GET /api/resource ------->|               |
  |               |   Authorization: Bearer     |               |
  |               |   <accessJWT>               |               |
  |               |               |              |               |
  |               |               | ... verify HS256, check exp   |
  |               |               |     claims: sub, exp, role    |
  |               |               |              |               |
  |               |<-- [200] Resource JSON ------|               |
  |               |               |              |               |
        (repeat for normal API calls while access token is valid)
```

---

## Phase 5 — Expired access token → refresh

```
User          Frontend        REST API        Email           Database
  |               |               |              |               |
  |               |         *** Access JWT expired ***           |
  |               |               |              |               |
  |               |-- GET /api/resource ------->|               |
  |               |   Bearer <expired accessJWT>  |               |
  |               |               |              |               |
  |               |<-- [401] Unauthorized -------|               |
  |               |    { "error": "token_expired" }               |
  |               |               |              |               |
  |               |-- POST /auth/refresh ------->|               |
  |               |   Cookie: session (HttpOnly,  |               |
  |               |            Secure, Strict)    |               |
  |               |               |              |               |
  |               |               | ... lookup token_hash ------>|
  |               |               |     reject if revoked/expired|
  |               |               |     UPDATE token_hash +      |
  |               |               |            last_used_at      |
  |               |               |              |               |
  |               |<-- [200] new access JWT -----|               |
  |               |    Set-Cookie: session        |               |
  |               |               |              |               |
  |               |-- retry original request ---->|               |
  |               |   Bearer <new accessJWT>      |               |
  |               |               |              |               |
  |               |<-- [200] Resource JSON ------|               |
  |               |               |              |               |
```

---

## Phase 6 — Logout

```
User          Frontend        REST API        Email           Database
  |               |               |              |               |
  |-- logout ---->|               |              |               |
  |               |               |              |               |
  |               |-- POST /auth/logout --------->|               |
  |               |   Cookie: session             |               |
  |               |               |              |               |
  |               |               | ... SET revoked_at ---------->|
  |               |               |              |               |
  |               |<-- [204] Clear cookie -------|               |
  |               |               |              |               |
  |               | ... drop access token from memory              |
  |<-- redirect to login ----------|              |               |
  |               |               |              |               |
```

---

## Full end-to-end overview

```
┌─────────┐   ┌──────────┐   ┌──────────┐   ┌───────┐   ┌──────────┐
│  User   │   │ Frontend │   │ REST API │   │ Email │   │ Database │
└────┬────┘   └────┬─────┘   └────┬─────┘   └───┬───┘   └────┬─────┘
     │             │              │             │            │
     │  register   │              │             │            │
     ├────────────>│ POST /register├─────────────┼───────────>│
     │             │              │ send mail   ├───────────>│
     │             │<── 201 ──────┤             │            │
     │<────────────┤              │             │            │
     │             │              │             │            │
     │ open link   │              │             │            │
     ├────────────>│ POST /activate├────────────┼───────────>│
     │             │<── 200 ──────┤             │            │
     │             │              │             │            │
     │ login       │              │             │            │
     ├────────────>│ POST /login  ├─────────────┼───────────>│
     │             │<── 200 + JWT ┤             │            │
     │             │              │             │            │
     │             │ API + Bearer ├─────────────┼───────────>│
     │             │<── 200 ──────┤             │            │
     │             │              │             │            │
     │             │ API (expired)├─────────────┼───────────>│
     │             │<── 401 ──────┤             │            │
     │             │ POST /refresh├─────────────┼───────────>│
     │             │<── 200 + JWT ┤             │            │
     │             │ retry API    ├─────────────┼───────────>│
     │             │<── 200 ──────┤             │            │
     │             │              │             │            │
     │ logout      │              │             │            │
     ├────────────>│ POST /logout ├─────────────┼───────────>│
     │             │<── 204 ──────┤             │            │
     │<────────────┤              │             │            │
     │             │              │             │            │
```

---

## Typical REST endpoints

| Method | Path | Purpose |
|--------|------|---------|
| `POST` | `/auth/register` | Create account (unverified) |
| `POST` | `/auth/activate` | Confirm email with token from link |
| `POST` | `/auth/resend-activation` | Resend activation email (rate-limited) |
| `POST` | `/auth/login` | Issue access JWT + session cookie |
| `POST` | `/auth/refresh` | Rotate session `token_hash` when access JWT expires |
| `POST` | `/auth/logout` | Set `revoked_at` / clear cookie |
| `GET` | `/api/*` | Protected resources (`Authorization: Bearer …`) |

---

## Persistence (v1)

| Table | Columns |
|-------|---------|
| **auth_users** | `id` (integer PK, not in JSON), `public_id` (UUIDv4 UNIQUE; JWT `sub` / API paths / WebAuthn `userHandle`), `email`, `alias`, `role`, `password_hash`, `email_verified_at`, `created_at`, `updated_at`, `last_login_at` |
| **auth_sessions** | `id`, `user_id`, `token_hash` (HMAC-SHA-256 + `AUTH_HASH_PEPPER`, unique), `created_at`, `last_used_at`, `expires_at`, `revoked_at`, `user_agent` |

Email activation is a **stateless JWT** (separate secret). No activation table.

Worker and native use the **same** Argon2id: **unique 16-byte salt per password** (inside the PHC in `password_hash`, not a separate column) plus `AUTH_HASH_PEPPER` from `SecretStore`.

---

## Token summary

| Token | Lifetime | Storage (frontend) | Used for |
|-------|----------|--------------------|----------|
| **Access JWT** | Short (5–15 min) | Memory (auth context) | Every API request (`sub` = `public_id`, `exp`, `role`; HS256) |
| **Activation JWT** | Hours | Email link only | `/auth/activate` |
| **Session / refresh token** | Long (days/weeks) | HttpOnly Secure `SameSite=Strict` cookie | Only `/auth/refresh` and `/auth/logout` |
