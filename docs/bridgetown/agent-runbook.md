# Bridgetown Agent Runbook (Local-First)

Use this runbook when executing the migration in a local/dev environment.

## 1) Preconditions

- Ruby installed (recommended via `rbenv` or `asdf`).
- Bundler installed.
- Node.js installed (if selected Bridgetown plugins require JS build steps).

## 2) Bootstrap Bridgetown in `docs/site`

From repository root:

```bash
mkdir -p docs/site
cd docs/site
bundle init
bundle add bridgetown
bundle exec bridgetown new . --force
```

Expected artifacts include:

- `docs/site/bridgetown.config.yml`
- `docs/site/src/index.md`
- `docs/site/config/initializers.rb`

## 3) Add baseline content structure

Inside `docs/site/src/`, create sections:

```text
src/
  guides/
  architecture/
  api/
```

Recommended mapping:

- `docs/reference/extend-solver.md` -> `src/guides/extend-solver.md`
- `docs/reference/extend-domain.md` -> `src/guides/extend-domain.md`
- `docs/reference/lifecycle-pause-resume-contract.md` -> `src/architecture/lifecycle-pause-resume-contract.md`
- `docs/reference/typed-contract-audit.md` -> `src/architecture/typed-contract-audit.md`

## 4) Local verification

From `docs/site`:

```bash
bundle exec bridgetown doctor
bundle exec bridgetown build
bundle exec bridgetown serve
```

Then open the local Bridgetown URL and verify migrated pages are linked from homepage/navigation.

## 5) Deployment handoff

Once local build is stable, implement CI deployment strategy from this repository:

- Build site from `docs/site`.
- Publish generated output to selected Pages target.
- Document canonical URL and redirect policy.

## 6) Commit hygiene for agents

- Separate commits by concern (bootstrap vs content vs deploy).
- Keep generated file updates reviewable.
- Update `docs/README.md` whenever paths or process docs move.
