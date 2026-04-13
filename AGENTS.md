# AGENTS.md

Guidance for Codex and other coding agents working in this repository.

## Overview

SolverForge is a high-performance heuristic constraint programming framework in Rust for planning and scheduling problems.

Workspace structure:

```text
solverforge (facade + re-exports)
    |
    |-- solverforge-solver   - Phases, moves, selectors, termination, SolverManager
    |-- solverforge-scoring  - ConstraintStream API, SERIO incremental scoring
    |-- solverforge-config   - TOML/YAML configuration
    |-- solverforge-console  - Tracing-driven console output and progress formatting
    |-- solverforge-cvrp     - CVRP domain helpers and distance utilities
    |
    `-> solverforge-core     - Score types, domain traits, descriptors
         |
         `-> solverforge-macros - #[planning_solution], #[planning_entity], #[problem_fact]
```

Publishing order: `core -> macros -> scoring -> config -> solver -> cvrp -> console -> facade`

Standalone ecosystem repos such as `solverforge-cli`, `solverforge-ui`, and `solverforge-maps` are not part of this workspace. Treat references to them as external integrations, not in-repo crates.

Current workspace release version: `0.8.5`.

Use `README.md`, crate manifests, and the crate wireframes to confirm current details before changing public APIs.

## Documentation Layer

Treat the repository documentation as a coordinated surface, not as isolated files.

- `README.md` is the user-facing entry point. Update it when public workflows, naming, or extension guidance change.
- `crates/*/WIREFRAME.md` files are the canonical public API maps. Update them for any public surface change.
- `docs/*.md` files capture focused extension and architecture guidance. When a refactor changes naming or explains an intentional boundary, update the relevant doc or add a dedicated audit note.
- `AGENTS.md` records repository-specific rules for future coding agents. Update it when the engineering workflow or documentation policy changes.

## Wireframes Are Canonical

Each crate has a `WIREFRAME.md` file that documents its public API surface, module map, and usage patterns. Treat the relevant wireframe as the canonical reference before changing a crate.

Current wireframes:

- `crates/solverforge/WIREFRAME.md`
- `crates/solverforge-core/WIREFRAME.md`
- `crates/solverforge-macros/WIREFRAME.md`
- `crates/solverforge-scoring/WIREFRAME.md`
- `crates/solverforge-solver/WIREFRAME.md`
- `crates/solverforge-config/WIREFRAME.md`
- `crates/solverforge-cvrp/WIREFRAME.md`

When public surface changes, update the matching wireframe in the same change:

1. Add new public types, traits, functions, and modules.
2. Update renamed items and changed signatures.
3. Remove deleted public items completely.
4. Keep file maps in sync with added or moved files.
5. Document public surface and usage only, not implementation detail.

When a public API change also affects how users discover or reason about the feature, update the corresponding top-level docs in the same change. Renames and architectural cleanups should leave behind a clear trail in `README.md`, relevant `docs/*.md` files, and the affected wireframes.

Version bumps are documentation changes too. When the published version changes, update the manifest version plus the coordinated public docs surface in the same change:

- `Cargo.toml` workspace version and publish-time inter-crate dependency versions
- `README.md`
- `AGENTS.md`
- affected `crates/*/WIREFRAME.md` files

## Architecture Constraints

### Zero-Erasure

All hot-path code must stay monomorphized.

- Do not introduce `Box<dyn Trait>` in hot paths.
- Do not introduce `Arc<T>` or `Rc<T>`.
- Do not introduce `dyn Trait` unless it is an existing intentional boundary.
- Avoid `.clone()` in move evaluation and scoring paths.
- Preserve concrete types through the full solver pipeline.

Intentional type-erasure boundaries that should not be "fixed":

- `DynDistanceMeter` in `nearby.rs`
- `DefaultPillarSelector` extractor closures in `pillar.rs`

### Move Ownership

Moves are never cloned in the solver path. The forager stores arena indices, and the selected move is taken from the arena by ownership.

Clone is acceptable only for:

1. Solution snapshots sent through channels.
2. Partitioned search creating partition copies.

### Threading

Use `rayon` for CPU-bound work and `tokio` for async coordination.

- Prefer `rayon::spawn`, `rayon::scope`, and parallel iterators.
- Use `tokio::sync::mpsc` for ownership transfer across async boundaries.
- Do not add `std::thread::spawn`.
- Do not add `std::thread::sleep`.
- Prefer ownership transfer over callback APIs that only expose `&Self`.

### PhantomData

For phantom type parameters, prefer `PhantomData<fn() -> T>` rather than `PhantomData<T>` so the phantom does not inherit unnecessary `Send`, `Sync`, or `Clone` bounds.

### Manual Clone

Avoid `#[derive(Clone)]` on generic types when it would introduce unnecessary bounds. Write a manual `Clone` impl that only constrains fields that actually need cloning.

## Code Organization

- Keep `mod.rs` files limited to module declarations and re-exports.
- Move implementation code into dedicated files.
- Prefer doctests for public APIs when practical, and make them compile meaningfully.
- Keep examples and scaffold-facing guidance on the public fluent API surface. If an example requires internal wiring, improve the public API instead.

## Change Policy

- Do not keep compatibility shims, fallback implementations, or transitional APIs.
- Delete dead code rather than hiding it behind `_unused` names or "removed" comments.
- When a design changes, migrate call sites to the new design instead of preserving the old path.
- Do not mention external solver frameworks in code comments, examples, or commits. SolverForge is an independent implementation.
- Version bumps require explicit user confirmation.
- `CHANGELOG.md` stays under `commit-and-tag-version`; do not hand-edit routine release notes unless the user explicitly asks for it.

## Build And Validation

- `make build`
- `make test`
- `make pre-release`

Minimum supported Rust version: `1.92+`
