---
title: "Migrating D1 database with zero downtime"
date: 2026-07-08
messageCount: 6
projectName: ""
model: "claude-sonnet-5"
tags:
  - "Claude"
  - "Obsidian"
url: "https://claude.ai/chat/47f2d423-2dc5-465f-be37-eddb7314df9e"
---

## You

`id:260708-1009` ^260708-100937

In cloudflare worker, how to migrate a d1 database and deploy a new version of the backend with no or minimal down time

---

## Claude

`id:260708-1009` ^260708-100959

Cloudflare Workers deploys are already atomic and near-instant — traffic switches to the new version with no restart, so the deploy step itself isn't usually the downtime risk. The real risk is **schema/code mismatch** during rollout, since D1 doesn't support branching or transactional cutover the way some other databases do. The standard fix is the **expand/contract pattern**:

**1. Expand (backward-compatible migration first)**
- Add new columns/tables as nullable or with defaults — never `NOT NULL` without a default in the same step.
- Apply it before deploying new code:
```
npx wrangler d1 migrations apply <DB_NAME> --remote
```
- Old code keeps working unchanged against the expanded schema.

**2. Deploy code that supports both old and new schema**
- New Worker version reads/writes in a way compatible with both states (dual-write if needed).
- Deploy with `wrangler deploy` — this is the atomic, low-downtime part.

**3. Backfill data**
- Do this in small batches (via a script or Worker cron/queue), not one giant migration file — large single-statement rewrites can time out or lock tables.

**4. Deploy code that fully depends on the new schema**
- Once backfill is confirmed complete.

**5. Contract (cleanup migration)**
- Drop old columns only now. SQLite/D1 doesn't support `DROP COLUMN` cleanly in all cases — the common approach is to create a new table with the desired schema, copy the data over, drop the old table, then rename the new one.

**Other practical notes:**
- D1 migrations are atomic per file, so avoid leaving a migration half-applied — split risky changes into separate files.
- D1's capabilities (transactions, limits) evolve quickly, so it's worth checking current docs if you're relying on multi-statement transactional guarantees.
- Test the full migration + deploy sequence locally first with `--local` before running `--remote`.
- Take a backup or Time Travel bookmark before applying remote migrations — D1 has no built-in rollback, so undoing a change means writing a new migration rather than reverting the old one.
- Worth noting: D1 doesn't have built-in zero-downtime schema branching the way some other databases do — so the expand/contract discipline above is effectively how people work around that today.

If you tell me your ORM (raw SQL, Drizzle, etc.) and CI setup, I can sketch the exact migration + deploy sequence for your pipeline.

---

## You

`id:260708-1012` ^260708-101210

What's the workflow if down time is acceptable? How should one park the site

---

## Claude

`id:260708-1012` ^260708-101227

Much simpler. Since you don't need backward compatibility during the window, skip the expand/contract dance entirely — just gate traffic, migrate, deploy, reopen.

**1. Park the site**
Cheapest option: add an env var / KV flag your existing Worker checks first, before any routing logic:
```js
export default {
  async fetch(request, env) {
    if (env.MAINTENANCE_MODE === "true") {
      return new Response(maintenanceHTML, {
        status: 503,
        headers: { "Content-Type": "text/html", "Retry-After": "300" }
      });
    }
    // normal routing...
  }
}
```
Toggle it with `wrangler deploy --var MAINTENANCE_MODE:true`, or better, store the flag in KV so you can flip it instantly without a redeploy:
```
npx wrangler kv key put --binding=CONFIG maintenance_mode "true"
```
- Return **503** with a `Retry-After` header (not 200) so search engines and monitors know it's temporary.
- If you use Cloudflare's DNS/proxy on a zone, Business/Enterprise plans also offer a no-code **Waiting Room** for this, but a Worker is more flexible and works on any plan.
- Whitelist your own IP or a bypass cookie so you can still test against the live domain during the window.

**2. Run the migration with no constraints**
With traffic parked, you can do breaking changes directly — drop columns, rename things, run the SQLite table-recreate pattern for `DROP COLUMN`, etc.:
```
npx wrangler d1 migrations apply <DB_NAME> --remote
```
Take a Time Travel bookmark or backup first regardless — free rollback insurance:
```
npx wrangler d1 time-travel info <DB_NAME>
```

**3. Deploy the new Worker version**
```
npx wrangler deploy
```
No need for dual-schema-compatible code since nothing's live to break.

**4. Smoke test**
Hit key endpoints yourself (via bypass cookie/IP) before reopening — confirm schema and code agree.

**5. Unpark**
Flip the KV flag back or redeploy with the flag off. Propagates globally in seconds.

A minor gotcha: if your migration or backfill takes a while, keep the maintenance page's `Retry-After` and copy honest about duration — and make sure the flag check happens as the very first thing in `fetch`, before any D1 calls, so a broken schema can't crash the maintenance page itself.

---

## You

`id:260708-1015` ^260708-101503

What's the best way to implement the bypass

---

## Claude

`id:260708-1015` ^260708-101517

Cookie-based bypass with a secret is the standard approach — simple, works from anywhere (not tied to your IP), and doesn't require a redeploy to grant/revoke access.

**How it works**

1. Store a bypass secret using Wrangler secrets (never hardcode it):
```
npx wrangler secret put BYPASS_SECRET
```

2. Visiting a special URL sets an `HttpOnly` cookie, then redirects to the real page:
```js
export default {
  async fetch(request, env) {
    const url = new URL(request.url);

    // Bypass login route: /maintenance-bypass?key=xxx
    if (url.pathname === "/maintenance-bypass") {
      const key = url.searchParams.get("key");
      if (key === env.BYPASS_SECRET) {
        const headers = new Headers();
        headers.append("Set-Cookie",
          `bypass=${env.BYPASS_SECRET}; HttpOnly; Secure; SameSite=Strict; Max-Age=3600; Path=/`);
        headers.set("Location", "/");
        return new Response(null, { status: 302, headers });
      }
      return new Response("Forbidden", { status: 403 });
    }

    const cookie = request.headers.get("Cookie") || "";
    const hasBypass = cookie.includes(`bypass=${env.BYPASS_SECRET}`);

    if (env.MAINTENANCE_MODE === "true" && !hasBypass) {
      return new Response(maintenanceHTML, { status: 503, headers: { "Content-Type": "text/html" } });
    }

    // normal routing...
  }
}
```

3. To get in: visit `https://yoursite.com/maintenance-bypass?key=<secret>` once — the cookie persists for the session, so you don't need the query param on every request afterward.

**Why this over alternatives:**
- **IP allowlisting** — breaks the moment anyone on the team is on a different network, VPN, or mobile hotspot. Fine as a *second* layer, not as the only gate.
- **Basic auth header** — works but is clunky to trigger from a browser tab without a login prompt every time; cookie is smoother for repeated checks during a migration.
- **Cloudflare Access (Zero Trust)** — cleanest if you already use it: gate the whole zone or a path behind an identity provider, no custom code needed. Worth it if you have Access set up already; overkill to stand up just for this.

**A few hardening notes:**
- Use `Secure` and `HttpOnly` on the cookie so it's not readable by JS and only sent over HTTPS.
- Don't compare the raw secret value with `===` on a request header/cookie in a way that's timing-attack-sensitive if this matters to you — for an internal maintenance gate it's usually not worth the complexity, but worth knowing.
- Set a reasonable `Max-Age` (an hour or two) so the cookie doesn't linger indefinitely on a shared machine.
- Rotate `BYPASS_SECRET` after the migration window closes, since it was passed in a URL (which can end up in browser history, logs, etc.).