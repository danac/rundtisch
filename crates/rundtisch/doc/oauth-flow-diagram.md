# Registration & Authentication Flow — OAuth (OIDC)

Web frontend ↔ REST API with **OAuth 2.0 / OpenID Connect** social sign-in, **JWT access tokens**, and **rotating refresh tokens**.

Passwordless model: the user proves identity at an **OAuth provider** (Google, GitHub, etc.). The backend exchanges the authorization code server-side, trusts the provider’s verified email, and issues the same JWT session used for password auth.

## Legend

| Symbol | Meaning |
|--------|---------|
| `──>` | Request / redirect |
| `<──` | Response |
| `...` | Internal or DB operation |
| `~~~` | Browser redirect chain (OAuth consent UI) |
| `[401]` | Unauthorized (expired access JWT) |
| `[409]` | Conflict (email already registered with another method) |

## Actors

```
  User       Frontend      REST API    OAuth Provider    Database
 (Browser)  (Web app)      (Backend)   (Google/GitHub)  (Users, oauth
    |            |             |              |          accounts, states,
    |            |             |              |          refresh tokens)
```

---

## Phase 1 — Start OAuth (shared redirect)

Registration and login both begin with a redirect to the provider. The `intent` query param guides post-callback handling; the OAuth ceremony itself is identical.

```
User       Frontend      REST API    OAuth Provider    Database
  |            |             |              |              |
  |-- "Sign up   |             |              |              |
  |   with Google"|             |              |              |
  |            |             |              |              |
  |            |-- GET /auth/oauth/google/authorize ------>|
  |            |   ?intent=register                          |
  |            |   &redirect_uri=…                           |
  |            |   &code_challenge=… (PKCE S256)             |
  |            |             |              |              |
  |            |             | ... generate state, store -->|
  |            |             |     code_verifier (PKCE)     |
  |            |             |     intent, expires_at ~10m  |
  |            |             |              |              |
  |            |<-- [302] authorization_url ----------------|
  |            |   Location: accounts.google.com/o/oauth2…  |
  |            |   ?client_id&redirect_uri&state&          |
  |            |   code_challenge&scope=openid email       |
  |            |             |              |              |
  | ~~~ browser redirect to provider consent screen ~~~~~~~~|
  |            |             |              |              |
  |-- navigate to IdP ---------------------->|              |
  |            |             |              |              |
```

For **login**, the frontend uses the same endpoint with `?intent=login`.

---

## Phase 2 — Provider consent & callback

```
User       Frontend      REST API    OAuth Provider    Database
  |            |             |              |              |
  |-- sign in / consent at provider ------->|              |
  |            |             |              |              |
  |            |             |              | ... user auth |
  |            |             |              |   + consent   |
  |            |             |              |              |
  |<-- [302] redirect to callback ----------|              |
  |   /auth/oauth/google/callback            |              |
  |   ?code=…&state=…                        |              |
  |            |             |              |              |
  |-- GET /auth/oauth/google/callback ----->|              |
  |   ?code&state            |              |              |
  |            |             |              |              |
  |            |             | ... load oauth_state by state>|
  |            |             |     validate TTL, single-use |
  |            |             |              |              |
  |            |             |-- POST /token ------------->|
  |            |             |   code + client_secret     |
  |            |             |   + code_verifier (PKCE)   |
  |            |             |              |              |
  |            |             |<-- id_token + access_token --|
  |            |             |              |              |
  |            |             |-- GET /userinfo (or decode -->|
  |            |             |   id_token claims)           |
  |            |             |              |              |
  |            |             |<-- { sub, email,             |
  |            |             |      email_verified, name } -|
  |            |             |              |              |
  |            |             | ... consume oauth_state ---->|
  |            |             |              |              |
        (branch: registration vs login — Phases 3A / 3B)
```

---

## Phase 3A — OAuth registration (new user)

Provider `sub` is unknown. A new local account is created; email is marked verified when the IdP asserts `email_verified`.

```
User       Frontend      REST API    OAuth Provider    Database
  |            |             |              |              |
  |            |             | ... lookup oauth_accounts -->|
  |            |             |     by (provider, sub)       |
  |            |             |     → not found              |
  |            |             |              |              |
  |            |             | ... lookup users by email -->|
  |            |             |     → not found              |
  |            |             |              |              |
  |            |             | ... create user ----------->|
  |            |             |     email, display_name      |
  |            |             |     email_verified = true    |
  |            |             |     (IdP email_verified)     |
  |            |             |     no password hash         |
  |            |             |              |              |
  |            |             | ... create oauth_account -->|
  |            |             |     provider, provider_sub   |
  |            |             |     user_id                  |
  |            |             |              |              |
  |            |             | ... create refresh token --->|
  |            |             |     sign access JWT          |
  |            |             |              |              |
  |            |<-- [302] /oauth/success ------------------|
  |            |   Set-Cookie: refreshToken (HttpOnly)      |
  |            |   ?access_token=… (one-time fragment     |
  |            |    or secure cookie — app reads once)    |
  |            |             |              |              |
  |            | ... store access JWT in memory           |
  |<-- enter app (authenticated) ------------|              |
  |            |             |              |              |
```

If `email_verified` from the IdP is **false**, registration is rejected with `[403]` — do not create an unverified OAuth account.

---

## Phase 3B — OAuth login (returning user)

Provider `sub` already linked to a local user.

```
User       Frontend      REST API    OAuth Provider    Database
  |            |             |              |              |
  |            |             | ... lookup oauth_accounts -->|
  |            |             |     by (provider, sub)       |
  |            |             |     → user_id found          |
  |            |             |              |              |
  |            |             | ... load user -------------->|
  |            |             |     check email_verified     |
  |            |             |              |              |
  |            |             | ... create refresh token --->|
  |            |             |     sign access JWT          |
  |            |             |              |              |
  |            |<-- [302] /oauth/success ------------------|
  |            |   Set-Cookie: refresh + access handoff     |
  |            |             |              |              |
  |            | ... store access JWT in memory             |
  |<-- enter app (authenticated) ------------|              |
  |            |             |              |              |
```

If `intent=login` but no `oauth_account` exists → `[404]` or redirect to register with message “No account linked to this provider”.

---

## Phase 3C — Email already registered (account linking)

OAuth `sub` is new, but the provider email matches an existing local account (e.g. password registration).

```
User       Frontend      REST API    OAuth Provider    Database
  |            |             |              |              |
  |            |             | ... lookup oauth_accounts -->|
  |            |             |     by (provider, sub) → none|
  |            |             | ... lookup users by email -->|
  |            |             |     → existing user found    |
  |            |             |              |              |
  |            |<-- [409] Conflict ------------------------|
  |            |   { "error": "email_exists",                |
  |            |     "link_url": "/auth/oauth/link/…" }     |
  |            |             |              |              |
  |<-- "Log in with password to link Google" -|              |
  |            |             |              |              |
  |-- password login ------->|              |              |
  |            |-- POST /auth/login ------->|              |
  |            |<-- [200] access JWT --------|              |
  |            |             |              |              |
  |-- "Link Google" -------->|              |              |
  |            |-- POST /auth/oauth/google/link ---------->|
  |            |   Bearer <accessJWT>                       |
  |            |   { code, state }  (from stored callback   |
  |            |     or re-run authorize with intent=link)  |
  |            |             |              |              |
  |            |             | ... verify user session      |
  |            |             | ... exchange code, get sub -->|
  |            |             | ... create oauth_account --->|
  |            |             |              |              |
  |            |<-- [200] Provider linked -----------------|
  |            |             |              |              |
```

---

## Phase 4 — Authenticated API usage

```
User       Frontend      REST API    OAuth Provider    Database
  |            |             |              |              |
  |            |-- GET /api/resource ------->|              |
  |            |   Authorization: Bearer     |              |
  |            |   <accessJWT>               |              |
  |            |             |              |              |
  |            |             | ... verify JWT signature    |
  |            |             |     check exp, claims         |
  |            |             |              |              |
  |            |<-- [200] Resource JSON -----|              |
  |            |             |              |              |
        (OAuth provider is NOT involved in routine API calls)
```

---

## Phase 5 — Expired access token → refresh

```
User       Frontend      REST API    OAuth Provider    Database
  |            |             |              |              |
  |            |         *** Access JWT expired ***         |
  |            |             |              |              |
  |            |-- GET /api/resource ------->|              |
  |            |   Bearer <expired accessJWT>  |              |
  |            |             |              |              |
  |            |<-- [401] Unauthorized -------|              |
  |            |   { "error": "token_expired" }               |
  |            |             |              |              |
  |            |-- POST /auth/refresh ------->|              |
  |            |   Cookie: refreshToken        |              |
  |            |   (HttpOnly, Secure)          |              |
  |            |             |              |              |
  |            |             | ... validate refresh token ->|
  |            |             |     rotate: revoke old,        |
  |            |             |             issue new          |
  |            |             |              |              |
  |            |<-- [200] new access JWT -----|              |
  |            |    Set-Cookie: refreshToken   |              |
  |            |             |              |              |
  |            |-- retry original request ---->|              |
  |            |   Bearer <new accessJWT>      |              |
  |            |             |              |              |
  |            |<-- [200] Resource JSON ------|              |
  |            |             |              |              |
```

If refresh also fails → clear session → redirect to login (OAuth or password).

---

## Phase 6 — Logout

```
User       Frontend      REST API    OAuth Provider    Database
  |            |             |              |              |
  |-- logout ->|             |              |              |
  |            |             |              |              |
  |            |-- POST /auth/logout -------->|              |
  |            |   Cookie: refreshToken        |              |
  |            |             |              |              |
  |            |             | ... revoke refresh token ---->|
  |            |             |              |              |
  |            |<-- [204] Clear cookie -------|              |
  |            |             |              |              |
  |            | ... drop access JWT from memory            |
  |<-- redirect to login -------|              |              |
  |            |             |              |              |
```

Logout is **local only** — it does not revoke the user’s session at the OAuth provider.

---

## Full end-to-end overview

```
┌──────┐ ┌──────────┐ ┌──────────┐ ┌──────────────┐ ┌──────────┐
│ User │ │ Frontend │ │ REST API │ │OAuth Provider│ │ Database │
└──┬───┘ └────┬─────┘ └────┬─────┘ └──────┬───────┘ └────┬─────┘
   │          │            │              │            │
   │ register │ GET /oauth/google/authorize             │
   │ with Google────────>│──────────────┼───────────>│ state+PKCE
   ├─────────>│<── 302 IdP URL ──────────┤            │
   │ redirect ├─────────────────────────>│ consent    │
   │<─────────┼────────────┼──────────────┤            │
   │ callback │ GET /callback?code&state │            │
   ├─────────>│───────────>│ POST /token ├───────────>│
   │          │            │ userinfo     │            │
   │          │            ├──────────────┼───────────>│ new user
   │          │            │              │            │ oauth_acct
   │          │<── 302 + JWT + cookie ───┤            │ refresh
   │<─────────┤            │              │            │
   │          │            │              │            │
   │ login    │ authorize ?intent=login   │            │
   ├─────────>│───────────>│──────────────┼───────────>│
   │          │            │ lookup sub   ├───────────>│
   │          │<── 302 + JWT + cookie ───┤            │
   │          │            │              │            │
   │          │ API Bearer ├──────────────┼───────────>│
   │          │<── 200 ────┤              │            │
   │          │ API expired├──────────────┼───────────>│
   │          │<── 401 ────┤              │            │
   │          │ POST /refresh             ├───────────>│ rotate
   │          │ retry API  ├──────────────┼───────────>│
   │          │<── 200 ────┤              │            │
   │          │            │              │            │
   │ logout   │ POST /logout              ├───────────>│ revoke
   ├─────────>│───────────>│              │            │
   │          │<── 204 ────┤              │            │
   │<─────────┤            │              │            │
```

---

## Typical REST endpoints

| Method | Path | Purpose |
|--------|------|---------|
| `GET` | `/auth/oauth/{provider}/authorize` | Start OAuth; `?intent=register\|login\|link` + PKCE `code_challenge` |
| `GET` | `/auth/oauth/{provider}/callback` | IdP redirect target; exchange code, branch register/login |
| `POST` | `/auth/oauth/{provider}/link` | Link provider to authenticated user (after password login) |
| `POST` | `/auth/login` | Password login (used before linking OAuth) |
| `POST` | `/auth/refresh` | Rotate tokens when access JWT expires |
| `POST` | `/auth/logout` | Revoke refresh token / clear cookie |
| `GET` | `/api/*` | Protected resources (`Authorization: Bearer …`) |

Supported `{provider}` values: `google`, `github` (extensible).

---

## Stored data model (simplified)

| Table / record | Key fields |
|----------------|------------|
| **users** | `id`, `email`, `display_name`, `email_verified`, `password_hash` (nullable for OAuth-only) |
| **oauth_accounts** | `provider`, `provider_sub`, `user_id`, `created_at` |
| **oauth_states** | `state`, `code_verifier`, `intent`, `redirect_uri`, `expires_at` |
| **refresh_tokens** | `token_hash`, `user_id`, `expires_at`, `revoked` |

---

## Token & OAuth artifact summary

| Artifact | Lifetime | Where it lives | Used for |
|----------|----------|----------------|----------|
| **Access JWT** | Short (5–15 min) | Frontend memory | Every API request |
| **Refresh token** | Long (days/weeks) | HttpOnly Secure cookie | `/auth/refresh`, `/auth/logout` |
| **OAuth state** | ~10 minutes | Server DB (ephemeral) | CSRF protection; bind callback to start request |
| **PKCE code_verifier** | ~10 minutes | Server DB (with state) | Prove authorization code exchange is from same client |
| **Provider id_token** | Minutes | Server only (transient) | Read `sub`, `email`, `email_verified`; not stored long-term |
| **provider_sub** | Permanent | Server DB (`oauth_accounts`) | Identify returning OAuth users |

---

## Security notes specific to OAuth

- **Authorization Code + PKCE** — required for public SPAs; never expose `client_secret` to the browser.
- **State parameter** — cryptographically random, single-use, stored server-side; reject callbacks with unknown or expired state.
- **Redirect URI allow-list** — callback URL must exactly match registered values at the provider and in API config.
- **Trust IdP email only when `email_verified` is true** — reject registration otherwise.
- **Account linking requires authenticated session** — never auto-merge OAuth to an existing email without proof of ownership (password login or verified email link).
- **Separate intents** — `register` vs `login` prevents silently creating accounts on login-only buttons.
- **Local logout ≠ provider logout** — revoking the refresh token does not sign the user out of Google/GitHub.
