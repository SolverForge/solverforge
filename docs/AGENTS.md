# AGENTS.md (docs scope)

This file applies to everything under `docs/`.

## Purpose

`docs/` now serves two roles:

1. **Website source-of-truth** for the upcoming Bridgetown site migration.
2. **Reference documentation** for architecture and extension topics.

## Required structure

- Keep planning and migration docs in `docs/bridgetown/`.
- Keep durable product/reference docs in `docs/reference/`.
- Keep a short entrypoint in `docs/README.md` so humans and agents can find the right document quickly.

## Bridgetown migration guardrails

- Prefer small, reviewable commits (setup, content migration, CI integration).
- Do not handwave commands; provide exact local commands and expected artifacts.
- Keep migration docs implementation-oriented (what to run, where files go, how to verify).
- If introducing generated Bridgetown files, keep custom logic minimal and documented.

## Validation expectations

When editing docs for the Bridgetown migration, run at least one relevant local check and record it in your final report. Prefer:

- `cargo fmt --all -- --check` (workspace baseline)
- Bridgetown checks (when Ruby/Bundler is available locally):
  - `bundle exec bridgetown doctor`
  - `bundle exec bridgetown build`

If Bridgetown tooling is unavailable in this environment, document that clearly and keep instructions reproducible.
