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
    |
    `-> solverforge-core     - Score types, domain traits, descriptors
         |
         `-> solverforge-macros - #[planning_solution], #[planning_entity], #[problem_fact]
```

Publishing order: `core -> macros -> scoring -> config -> solver -> facade`

Use `README.md`, crate manifests, and the crate wireframes to confirm current details before changing public APIs.

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
- `crates/solverforge-cli/WIREFRAME.md`

When public surface changes, update the matching wireframe in the same change:

1. Add new public types, traits, functions, and modules.
2. Update renamed items and changed signatures.
3. Remove deleted public items completely.
4. Keep file maps in sync with added or moved files.
5. Document public surface and usage only, not implementation detail.

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
- Keep examples and quickstarts on the public fluent API surface. If an example requires internal wiring, improve the public API instead.

## Change Policy

- Do not keep compatibility shims, fallback implementations, or transitional APIs.
- Delete dead code rather than hiding it behind `_unused` names or "removed" comments.
- When a design changes, migrate call sites to the new design instead of preserving the old path.
- Do not mention external solver frameworks in code comments, examples, or commits. SolverForge is an independent implementation.

## Build And Validation

- `make build`
- `make test`
- `make pre-release`

Minimum supported Rust version: `1.80+`
