# solverforge-bridge WIREFRAME

Public dynamic model bridge contracts for SolverForge host-language bindings.

**Location:** `crates/solverforge-bridge/`
**Workspace Release:** `0.23.0`

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
├── backend.rs       - Re-exports dynamic model access, list capability/metadata, and scalar-assignment metadata contracts from `solverforge-core`
├── backend_tests.rs - Backend re-export tests
├── ids.rs           - Re-exports logical descriptor ID types from `solverforge-core`
├── lib.rs           - Crate root; module declarations and public re-exports
├── runner.rs        - Binding-oriented dynamic runner helper
├── score.rs         - `DynamicScore`, `DynamicScoreFamily`, and the scoped presentation family
├── score_tests.rs   - Dynamic score parse/display/arithmetic tests
└── slots.rs         - Re-exports dynamic scalar/list slot adapters from `solverforge-core`
```

## Public Re-exports

```rust
pub use backend::{
    DynamicListAccess, DynamicListAccessCapabilities, DynamicListMetadata,
    DynamicListMetadataCapabilities, DynamicModelBackend, DynamicScalarAccess,
    DynamicScalarAssignmentMetadata, DynamicScalarAssignmentMetadataCapabilities,
};
pub use ids::{EntityClassId, ProblemFactClassId, VariableId};
pub use runner::try_run_dynamic_solver_with_config_parts;
pub use score::{
    active_dynamic_score_family, scoped_dynamic_score_family, DynamicScore, DynamicScoreFamily,
};
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
- `DynamicListAccessCapabilities`
- `DynamicListMetadata<S>`
- `DynamicListMetadataCapabilities`
- `DynamicScalarAssignmentMetadata<S>`
- `DynamicScalarAssignmentMetadataCapabilities`

`DynamicModelBackend` is the normal implementation target for Rust-owned
dynamic solution state. `DynamicScalarAccess` and `DynamicListAccess` are
object-safe access traits used by dynamic slot adapters. A custom scalar access
adapter may additionally declare and lazily visit nearby value/entity sources,
provide optional distances, and distinguish a row-local absent source from an
empty source so the canonical dynamic nearby selectors preserve fallback and
source-consumption semantics.

`DynamicListAccessCapabilities` declares whether one slot implements `set`,
whole-row `replace`, `reverse`, and atomic sublist operations. Optional access
methods return `false` or `None` when their capability is absent; SolverForge
does not emulate a missing operation by composing basic mutations.

`DynamicListMetadataCapabilities` declares immutable, slot-bound
`element_owner`, construction-order, precedence-duration/successor,
cross/intra-position distance, route, and savings bundles.
`DynamicListMetadata<S>` supplies the corresponding owner/order/precedence,
distance, route depot/distance/feasibility, and savings
depot/metric-class/distance/feasibility values. Metadata identity is bound to
the same logical entity and variable IDs as the slot, and a selected phase
fails validation when its required bundle is absent.

### Dynamic Assignment Metadata

`DynamicScalarAssignmentMetadata<S>` is a declarative, group-bound metadata
contract for a dynamic nullable scalar assignment group. A bridge consumer
creates one `Send + Sync` metadata object for one group registration; it must
not select a group through a phase name, active solve, or thread-local state.
Its capabilities declare which hooks are structurally present, and an
assignment rule requires a sequence key. Hooks whose capability is absent
return neutral values (`false`, `None`, or `true` for an edge check).

The bridge exposes metadata only. It never supplies an alternate construction
stream, custom dynamic phase, or selector escape hatch: SolverForge compiles
the registered group into its canonical construction and grouped local-search
paths.

### Dynamic Variable Slots

Re-exported from `solverforge-core::domain`:

- `DynamicScalarVariableSlot<S>`
- `DynamicListVariableSlot<S>`

Slots carry logical entity/variable IDs, human-readable entity and variable
names, dynamic access adapters, optional immutable list metadata, and resolved
descriptor/variable indexes. Runtime
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
}
```

Constructors:

- `ZERO`
- `of(hard, medium, soft)`
- `soft(soft)`
- `hard_soft(hard, soft)`

Methods:

- `family_levels(self, family) -> Vec<i64>`

Implements:

- `Score`
- `ParseableScore`
- `Display`
- `Debug`
- `Add`, `Sub`, `Neg`
- `Ord`, `PartialOrd`, `Eq`, `PartialEq`, `Hash`, `Clone`, `Copy`, `Default`

Every family shares one three-level value: unused levels stay zero and
`HardSoftDecimal` levels hold values scaled by 100 000. Equality, ordering, and
hashing compare the levels only, and `zero()`, `from_level_numbers(...)`, and
the arithmetic never read a family, so scores computed on any thread agree.
Arithmetic follows the core overflow contract: `Add`, `Sub`, `Neg`, `abs`,
`scale`, `multiply` and `divide` panic instead of wrapping or saturating.
Under `HardSoftDecimal` the text form is `HardSoftDecimalScore`'s: `Display`
prints it exactly and `parse` reads decimal literals with
`HardSoftDecimalScore::parse_level`, rejecting `NaN`, infinities, exponents,
more than five decimals and values outside `i64` after scaling.

### Family scope

```rust
pub fn scoped_dynamic_score_family<T>(
    family: DynamicScoreFamily,
    callback: impl FnOnce() -> T,
) -> T;

pub fn active_dynamic_score_family() -> DynamicScoreFamily;
```

The declared family is a property of the solve, held per thread. It decides
presentation and parsing only: `Display` renders the active family's levels,
and `ParseableScore::parse` reads the active family's units, scaling
whole-number parts under `HardSoftDecimal` so parsing inverts `Display`.
Bindings enter the family around the solve; parallel evaluation's worker
threads need no scope.

## Public Functions

### Compiled dynamic runner entrypoints

```rust
pub fn try_run_dynamic_solver_with_config_parts<S, C, V, DM, IDM>(
    solution: S,
    constraints: fn(&S) -> C,
    descriptor: SolutionDescriptor,
    entity_count_by_descriptor: fn(&S, usize) -> usize,
    runtime: SolverRuntime<S>,
    config: SolverConfig,
    default_time_limit_secs: u64,
    log_scale: fn(&S),
    qualified_candidate_trace_provenance: Option<QualifiedCandidateTraceRunProvenance>,
    model: RuntimeModel<S, V, DM, IDM>,
) -> RuntimeBuildResult<S>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score + Copy + Ord + ParseableScore,
    C: ConstraintSet<S, S::Score>,
    V: Clone + Copy + PartialEq + Eq + Hash + Into<usize> + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + Send + Sync + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + Send + Sync + 'static;
```

This is the binding-oriented entrypoint. It consumes frozen dynamic model parts
into the same compiled graph and phase runner used by native models. It accepts
no phase builder, selector registration, or alternate construction path.
`constraints` builds a constraint set for a given solution; the runtime calls it
for the working solution and for every evaluator replica, so
`move_thread_count` evaluates dynamic models in parallel. Bindings rebuild
their host constraint set from the solution they receive. Replicas score on
Rayon worker threads, so a binding whose host runtime has a global lock must
release it on the calling thread for the duration of the solve.
The optional qualified provenance changes only the candidate-trace header; it
does not select another runner.

## Scope

- Stable logical IDs for dynamic entity, fact, and variable classes.
- Dynamic planning-model backend and slot contracts.
- Explicit dynamic-list mutation and metadata capability contracts.
- Dynamic scalar-assignment metadata for declarative group registration.
- Dynamic scalar/list runtime slot re-exports for host-language bindings.
- Dynamic score support for the binding path.
- Public runner helper for already-built dynamic runtime parts.

## Non-Goals

- Python-specific types.
- Generated Rust.
- String-parsed constraints.
- A second solver engine.
- Dynamic construction cursors, phases, or selector registration.
