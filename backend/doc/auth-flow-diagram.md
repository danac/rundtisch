# Registration & Authentication Flow

Web frontend ↔ REST API with **email activation**, **JWT access tokens**, and **rotating refresh tokens**.

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
 (Browser)      (Web app)       (Backend)      (Mailer)    (Users + tokens)
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
  |               |               |     hashed pwd, verified=false |
  |               |               | ... store activation token --->|
  |               |               |              |               |
  |               |               |-- send activation email ---->|
  |               |               |   (link + token)             |
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
  |               |               | ... validate token, set ---->|
  |               |               |     email_verified = true    |
  |               |               |     invalidate token         |
  |               |               |              |               |
  |               |<-- [200] Activated ----------|               |
  |               |    (optional auto-login)      |               |
  |               |               |              |               |
  |               |    +--[ auto-login ]--------+               |
  |               |    |  ... create refresh token ------------>|
  |               |    |  access JWT + Set-Cookie refresh      |
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
  |               |       ... store refresh token --------------->|
  |               |       sign access JWT                         |
  |               |<-- [200] access JWT + Set-Cookie refresh -----|
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
  |               |               | ... verify JWT signature      |
  |               |               |     check exp, claims         |
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
  |               |   Cookie: refreshToken        |               |
  |               |   (HttpOnly, Secure)          |               |
  |               |               |              |               |
  |               |               | ... validate refresh token ->|
  |               |               |     rotate: revoke old,      |
  |               |               |             issue new          |
  |               |               |              |               |
  |               |<-- [200] new access JWT -----|               |
  |               |    Set-Cookie: refreshToken   |               |
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
  |               |   Cookie: refreshToken        |               |
  |               |               |              |               |
  |               |               | ... revoke refresh token ---->|
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
| `POST` | `/auth/login` | Issue access JWT + refresh cookie |
| `POST` | `/auth/refresh` | Rotate tokens when access JWT expires |
| `POST` | `/auth/logout` | Revoke refresh token / clear cookie |
| `GET` | `/api/*` | Protected resources (`Authorization: Bearer …`) |

---

## Token summary

| Token | Lifetime | Storage (frontend) | Used for |
|-------|----------|--------------------|----------|
| **Access JWT** | Short (5–15 min) | Memory (auth context) | Every API request |
| **Refresh token** | Long (days/weeks) | HttpOnly Secure cookie | Only `/auth/refresh` and `/auth/logout` |
