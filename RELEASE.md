# Release Guide

Release reference for the `release/0.6.0` line and later patch releases.

## Stability Matrix

| Crate | Status | Release Expectation | Notes |
|-------|--------|---------------------|-------|
| `solverforge-core` | Stable | Publish on every coordinated release | Foundational score/domain traits; lowest-level dependency |
| `solverforge-macros` | Stable | Publish on every coordinated release | Proc-macro surface used by the facade and templates |
| `solverforge-scoring` | Stable | Publish on every coordinated release | Constraint-stream and incremental scoring engine |
| `solverforge-config` | Stable | Publish on every coordinated release | Runtime configuration consumed by the solver layer |
| `solverforge-solver` | Stable | Publish on every coordinated release | Main solve engine and manager APIs |
| `solverforge` | Stable | Publish on every coordinated release | Public facade crate and primary library entry point |
| `solverforge-cli` | Beta | Publish when scaffold or operator workflow changes | User-facing binary; validate generated projects before release |
| `solverforge-cvrp` | Beta | Publish whenever the facade version changes and CVRP helpers changed | Required by the facade's versioned dependency set |
| `solverforge-console` | Beta | Publish whenever the facade version changes and console support changed | Optional facade dependency, but versioned independently |
| `solverforge-test` | Internal | Do not publish | Shared test fixtures only |

Status definitions:

- `Stable`: public API expected to remain coherent across the release line; regressions block release.
- `Beta`: supported and versioned, but still more likely to receive usability and coverage fixes between patch releases.
- `Internal`: workspace-only support crate; not part of the published product surface.

## Publish Order

When versions change across the workspace, publish crates in dependency order:

1. `solverforge-core`
2. `solverforge-macros`
3. `solverforge-scoring`
4. `solverforge-config`
5. `solverforge-solver`
6. `solverforge-cvrp`
7. `solverforge-console`
8. `solverforge`
9. `solverforge-cli`

`solverforge-test` stays unpublished.

## Release Checklist

1. Confirm the release branch is correct.
   Use `release/0.6.0` as the base for this release line.
2. Confirm all release-blocking PRs are merged.
   Verify open issues/PRs targeted at the release branch are either merged or explicitly deferred.
3. Sync version and changelog state.
   Update `CHANGELOG.md` and ensure workspace crate versions are coherent.
4. Validate canonical docs.
   Check `README.md`, crate `WIREFRAME.md` files, and this document for stale public-surface details.
5. Run formatting and lint gates.
   `cargo fmt --all -- --check`
   `cargo clippy --workspace --all-targets -- -D warnings`
6. Run test gates.
   `cargo test --workspace`
   `make pre-release`
7. Validate scaffolded user flows.
   Run `cargo test -p solverforge-cli`
   Run ignored scaffold cargo-check tests if template-affecting changes landed.
8. Verify publishability.
   `make publish-crates-dry`
9. Publish crates in dependency order.
   Use `make publish-crates` or publish manually in the order listed above.
10. Post-publish verification.
    Confirm crates.io versions, docs.rs builds, and install smoke tests:
    `cargo install solverforge-cli --version <version>`
    `cargo add solverforge@<version>`

## Release Notes Inputs

Capture these before tagging:

- Merged PR list against `release/0.6.0`
- Breaking public-surface changes, if any
- New templates, commands, macros, or helper APIs
- New validation coverage or release-risk reductions
