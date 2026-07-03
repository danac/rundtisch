---
name: cloudflare-preview-pr
description: Add the Cloudflare Workers preview alias URL to pull request descriptions when creating or updating PRs. Use after opening a PR, when updating PR bodies, or when the user asks for preview deployment links.
---

# Cloudflare preview URL in PR descriptions

When creating or updating a pull request for this repository, include a **Preview deployment** section in the PR body with the Cloudflare Workers preview alias URL.

## When to use

- Creating a new PR via `ManagePullRequest`
- Updating an existing PR body
- The user asks for the preview URL or wants it documented on the PR

## Preview URL format

This repo deploys PR previews via GitHub Actions (`.github/workflows/deploy.yml`) using:

```bash
wrangler versions upload --preview-alias pr-<PR_NUMBER>
```

The stable preview alias URL is:

```
https://pr-<PR_NUMBER>-<WORKER_NAME>.<ACCOUNT_SUBDOMAIN>.workers.dev
```

### Project constants

Read these from the repo when possible:

| Value | Source | Example |
|-------|--------|---------|
| Worker name | `wrangler.jsonc` → `name` | `anikaelsa` |
| Preview alias prefix | `.github/workflows/deploy.yml` → `--preview-alias` | `pr-<PR_NUMBER>` |
| Account subdomain | workers.dev subdomain for this account | check deploy logs |

**Example for PR #42:** `https://pr-42-anikaelsa.<account>.workers.dev`

### Confirming the URL

If a deploy has already run for the PR, prefer the URL from workflow logs:

```bash
gh run list --branch <branch-name> --workflow Deploy --limit 1
gh run view <run-id> --log | rg "Version Preview Alias URL"
```

Use the logged URL if it differs from the constructed one.

## PR body section to add

Insert this section in the PR description (after Summary, before other sections). Preserve existing PR content — merge, do not replace unrelated sections.

```markdown
## Preview deployment

**Preview alias URL:** https://pr-<PR_NUMBER>-<WORKER_NAME>.<ACCOUNT_SUBDOMAIN>.workers.dev

- SPA: https://pr-<PR_NUMBER>-<WORKER_NAME>.<ACCOUNT_SUBDOMAIN>.workers.dev
- Health check: https://pr-<PR_NUMBER>-<WORKER_NAME>.<ACCOUNT_SUBDOMAIN>.workers.dev/api/health

This alias stays the same for this PR; new commits update the deployment behind it.
```

Replace `<PR_NUMBER>`, `<WORKER_NAME>`, and `<ACCOUNT_SUBDOMAIN>` with your project values.

## Notes

- The preview deploy runs only on pull requests targeting `master` (see deploy workflow).
- If the deploy has not run yet, still include the constructed URL — it will work after the first successful deploy.
- Do not post a separate PR comment unless the user asks; include the link in the PR description.
- If `ManagePullRequest` update fails, provide the markdown block for the user to paste manually.
