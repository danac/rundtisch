# Registration & Authentication Flow — WebAuthn Hardware Passkey

Web frontend ↔ REST API with **email activation**, **WebAuthn (FIDO2) hardware passkey**, **JWT access tokens**, and **rotating refresh tokens**.

Passwordless model: the user proves identity with a **hardware security key** (USB / NFC / BLE). The browser mediates via the **Web Authentication API** (`navigator.credentials`).

## Legend

| Symbol | Meaning |
|--------|---------|
| `──>` | Request / message |
| `<──` | Response |
| `...` | Internal or DB operation |
| `~~~` | WebAuthn / CTAP2 ceremony (browser ↔ passkey) |
| `[401]` | Unauthorized (expired access JWT) |
| `[403]` | Forbidden (email not verified) |

## Actors

```
  User       Frontend      REST API     Passkey       Email        Database
 (human)    (Web app +     (Backend)   (Hardware      (Mailer)   (Users, creds,
            WebAuthn API)              FIDO2 key)                challenges, tokens)
    |            |             |            |            |              |
```

---

## Phase 1 — Account registration (email only)

No password is stored. Authentication is delegated to a passkey registered after email verification.

```
User       Frontend      REST API     Passkey       Email        Database
  |            |             |            |            |              |
  |-- register |             |            |            |              |
  |  (email)   |             |            |            |              |
  |            |-- POST /auth/register -->|            |              |
  |            |   { email }  |            |            |              |
  |            |             |            |            |              |
  |            |             | ... create user --------------------->|
  |            |             |     email_verified = false             |
  |            |             |     no password hash                   |
  |            |             | ... store activation token ----------->|
  |            |             |            |            |              |
  |            |             |-- send activation email ------------->|
  |            |             |   link + token         |              |
  |            |             |            |            |              |
  |            |<-- [201] Created --------|            |              |
  |            |   (no session tokens)    |            |              |
  |            |             |            |            |              |
  |<-- "Check your email to activate" ---|            |              |
  |            |             |            |            |              |
```

---

## Phase 2 — Email activation

```
User       Frontend      REST API     Passkey       Email        Database
  |            |             |            |            |              |
  |-- open link from inbox --------------------------->|              |
  |            |             |            |            |              |
  |-- /activate?token=... -->|            |            |              |
  |            |             |            |            |              |
  |            |-- POST /auth/activate -->|            |              |
  |            |   { token }  |            |            |              |
  |            |             |            |            |              |
  |            |             | ... validate token, set -------------->|
  |            |             |     email_verified = true              |
  |            |             |     invalidate activation token        |
  |            |             | ... issue short-lived session -------->|
  |            |             |     (for passkey registration only)  |
  |            |             |            |            |              |
  |            |<-- [200] Activated -------|            |              |
  |            |   temp access JWT        |            |              |
  |            |             |            |            |              |
  |<-- "Register your passkey" ------------|            |              |
  |            |             |            |            |              |
```

---

## Phase 3 — Passkey registration ceremony (WebAuthn)

The backend issues a **challenge**; the browser calls `navigator.credentials.create()`; the hardware key performs **CTAP2 `makeCredential`**.

```
User       Frontend      REST API     Passkey       Email        Database
  |            |             |            |            |              |
  |-- "Add passkey" -------->|            |            |              |
  |            |             |            |            |              |
  |            |-- POST /auth/webauthn/register/options -->|          |
  |            |   Bearer <temp accessJWT>  |            |              |
  |            |             |            |            |              |
  |            |             | ... generate challenge, store -------->|
  |            |             |     (TTL ~60s, bound to user session)  |
  |            |             |            |            |              |
  |            |<-- [200] PublicKeyCredentialCreationOptions --------|
  |            |   { challenge, rp: { id, name },                     |
  |            |     user: { id, name, displayName },                  |
  |            |     pubKeyCredParams: [ES256, RS256],                 |
  |            |     authenticatorSelection: {                         |
  |            |       authenticatorAttachment: "cross-platform",    |
  |            |       residentKey: "required",   ← discoverable cred |
  |            |       requireResidentKey: true,                         |
  |            |       userVerification: "required" },                 |
  |            |     timeout: 60000 }                                  |
  |            |             |            |            |              |
  |            | navigator.credentials.create({ publicKey })         |
  |            | ~~~~~~~~~~~~ WebAuthn ceremony ~~~~~~~~~~~~~~~~~~~~~~|
  |            |             |            |            |              |
  |            |-- CTAP2 makeCredential ---------------->|              |
  |            |   rpId, user, algorithms, UV required   |              |
  |            |             |            |            |              |
  |-- touch key / enter PIN on device ----------------->|              |
  |            |             |            |   generate key pair        |
  |            |             |            |   sign attestation         |
  |            |             |            |            |              |
  |            |<-- attestationObject + clientDataJSON -|              |
  |            | ~~~~~~~~~~~~ end ceremony ~~~~~~~~~~~~~~~~~~~~~~~~~~~~|
  |            |             |            |            |              |
  |            |-- POST /auth/webauthn/register/verify -->|            |
  |            |   { id, rawId, type: "public-key",                    |
  |            |     response: { attestationObject, clientDataJSON },  |
  |            |     transports: ["usb","nfc","ble"] }                 |
  |            |             |            |            |              |
  |            |             | ... verify:                            |
  |            |             |     challenge matches stored value     |
  |            |             |     origin + rpId match                  |
  |            |             |     attestation signature valid          |
  |            |             | ... store credential ------------------->|
  |            |             |     credentialId, publicKey, signCount,|
  |            |             |     aaguid, transports, userId,          |
  |            |             |     userHandle (= opaque user.id)        |
  |            |             |                                          |
  |            |             |   Passkey also stores resident copy:     |
  |            |             |     rpId + userHandle + private key      |
  |            |             |     (enables usernameless login later)   |
  |            |             | ... consume challenge (single-use) ---->|
  |            |             |            |            |              |
  |            |<-- [201] Passkey registered --------|            |              |
  |            |   { credentialId }     |            |            |              |
  |            |             |            |            |              |
  |<-- "Passkey ready — you can sign in" --|            |            |              |
  |            |             |            |            |              |
```

---

## Phase 4 — Login with hardware passkey (WebAuthn authentication)

Two login paths share the same verify step. **Usernameless** (below) is the primary flow when resident credentials were registered.

### 4A — Usernameless login (resident / discoverable credentials)

No email is entered. Because the passkey stored a **resident credential** (rpId + userHandle) at registration, the authenticator can identify the account on its own.

```
User       Frontend      REST API     Passkey       Email        Database
  |            |             |            |            |              |
  |-- "Sign in with passkey" ->|            |            |              |
  |  (no email) |             |            |            |              |
  |            |             |            |            |              |
  |            |-- POST /auth/webauthn/login/options -->|            |
  |            |   { }        |  empty body — no email, no user hint  |
  |            |             |            |            |              |
  |            |             | ... generate challenge, store -------->|
  |            |             |     (no user lookup yet)               |
  |            |             |            |            |              |
  |            |<-- [200] PublicKeyCredentialRequestOptions ---------|
  |            |   { challenge, rpId,                                      |
  |            |     allowCredentials: undefined,   ← discoverable mode |
  |            |     userVerification: "required", timeout: 60000 }       |
  |            |             |            |            |              |
  |            | navigator.credentials.get({ publicKey })                |
  |            | ~~~~~~~~~~~~ WebAuthn ceremony ~~~~~~~~~~~~~~~~~~~~~~|
  |            |             |            |            |              |
  |            |-- CTAP2 getAssertion (discoverable) --->|              |
  |            |   challenge, rpId, NO allowList, UV     |              |
  |            |             |            |            |              |
  |            |             |            |  scan resident creds      |
  |            |             |            |  for matching rpId         |
  |            |             |            |            |              |
  |-- touch key / pick account (if multiple) --------->|              |
  |            |             |            |   lookup userHandle + key  |
  |            |             |            |   sign challenge           |
  |            |             |            |            |              |
  |            |<-- authenticatorData + signature -----|              |
  |            |    + clientDataJSON + userHandle       |              |
  |            | ~~~~~~~~~~~~ end ceremony ~~~~~~~~~~~~~~~~~~~~~~~~~~~~|
  |            |             |            |            |              |
  |            |-- POST /auth/webauthn/login/verify --->|              |
  |            |   { id, rawId, type: "public-key",                    |
  |            |     response: { authenticatorData, clientDataJSON,     |
  |            |                signature, userHandle } }               |
  |            |             |            |            |              |
  |            |             | ... lookup credential by credentialId -->|
  |            |             |     OR user by userHandle (opaque id)    |
  |            |             | ... verify:                            |
  |            |             |     challenge matches stored value     |
  |            |             |     origin + rpId match                  |
  |            |             |     signature over authData+clientData   |
  |            |             |     signCount incremented (clone check)  |
  |            |             |     email_verified = true                |
  |            |             | ... create refresh token -------------->|
  |            |             | ... consume challenge ------------------>|
  |            |             |            |            |              |
  |            |<-- [200] access JWT + Set-Cookie: refresh -------------|
  |            |   { accessToken, expiresIn, user }                    |
  |            |             |            |            |              |
  |<-- enter app (authenticated) ----------|            |            |              |
  |            |             |            |            |              |
```

**Why no email is needed:** at registration the server assigned an opaque `user.id` (userHandle). The passkey stored it as a resident credential. On login the assertion's `userHandle` identifies the account — the server never needed the email upfront.

### 4B — Login with email (non-resident fallback)

Use when resident credentials are unavailable or the user prefers entering their email first.

```
User       Frontend      REST API     Passkey       Email        Database
  |            |             |            |            |              |
  |-- email + "Sign in" ---->|            |            |              |
  |            |             |            |            |              |
  |            |-- POST /auth/webauthn/login/options -->|            |
  |            |   { email }  |            |            |              |
  |            |             | ... lookup user + credentials -------->|
  |            |             | ... generate challenge, store -------->|
  |            |             |            |            |              |
  |            |<-- [200] PublicKeyCredentialRequestOptions ---------|
  |            |   { challenge, rpId,                                      |
  |            |     allowCredentials: [{ id, type, transports }],      |
  |            |     userVerification: "required" }                        |
  |            |             |            |            |              |
  |            |  (same WebAuthn ceremony + verify as 4A from here)       |
  |            |             |            |            |              |
```

### Login blocked if email not verified

Checked at **verify** time (usernameless) or **options** time (email path):

```
  |            |-- login/verify (or options with email) -->|
  |            |             | ... user found, verified=false
  |            |<-- [403] Email not verified -------------|
  |<-- resend activation / check email ----|
```

---

## Phase 5 — Authenticated API usage

```
User       Frontend      REST API     Passkey       Email        Database
  |            |             |            |            |              |
  |            |-- GET /api/resource --->|            |              |
  |            |   Authorization: Bearer <accessJWT>  |              |
  |            |             |            |            |              |
  |            |             | ... verify JWT signature + exp         |
  |            |             |            |            |              |
  |            |<-- [200] Resource JSON --|            |              |
  |            |             |            |            |              |
        (passkey is NOT involved in routine API calls — only JWT is sent)
```

---

## Phase 6 — Expired access token → refresh

```
User       Frontend      REST API     Passkey       Email        Database
  |            |             |            |            |              |
  |            |         *** Access JWT expired ***    |              |
  |            |             |            |            |              |
  |            |-- GET /api/resource --->|            |              |
  |            |   Bearer <expired accessJWT>         |              |
  |            |             |            |            |              |
  |            |<-- [401] Unauthorized --|            |              |
  |            |   { "error": "token_expired" }       |              |
  |            |             |            |            |              |
  |            |-- POST /auth/refresh --->|            |              |
  |            |   Cookie: refreshToken (HttpOnly)    |              |
  |            |             |            |            |              |
  |            |             | ... validate + rotate refresh token --->|
  |            |             |            |            |              |
  |            |<-- [200] new access JWT + Set-Cookie: refresh ------|
  |            |             |            |            |              |
  |            |-- retry original request ->|            |              |
  |            |   Bearer <new accessJWT>   |            |              |
  |            |             |            |            |              |
  |            |<-- [200] Resource JSON --|            |              |
  |            |             |            |            |              |
```

If refresh also fails → clear session, redirect to passkey login (no password fallback).

---

## Phase 7 — Logout

```
User       Frontend      REST API     Passkey       Email        Database
  |            |             |            |            |              |
  |-- logout ->|             |            |            |              |
  |            |             |            |            |              |
  |            |-- POST /auth/logout ---->|            |              |
  |            |   Cookie: refreshToken   |            |              |
  |            |             |            |            |              |
  |            |             | ... revoke refresh token -------------->|
  |            |             |            |            |              |
  |            |<-- [204] Clear cookie ----|            |              |
  |            |             |            |            |              |
  |            | ... drop access JWT from memory       |              |
  |<-- redirect to passkey login --------|            |              |
  |            |             |            |            |              |
```

---

## Full end-to-end overview

```
┌──────┐ ┌──────────┐ ┌──────────┐ ┌─────────┐ ┌───────┐ ┌──────────┐
│ User │ │ Frontend │ │ REST API │ │ Passkey │ │ Email │ │ Database │
└──┬───┘ └────┬─────┘ └────┬─────┘ └────┬────┘ └───┬───┘ └────┬─────┘
   │          │            │            │          │          │
   │ register │ POST /register          │          │          ├─> user
   ├─────────>│───────────>│ send mail  ├─────────>│          │
   │          │<── 201 ────┤            │          │          │
   │          │            │            │          │          │
   │ activate │ POST /activate           │          │          ├─> verified
   ├─────────>│───────────>│            │          │          │
   │          │<── 200 + temp JWT        │          │          │
   │          │            │            │          │          │
   │ add key  │ POST /webauthn/register/options     │          ├─> challenge
   ├─────────>│───────────>│            │          │          │
   │          │<── creation options ─────┤ residentKey: required
   │          │ credentials.create()     │          │          │
   │ touch ───┼────────────┼───────────>│ CTAP2    │          │
   │          │<── attestation ──────────┤ makeCred │          │
   │          │ POST /register/verify    │          │          ├─> store cred
   │          │───────────>│            │          │          │   + userHandle
   │          │<── 201 ────┤            │          │          │   on passkey
   │          │            │            │          │          │
   │ login    │ POST /login/options { }  │          │          ├─> challenge
   │ (no email)──────────>│───────────>│            │          │   (no user yet)
   ├─────────>│<── options, no allowCredentials ──┤          │
   │          │ credentials.get()        │          │          │
   │ touch ───┼────────────┼───────────>│ discover │          │
   │          │<── assertion + userHandle ─┤ resident │          │
   │          │ POST /login/verify         │          │          ├─> lookup by
   │          │───────────>│            │          │          │   userHandle
   │          │<── JWT + cookie ───────┤            │          │
   │          │            │            │          │          │
   │          │ API Bearer ├────────────┼──────────┼──────────>│
   │          │<── 200 ────┤            │          │          │
   │          │ API expired├────────────┼──────────┼──────────>│
   │          │<── 401 ────┤            │          │          │
   │          │ POST /refresh            │          │          ├─> rotate
   │          │───────────>│            │          │          │
   │          │ retry API  ├────────────┼──────────┼──────────>│
   │          │<── 200 ────┤            │          │          │
   │          │            │            │          │          │
   │ logout   │ POST /logout             │          │          ├─> revoke
   ├─────────>│───────────>│            │          │          │
   │          │<── 204 ────┤            │          │          │
   │<─────────┤            │            │          │          │
```

---

## Typical REST endpoints

| Method | Path | Purpose |
|--------|------|---------|
| `POST` | `/auth/register` | Create account (email only) |
| `POST` | `/auth/activate` | Confirm email; issue temp JWT for passkey setup |
| `POST` | `/auth/resend-activation` | Resend activation email |
| `POST` | `/auth/webauthn/register/options` | Start passkey registration (authenticated) |
| `POST` | `/auth/webauthn/register/verify` | Finish registration; store credential |
| `POST` | `/auth/webauthn/login/options` | Start passkey login; **empty body** for usernameless, or `{ email }` for allow-list |
| `POST` | `/auth/webauthn/login/verify` | Verify assertion; resolve user via **userHandle** or credentialId; issue JWT |
| `POST` | `/auth/refresh` | Rotate tokens when access JWT expires |
| `POST` | `/auth/logout` | Revoke refresh token |
| `GET` | `/api/*` | Protected resources (`Authorization: Bearer …`) |

---

## Stored data model (simplified)

| Table / record | Key fields |
|----------------|------------|
| **webauthn_credentials** | `credential_id`, `user_id`, `public_key`, `sign_count`, `aaguid`, `transports`, `created_at` |
| **users** | `id` (= opaque **userHandle** bytes, base64url), `email`, `email_verified`, `display_name` |
| **webauthn_challenges** | `challenge`, `user_id`, `type` (register \| login), `expires_at` |
| **refresh_tokens** | `token_hash`, `user_id`, `expires_at`, `revoked` |

---

## Token & credential summary

| Artifact | Lifetime | Where it lives | Used for |
|----------|----------|----------------|----------|
| **Access JWT** | Short (5–15 min) | Frontend memory | Every API request |
| **Refresh token** | Long (days/weeks) | HttpOnly Secure cookie | `/auth/refresh`, `/auth/logout` |
| **Passkey private key** | Permanent | Hardware authenticator only | WebAuthn login ceremonies |
| **Passkey public key** | Permanent | Server DB | Verify login assertions |
| **Resident credential** | Permanent | Hardware authenticator | Stores rpId + userHandle — enables usernameless login |
| **userHandle** | Permanent | Passkey + server `users.id` | Identifies account without email at login time |
| **WebAuthn challenge** | ~60 seconds | Server DB (ephemeral) | Bind each ceremony to one request |

---

## Security notes specific to hardware passkeys

- **Resident key required** (`residentKey: "required"`) stores credential on the authenticator with the opaque `user.id` as **userHandle** — prerequisite for usernameless login.
- **Usernameless login**: omit `allowCredentials` in request options; passkey discovers resident creds for `rpId`; server identifies user from `userHandle` in the assertion.
- **User verification** (`userVerification: "required"`) enforces PIN/biometric on the authenticator.
- **Cross-platform** (`authenticatorAttachment: "cross-platform"`) targets removable hardware keys (not platform passkeys).
- **Sign count** must monotonically increase on each login — a decrease indicates possible cloned credential.
- **RP ID** must exactly match the site origin's effective domain; prevents phishing.
- **Challenges** are single-use, server-generated, and cryptographically random.
- Private keys **never leave** the hardware authenticator.
