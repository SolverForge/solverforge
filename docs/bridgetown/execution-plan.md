# Bridgetown Migration Execution Plan

This plan defines how to migrate SolverForge documentation to Bridgetown while making this repository the canonical website source.

## Goals

- Replace ad-hoc Markdown docs navigation with a Bridgetown site.
- Keep all source content in this repository.
- Enable deterministic local execution so agents can provide code-level references.
- Preserve existing technical docs while improving discoverability.

## Proposed target layout

```text
docs/
  README.md
  AGENTS.md
  bridgetown/
    execution-plan.md
    agent-runbook.md
  reference/
    *.md
  site/                  # Bridgetown app root (to be created)
    bridgetown.config.yml
    src/
      index.md
      guides/
      architecture/
      api/
```

## Phased rollout

### Phase 1 — Bootstrap site

- Create Bridgetown app in `docs/site`.
- Add minimal navigation and homepage.
- Configure local serve/build scripts.

### Phase 2 — Content migration

- Migrate docs from `docs/reference/` into `docs/site/src/` information architecture.
- Keep redirects/stubs for moved paths as needed.
- Normalize heading and front matter conventions.

### Phase 3 — Deployment

- Decide canonical URL strategy:
  - keep `solverforge.github.io` host repository as thin deploy target, or
  - switch canonical URL to custom domain and deploy directly from this repo.
- Add CI workflow to build and publish Bridgetown output.

### Phase 4 — Cleanup

- Remove temporary migration stubs.
- Keep `docs/reference/` only if needed as archival source; otherwise fully fold into `docs/site/src/`.

## Definition of done

- Bridgetown site builds locally from documented commands.
- Existing core docs are migrated and accessible via site navigation.
- Deployment path is documented and reproducible.
- Agent instructions in `docs/AGENTS.md` and `docs/bridgetown/agent-runbook.md` remain current.
