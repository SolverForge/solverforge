# solverforge-bridge WIREFRAME

Public dynamic model bridge contracts for SolverForge host-language bindings.

**Location:** `crates/solverforge-bridge/`
**Workspace Release:** `0.15.2`

This crate is the boundary between monomorphized Rust models and dynamic
binding models. It is additive: the macro-generated Rust path remains in
`solverforge-core`, `solverforge-scoring`, and `solverforge-solver` and stays
the performance ceiling.

## Dependencies

- `solverforge-core` (path) - logical IDs, planning solution traits, descriptors, dynamic slots, score traits
- `solverforge-scoring` (path) - `ConstraintSet` and `ScoreDirector`
- `solverforge-config` (path) - `SolverConfig`
- `solverforge-solver` (path) - runtime runner, phases, and `SolverRuntime`

No feature flags are currently declared.

## File Map

```text
src/
├── backend.rs       - Re-exports dynamic model access traits from `solverforge-core`
├── backend_tests.rs - Backend re-export tests
├── ids.rs           - Re-exports logical descriptor ID types from `solverforge-core`
├── lib.rs           - Crate root; module declarations and public re-exports
├── runner.rs        - Binding-oriented dynamic runner helper
├── score.rs         - `DynamicScore`, `DynamicScoreFamily`, and scoped family guard
├── score_tests.rs   - Dynamic score parse/display/arithmetic tests
└── slots.rs         - Re-exports dynamic scalar/list slot adapters from `solverforge-core`
```

## Public Re-exports

```rust
pub use backend::{DynamicListAccess, DynamicModelBackend, DynamicScalarAccess};
pub use ids::{EntityClassId, ProblemFactClassId, VariableId};
pub use runner::run_dynamic_solver_with_config;
pub use score::{scoped_dynamic_score_family, DynamicScore, DynamicScoreFamily};
pub use slots::{DynamicListVariableSlot, DynamicScalarVariableSlot};
```

## Public Types

### Logical IDs

Re-exported from `solverforge-core::domain`:

- `EntityClassId(pub usize)`
- `ProblemFactClassId(pub usize)`
- `VariableId(pub usize)`

Binding models use these IDs to bind host-language entity, fact, and variable
classes to `SolutionDescriptor` metadata without relying on Rust `TypeId` or
descriptor order.

### Dynamic Backend Traits

Re-exported from `solverforge-core::domain`:

- `DynamicModelBackend`
- `DynamicScalarAccess<S>`
- `DynamicListAccess<S>`

`DynamicModelBackend` is the normal implementation target for Rust-owned
dynamic solution state. `DynamicScalarAccess` and `DynamicListAccess` are
object-safe access traits used by dynamic slot adapters.

### Dynamic Variable Slots

Re-exported from `solverforge-core::domain`:

- `DynamicScalarVariableSlot<S>`
- `DynamicListVariableSlot<S>`

Slots carry logical entity/variable IDs, human-readable entity and variable
names, dynamic access adapters, and a resolved descriptor index. Runtime
builders resolve slots against `SolutionDescriptor` before construction or
local-search selector assembly so score-director notifications use descriptor
indexes rather than raw logical IDs.

### `DynamicScoreFamily`

```rust
pub enum DynamicScoreFamily {
    Soft,
    HardSoft,
    HardSoftDecimal,
    HardMediumSoft,
}
```

`HardMediumSoft` is the default family.

### `DynamicScore`

```rust
pub struct DynamicScore {
    pub hard: i64,
    pub medium: i64,
    pub soft: i64,
    pub family: DynamicScoreFamily,
}
```

Constructors:

- `ZERO`
- `of(hard, medium, soft)`
- `with_family(hard, medium, soft, family)`
- `soft(soft)`
- `hard_soft(hard, soft)`
- `hard_soft_decimal(hard_scaled, soft_scaled)`
- `hard_medium_soft(hard, medium, soft)`
- `zero_for_family(family)`

Methods:

- `family_levels(self, family) -> Vec<i64>`

Implements:

- `Score`
- `ParseableScore`
- `Display`
- `Debug`
- `Add`, `Sub`, `Neg`
- `Ord`, `PartialOrd`, `Eq`, `PartialEq`, `Hash`, `Clone`, `Copy`, `Default`

The static `Score` trait still reports three levels. The `family` field controls
presentation and host-boundary conversion. `scoped_dynamic_score_family(family,
callback)` sets the thread-local family used by `DynamicScore::zero()` and
`DynamicScore::from_level_numbers(...)` during the callback.

## Public Functions

### `run_dynamic_solver_with_config`

```rust
pub fn run_dynamic_solver_with_config<S, C, P, BuildPhases>(
    solution: S,
    constraints: C,
    descriptor: SolutionDescriptor,
    entity_count_by_descriptor: fn(&S, usize) -> usize,
    runtime: SolverRuntime<S>,
    config: SolverConfig,
    default_time_limit_secs: u64,
    is_trivial: fn(&S) -> bool,
    log_scale: fn(&S),
    build_phases: BuildPhases,
) -> S
where
    S: PlanningSolution,
    S::Score: Score + ParseableScore,
    C: ConstraintSet<S, S::Score>,
    P: Phase<S, ScoreDirector<S, C>, solverforge_solver::run::ChannelProgressCallback<S>>
        + Send
        + std::fmt::Debug,
    BuildPhases: Fn(&SolverConfig, &SolutionDescriptor) -> P;
```

This is the binding-oriented entrypoint. The caller supplies descriptor,
constraints, config, entity-count callback, retained runtime, and phase builder
values instead of relying on macro-generated factories.

## Scope

- Stable logical IDs for dynamic entity, fact, and variable classes.
- Dynamic planning-model backend and slot contracts.
- Dynamic scalar/list runtime slot re-exports for host-language bindings.
- Dynamic score support for the binding path.
- Public runner helper for already-built dynamic runtime parts.

## Non-Goals

- Python-specific types.
- Generated Rust.
- String-parsed constraints.
- A second solver engine.
