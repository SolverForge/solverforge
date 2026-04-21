# solverforge WIREFRAME

Facade crate: re-exports the public API from all sub-crates under a single `solverforge` dependency.

**Location:** `crates/solverforge/`
**Workspace Release:** `0.8.11`

The CLI lives in the standalone `solverforge-cli` repository and is not part of this workspace or facade crate.

## Dependencies

- `solverforge-core` (path) — Score types, domain traits
- `solverforge-macros` (path) — Attribute and derive macros
- `solverforge-scoring` (path) — Constraint API, Director
- `solverforge-solver` (path) — Solver engine, manager, phases
- `solverforge-config` (path) — Configuration types
- `solverforge-cvrp` (path) — CVRP domain helpers
- `solverforge-console` (path, optional) — Console output (feature-gated)

## Features

| Feature | Effect |
|---------|--------|
| `decimal` | Enables `solverforge-core/decimal` (HardSoftDecimalScore backed by rust_decimal) |
| `serde` | Enables `solverforge-core/serde` (Serialize/Deserialize for score types) |
| `console` | Enables `dep:solverforge-console` (terminal output) |
| `verbose-logging` | Enables verbose tracing output |

## File Map

```
src/
├── __internal.rs  — Hidden macro support re-exports and helpers
├── cvrp.rs        — CVRP facade re-exports
├── lib.rs         — Top-level crate re-exports and module declarations
├── prelude.rs     — Prelude exports
└── stream.rs      — Fluent constraint stream facade
```

## Public Re-exports

### Attribute Macros (from `solverforge-macros`)

- `planning_entity`
- `planning_solution`
- `problem_fact`

### Derive Macros (from `solverforge-macros`)

- `PlanningEntityImpl`
- `PlanningSolutionImpl`
- `ProblemFactImpl`

### Score Types (from `solverforge-core`)

- `Score` (trait)
- `SoftScore`
- `HardSoftScore`
- `HardMediumSoftScore`
- `HardSoftDecimalScore`
- `BendableScore`

### Constraint API (from `solverforge-scoring`)

- `ConstraintSet` (trait)
- `IncrementalConstraint` (trait)
- `IncrementalUniConstraint`
- `IncrementalBiConstraint`

### Score Director (from `solverforge-scoring`)

- `Director` (trait)
- `ScoreDirector`

### Configuration (from `solverforge-config`)

- `SolverConfig`
- `SolverConfigOverride`

### Solver (from `solverforge-solver`)

- `run_solver`
- `run_solver_with_config`
- `analyze` (free function)
- `Solvable` (trait)
- `Analyzable` (trait)
- `AnyTermination`
- `build_termination`
- `SolverManager`
- `SolverRuntime`
- `SolverEvent`
- `SolverEventMetadata`
- `SolverLifecycleState`
- `SolverStatus`
- `SolverManagerError`
- `SolverSnapshot`
- `SolverSnapshotAnalysis`
- `SolverTelemetry`
- `SolverTerminalReason`
- `ScoreAnalysis`
- `ConstraintAnalysis`
- `DefaultDistanceMeter`
- `CrossEntityDistanceMeter`

### CVRP Domain Helpers (from `solverforge-cvrp`)

Module: `solverforge::cvrp`

- `VrpSolution` (trait)
- `ProblemData`
- `MatrixDistanceMeter`
- `MatrixIntraDistanceMeter`
- `replace_route`, `get_route`
- `capacity`, `depot_for_cw`, `depot_for_entity`
- `distance`, `element_load`
- `is_kopt_feasible`, `is_time_feasible`

### Console (feature-gated)

- `solverforge::console` module (re-exports `solverforge-console`)

## `prelude` Module

Convenient single import for user code:

```rust
pub use crate::stream::{joiner, ConstraintFactory};
pub use crate::{
    planning_entity, planning_solution, problem_fact,
    BendableScore, ConstraintSet, HardMediumSoftScore,
    HardSoftDecimalScore, HardSoftScore, Score, Director,
    SoftScore, ScoreDirector,
};
```

## `stream` Module

Re-exports the fluent constraint stream API:

```rust
pub use solverforge_scoring::stream::collection_extract::vec;
pub use solverforge_scoring::stream::collection_extract::{
    tracked, ChangeSource, CollectionExtract, FlattenExtract, TrackedCollectionExtract,
    TrackedExtract, VecExtract,
};
pub use solverforge_scoring::stream::{joiner, ConstraintFactory, FlattenedCollectionTarget};
```

Key stream API: `ConstraintFactory::new().for_each(extractor).filter(pred).penalize(weight).named("name")` — no `as_constraint`, no `for_each_unique_pair`, no `join_self`/`join_keyed`. Use `.join(target)` for all join patterns (self-join, keyed, predicate).

Extractor ergonomics: all `for_each` and join extractor params accept `CollectionExtract<S, Item = A>`. Use `|s| s.field.as_slice()` for slices, or `vec(|s| &s.field)` when the field is a `Vec<A>` and you prefer `&field` syntax.

Tracked existence ergonomics: `ConstraintFactory::for_each_tracked()` and generated `{Name}ConstraintStreams` accessors produce tracked uni streams, exposing `ChangeSource` metadata for incremental `.if_exists(...)` / `.if_not_exists(...)`. Flattened existence targets use `.flattened(...)` and `FlattenedCollectionTarget`.

## `__internal` Module (`#[doc(hidden)]`)

Used exclusively by macro-generated code. Not public API.

### Re-exports

**Domain types (from `solverforge-core::domain`):**
- `PlanningEntity`, `PlanningSolution`, `PlanningId`, `ProblemFact`
- `EntityDescriptor`, `SolutionDescriptor`, `ProblemFactDescriptor`, `VariableDescriptor`
- `EntityCollectionExtractor`
- `ShadowVariableKind`

**Scoring (from `solverforge-scoring`):**
- `Director`, `ScoreDirector`
- `SolvableSolution`

**Solver infrastructure (from `solverforge-solver`):**
- `FromSolutionEntitySelector`, `DefaultCrossEntityDistanceMeter`, `DefaultDistanceMeter`
- `KOptPhaseBuilder`, `ListConstructionPhaseBuilder`
- `PhaseFactory`, `SolverFactory`
- `Construction`, `ConstructionArgs`, `PhaseSequence`, `RuntimePhase`
- `ProgressCallback`, `SolverScope`
- `SolverRuntime`, `SolverEvent`, `SolverTelemetry`
- `build_phases`, `descriptor_has_bindings`, `log_solve_start`, `run_solver`, `run_solver_with_config`
- `ListVariableEntity`, `ListVariableMetadata`

**Config (from `solverforge-config`):**
- `SolverConfig`

**Stream types for macro-generated extension traits (from `solverforge-scoring`):**
- `ChangeSource`, `CollectionExtract`, `TrackedExtract`
- `UniConstraintStream`, `UniConstraintBuilder`
- `TrueFilter`, `UniFilter`, `FnUniFilter`, `AndUniFilter`

**Derive macros (from `solverforge-macros`):**
- `PlanningEntityImpl`, `PlanningSolutionImpl`, `ProblemFactImpl`

### Functions

| Function | Signature | Note |
|----------|-----------|------|
| `init_console` | `fn()` | No-op unless `console` feature enabled |
| `load_solver_config` | `fn() -> SolverConfig` | Loads `solver.toml`, falling back to `SolverConfig::default()` |

## Architectural Notes

- **Pure re-export crate.** Contains zero implementation logic — only `pub use` statements and the `__internal` module.
- **`__internal` module** exists so that macro-generated code can reference types via `::solverforge::__internal::*` paths. This allows derive macros in `solverforge-macros` to generate code that compiles in user crates that only depend on `solverforge`.
- **Neutral naming surface.** The facade now exposes the prefix-free selector and extractor terminology used across the workspace, including `EntityCollectionExtractor` in `__internal` and the renamed solver selector modules documented in the solver wireframe.
- **Retained lifecycle surface.** The facade re-exports the retained job / snapshot / checkpoint lifecycle contract from `solverforge-solver`, including exact pause/resume, lifecycle-complete events, and snapshot-bound analysis types.
- **Prelude** provides the minimal set of types needed for defining domain models and constraints. Users import `use solverforge::prelude::*` and get attribute macros, score types, constraint traits, and the stream API.
- **Feature flags** propagate to sub-crates: `decimal` → `solverforge-core/decimal`, `serde` → `solverforge-core/serde`.
