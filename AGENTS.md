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
    |-- solverforge-macros   - planning_model!, #[planning_solution], #[planning_entity], #[problem_fact]
    |
    `-> solverforge-core     - Score types, domain traits, descriptors
```

Publishing order: `core -> macros -> scoring -> config -> solver -> cvrp -> console -> facade`

Standalone ecosystem repos such as `solverforge-cli`, `solverforge-ui`, and `solverforge-maps` are not part of this workspace. Treat references to them as external integrations, not in-repo crates.

Current workspace release version: `0.14.1`.

Use `README.md`, crate manifests, and the crate wireframes to confirm current details before changing public APIs.

Canonical generated domains use `solverforge::planning_model!` in
`src/domain/mod.rs` as the model-owned metadata manifest. Entity, fact, and
solution files remain normal separate Rust modules listed in that manifest.

## Documentation Layer

Treat the repository documentation as a coordinated surface, not as isolated files.

- `README.md` is the user-facing entry point. Keep the first screen oriented
  to a first-time human user: what SolverForge is, when to use it, how to try
  it with `solverforge-cli`, and where to go next. Do not turn the README into
  a long release-history dump; use `CHANGELOG.md` for release history and
  `RELEASE.md` for maintainer release operations.
- `crates/*/WIREFRAME.md` files are the canonical public API maps. Update them for any public surface change.
- `docs/*.md` files capture focused extension and architecture guidance. When a refactor changes naming or explains an intentional boundary, update the relevant doc or add a dedicated audit note.
- `docs/naming-charter.md` is the canonical naming contract for scalar/list terminology. Keep it in sync with any public rename or cleanup sweep.
- `AGENTS.md` records repository-specific rules for future coding agents. Update it when the engineering workflow or documentation policy changes.
- Documentation must describe the current checked-in code and public surface, not an intended future design. If a refactor is incomplete, document the shipped boundary and current limitation explicitly instead of documenting the target state as if it already exists.
- Distinguish crate-root re-exports from module-level exports and hidden `__internal` bridges. Do not document a module-only or macro-only symbol as if it were a supported root-level facade export.

## Wireframes Are Canonical

Each crate has a `WIREFRAME.md` file that documents its public API surface, module map, and usage patterns. Treat the relevant wireframe as the canonical reference before changing a crate.

Current wireframes:

- `crates/solverforge/WIREFRAME.md`
- `crates/solverforge-core/WIREFRAME.md`
- `crates/solverforge-macros/WIREFRAME.md`
- `crates/solverforge-scoring/WIREFRAME.md`
- `crates/solverforge-solver/WIREFRAME.md`
- `crates/solverforge-config/WIREFRAME.md`
- `crates/solverforge-console/WIREFRAME.md`
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

Selector cursors own move storage. Search phases evaluate borrowable candidates by stable index and materialize exactly the selected winner by ownership. Do not reintroduce owned `open_cursor()` streams for cartesian composition.

Clone is acceptable only for:

1. Solution snapshots sent through channels.
2. Partitioned search creating partition copies.
3. Explicit user-owned result snapshots outside retained scoring state, such as
   `CollectedVec::to_vec()` when the caller deliberately asks for an owned
   `Vec<T>`.

### Collector Ownership

Grouped collectors own extracted values. `Accumulator::accumulate(value)` takes
the mapped value by ownership and returns a retraction token; grouped,
cross-grouped, projected-grouped, complemented-grouped, and projected
complemented-grouped constraints cache that token for exact retraction after
entity mutation. Do not restore a borrowed `accumulate(&value)` /
`retract(&value)` protocol, and do not require `Copy`, `Clone`, or `PartialEq`
for payloads solely so `collect_vec` can retain them. `collect_vec` returns a
`CollectedVec<T>` result view, not an owned `Vec<T>` scoring result.

### Joined Filter Indexes

Low-level joined filter traits receive the semantic source indexes for the rows
being tested. Do not replace them with placeholder values in builders,
finalizers, or grouped/projection paths.

- Self-join `BiFilter`/`TriFilter`/`QuadFilter`/`PentaFilter` indexes are the
  same-source entity indexes in canonical tuple order.
- Cross-bi filters use the left and right source slice indexes.
- Flattened-bi filters use the A source index and the owning B source index for
  the flattened C row.
- Projected-bi filters use the projected row's primary owner entity index; row
  orientation is still determined by `ProjectedRowCoordinate`, and retained
  storage row IDs are never semantic.
- Public fluent `.filter(...)` methods stay entity/value-only and adapt to the
  indexed trait shape internally.

### Neighborhood Hot Paths

List, nearby-list, and sublist selector cursor paths are production hot loops.

- Preserve canonical enumeration order. Seeded order drift is a regression.
- Generated moves from finite selectors must remain `is_doable`.
- Cartesian selectors must keep preview-safe left-child validation, selector-order tabu composition, and borrowable sequential candidates.
- Benchmark touched neighborhood families in release mode before and after refactors.
- Shared helpers are acceptable for selected-entity snapshots, candidate ordering, and exact `size()` accounting, but keep `open_cursor()` loops explicit when abstraction hurts throughput or clarity.
- Do not accept a median throughput regression greater than 5% without explicit user approval.

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
- Keep Rust source and test files under 500 LOC. When a module grows beyond
  that boundary, split it by subsystem or behavior into adjacent implementation
  files and keep the public module entrypoint small.
- Prefer doctests for public APIs when practical, and make them compile meaningfully.
- Keep examples and scaffold-facing guidance on the public fluent API surface. If an example requires internal wiring, improve the public API instead.
- Canonical multi-file move and selector behavior tests belong under subsystem `tests/` trees. Do not reintroduce parallel `*_tests.rs` siblings once coverage has been consolidated.
- When selector or construction behavior changes, update the dedicated `tests/` tree for that subsystem instead of adding ad hoc sibling test entrypoints.

## Change Policy

- Do not keep compatibility shims, fallback implementations, or transitional APIs.
- Do not add aliases or targeted handling for removed configuration keys solely to support deprecated solver files.
- Delete dead code rather than hiding it behind `_unused` names or "removed" comments.
- When a design changes, migrate call sites to the new design instead of preserving the old path.
- Do not mention external solver frameworks in code comments, examples, or commits. SolverForge is an independent implementation.
- Version bumps require explicit user confirmation.
- `CHANGELOG.md` stays under `commit-and-tag-version`; do not hand-edit routine release notes unless the user explicitly asks for it.
- GitHub Release bodies are generated by the release workflow from GitHub release notes; contributor credit on the release page comes from the `@mentions` in those generated notes.

## Build And Validation

- `make build`
- `make examples`
- `make test`
- `make pre-release`

`make examples` derives the package list from checked-in `examples/*/Cargo.toml`
manifests, so every workspace example should be covered without a manually
maintained package loop.

Minimum supported Rust version: `1.95+`
