# solverforge WIREFRAME

Facade crate: re-exports the public API from all sub-crates under a single `solverforge` dependency.

**Location:** `crates/solverforge/`
**Workspace Release:** `0.10.0`

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

### Model Macros (from `solverforge-macros`)

- `planning_model`
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
- `ConstraintMetadata`
- `IncrementalConstraint` (trait)
- `IncrementalUniConstraint`
- `IncrementalBiConstraint`
- `Projection` (trait)
- `ProjectionSink` (trait)

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
- `SelectorTelemetry`
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
    planning_entity, planning_model, planning_solution, problem_fact,
    BendableScore, ConstraintMetadata, ConstraintSet, HardMediumSoftScore,
    HardSoftDecimalScore, HardSoftScore, Score, Director,
    SoftScore, ScoreDirector,
};
```

## `stream` Module

Re-exports the fluent constraint stream API:

```rust
pub use solverforge_scoring::stream::collection_extract::vec;
pub use solverforge_scoring::stream::collection_extract::{
    source, ChangeSource, CollectionExtract, FlattenExtract, SourceExtract, VecExtract,
};
pub use solverforge_scoring::stream::{joiner, ConstraintFactory, FlattenedCollectionTarget};
```

Key stream API: `ConstraintFactory::new().for_each(extractor).filter(pred).penalize(weight).named("name")` — no `as_constraint`, no `for_each_unique_pair`, no `join_self`/`join_keyed`. Use `.join(target)` for all join patterns (self-join, keyed, predicate).

Collector helpers are available at `solverforge::stream::collector`, and the
prelude re-exports `count`, `sum`, and `load_balance`.

Extractor ergonomics: all `for_each` and join extractor params accept `CollectionExtract<S, Item = A>`. Use `|s| s.field.as_slice()` for slices, or `vec(|s| &s.field)` when the field is a `Vec<A>` and you prefer `&field` syntax.

Generated keyed joins: unfiltered generated accessors can be passed directly as keyed join targets, preserving hidden `ChangeSource` metadata:
```rust
factory.assignments().join((
    ConstraintFactory::<Plan, HardSoftScore>::new().furnaces(),
    equal_bi(|assignment| assignment.furnace_idx(), |furnace| Some(furnace.id)),
))
```

Generated existence ergonomics: there is one public `ConstraintFactory::for_each(...)`. Generated `{Name}ConstraintStreams` accessors call it with hidden `ChangeSource::Descriptor(idx)` / `ChangeSource::Static` metadata so localized incremental callbacks use entity indexes only for the owning planning-entity collection. Raw facade `for_each(...)` extractors do not carry localized source ownership. Flattened existence targets use `.flattened(...)` and `FlattenedCollectionTarget`.

Projected scoring ergonomics: `factory.assignments().project(TaskShiftWorkEntries)` creates bounded derived scoring rows from a named `Projection<A>` type without materializing facts or entities. Projected streams can be filtered, self-joined, merged, grouped, and weighted like normal scoring state. Projections emit through `ProjectionSink` and declare `MAX_EMITS`; Vec-returning projection closures are not part of the public API. Projected self-join ordering is coordinate-stable by `(source_slot, entity_index, emission_index)`, not sparse storage row id.

## `__internal` Module (`#[doc(hidden)]`)

Used exclusively by macro-generated code. Not public API.

### Re-exports

**Domain types (from `solverforge-core::domain`):**
- `PlanningEntity`, `PlanningSolution`, `PlanningId`, `ProblemFact`
- `EntityDescriptor`, `SolutionDescriptor`, `ProblemFactDescriptor`, `VariableDescriptor`
- `EntityCollectionExtractor`
- `ShadowVariableKind`, `ValueRangeType`

**Scoring (from `solverforge-scoring`):**
- `Director`, `ScoreDirector`
- `SolvableSolution`

**Solver infrastructure (from `solverforge-solver`):**
- `ListVariableContext`, `LocalSearch`, `ModelContext`, `ScalarGroupCandidate`, `ScalarGroupContext`, `ScalarGroupEdit`, `ScalarGroupLimits`, `ScalarGroupMember`, `ScalarVariableContext`, `ValueSource`, `VariableContext`, `Vnd`
- `FromSolutionEntitySelector`, `DefaultCrossEntityDistanceMeter`, `DefaultDistanceMeter`
- `KOptPhaseBuilder`, `ListConstructionPhaseBuilder`
- `PhaseFactory`, `SolverFactory`
- `Construction`, `PhaseSequence`, `RuntimePhase`
- `ProgressCallback`, `SolverScope`
- `SolverRuntime`, `SolverEvent`, `SolverTelemetry`
- `build_phases`, `descriptor_has_bindings`, `log_solve_start`, `run_solver`, `run_solver_with_config`
- `ListVariableEntity`, `ListVariableMetadata`
- `PlanningModelSupport`

Grouped scalar re-exports include the construction metadata surface on
`ScalarGroupCandidate` and the split `ScalarGroupLimits` fields used by grouped
construction and grouped local-search selectors.

**Config (from `solverforge-config`):**
- `PhaseConfig`, `SolverConfig`

**Async bridge types:**
- `tokio::sync::mpsc::UnboundedSender`

**Stream types for macro-generated extension traits (from `solverforge-scoring`):**
- `ChangeSource`, `CollectionExtract`, `SourceExtract`
- `UniConstraintStream`, `UniConstraintBuilder`
- `TrueFilter`, `UniFilter`, `FnUniFilter`, `AndUniFilter`
- `source`

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
- **Shape-aware startup telemetry.** Hidden runtime logging helpers under `__internal` emit `element_count` for list solves and average `candidate_count` for scalar solves so console startup output can label the scale correctly.
- **Macro-built runtime contexts stay model-owned.** `planning_model!` generates
  the hidden `PlanningModelSupport` impl that attaches nearby hooks plus scalar
  construction order-key hooks from `#[planning_variable]`, while list
  construction capabilities continue to come from `#[planning_list_variable]`.
  Construction order-key hooks are construction-only and do not reorder
  local-search scalar candidate neighborhoods.
- **Retained lifecycle surface.** The facade re-exports the retained job / snapshot / checkpoint lifecycle contract from `solverforge-solver`, including exact pause/resume, lifecycle-complete events, and snapshot-bound analysis types.
- **Prelude** provides the minimal set of types needed for defining domain models and constraints. Users import `use solverforge::prelude::*` and get attribute macros, score types, constraint traits, and the stream API.
- **Feature flags** propagate to sub-crates: `decimal` → `solverforge-core/decimal`, `serde` → `solverforge-core/serde`.
