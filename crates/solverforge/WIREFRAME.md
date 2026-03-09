# solverforge WIREFRAME

Facade crate: re-exports the public API from all sub-crates under a single `solverforge` dependency.

**Location:** `crates/solverforge/`

## Dependencies

- `solverforge-core` (path) — Score types, domain traits
- `solverforge-macros` (path) — Attribute and derive macros
- `solverforge-scoring` (path) — Constraint API, Director
- `solverforge-solver` (path) — Solver engine, manager, phases
- `solverforge-config` (path) — Configuration types
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
└── lib.rs  — All re-exports, prelude module, __internal module
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

### Solver (from `solverforge-solver`)

- `run_solver`
- `run_list_solver`
- `analyze` (free function)
- `Solvable` (trait)
- `Analyzable` (trait)
- `SolverManager`
- `SolverStatus`
- `ScoreAnalysis`
- `ConstraintAnalysis`
- `DefaultDistanceMeter`

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
pub use solverforge_scoring::stream::{joiner, ConstraintFactory};
```

## `__internal` Module (`#[doc(hidden)]`)

Used exclusively by macro-generated code. Not public API.

### Re-exports

**Domain types (from `solverforge-core::domain`):**
- `PlanningEntity`, `PlanningSolution`, `PlanningId`, `ProblemFact`
- `EntityDescriptor`, `SolutionDescriptor`, `ProblemFactDescriptor`, `VariableDescriptor`
- `TypedEntityExtractor`
- `ShadowVariableKind`

**Scoring (from `solverforge-scoring`):**
- `Director`, `ScoreDirector`
- `ShadowVariableSupport`, `SolvableSolution`

**Solver infrastructure (from `solverforge-solver`):**
- `FromSolutionEntitySelector`, `DefaultDistanceMeter`
- `KOptPhaseBuilder`, `ListConstructionPhaseBuilder`
- `PhaseFactory`, `SolverFactory`

**Config (from `solverforge-config`):**
- `SolverConfig`

**Derive macros (from `solverforge-macros`):**
- `PlanningEntityImpl`, `PlanningSolutionImpl`, `ProblemFactImpl`

### Functions

| Function | Signature | Note |
|----------|-----------|------|
| `init_console` | `fn()` | No-op unless `console` feature enabled |

## Architectural Notes

- **Pure re-export crate.** Contains zero implementation logic — only `pub use` statements and the `__internal` module.
- **`__internal` module** exists so that macro-generated code can reference types via `::solverforge::__internal::*` paths. This allows derive macros in `solverforge-macros` to generate code that compiles in user crates that only depend on `solverforge`.
- **Prelude** provides the minimal set of types needed for defining domain models and constraints. Users import `use solverforge::prelude::*` and get attribute macros, score types, constraint traits, and the stream API.
- **Feature flags** propagate to sub-crates: `decimal` → `solverforge-core/decimal`, `serde` → `solverforge-core/serde`.
