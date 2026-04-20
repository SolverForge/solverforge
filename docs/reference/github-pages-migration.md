# GitHub Pages Migration: `solverforge.github.io` -> `solverforge/docs`

This note documents whether we can retire the standalone `solverforge.github.io` repository and publish the website directly from the `docs/` directory in this repository.

## Short answer

Yes, we can move the **content source of truth** into this repository's `docs/` folder.

However, if we must keep the site URL as **`https://solverforge.github.io/`**, we should expect to keep either:

1. the `solverforge.github.io` repository itself, or
2. an equivalent GitHub Pages target that still owns that hostname.

In practice, for an organization/user Pages root URL (`<org>.github.io`), the special Pages target remains the safest anchor.

## If you control a custom domain

If you can point your own domain wherever you want (DNS + TLS), then yes: you can usually retire the separate `solverforge.github.io` repository.

In that model:

- Publish Pages from this `solverforge` repository (for example via GitHub Pages Actions).
- Set your custom domain (for example `docs.solverforge.dev`) to that Pages site.
- Keep redirects in place from old URLs as needed.

Important distinction:

- `https://solverforge.github.io/` is GitHub-owned hostname semantics.
- Your custom domain is fully under your control.

So if your canonical URL becomes your custom domain, the special `solverforge.github.io` host repo is not strictly required.

## Recommended migration model

Use this repository (`solverforge`) as the single authoring source, and publish to the Pages target via CI.

### Phase 1: Make `docs/` authoritative

- Keep all website content in `solverforge/docs/`.
- If the site needs generation (e.g. mdBook, Docusaurus, MkDocs), build from `docs/` in CI.
- If the site is static HTML/CSS/JS already, deploy `docs/` directly.

### Phase 2: Automated publish bridge

- Add a workflow in this repository that triggers on changes under `docs/**`.
- The workflow publishes the rendered site artifact to the Pages host repository branch.
- Keep deploy credentials in org secrets and lock workflow permissions to least privilege.

### Phase 3: Decommission manually-edited content in `solverforge.github.io`

- Remove hand-maintained site files there.
- Keep only the minimal Pages/deploy plumbing needed to serve `https://solverforge.github.io/`.
- Optionally archive that repository if deployment ownership is moved elsewhere without losing the hostname target.

## Alternatives

### A) Full delete of `solverforge.github.io` repo

This is only safe if we are also willing to change public URL strategy (for example, project Pages URL or custom domain) and update inbound links.

### B) Keep URL and keep tiny host repo

This is the least risky option:

- `solverforge` holds all docs and site source.
- `solverforge.github.io` becomes a thin deploy target only.
- No manual edits needed in the host repo.

## Decision checklist

Before proceeding, confirm:

- Must canonical URL stay `https://solverforge.github.io/`?
- Is a custom domain planned now or later, and who owns DNS/TLS renewals?
- Is generated output committed, or build-only in CI?
- Who owns deploy credentials and branch protections?
- Do we need redirects from old paths?

## Suggested next step

If canonical URL must stay exactly `https://solverforge.github.io/`, proceed with **Option B** (thin host repo + docs source in this repository).

If URL can change (or custom domain is ready), we can evaluate retiring the special repository entirely.
