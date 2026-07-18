# solverforge WIREFRAME

Facade crate: re-exports the public API from all sub-crates under a single `solverforge` dependency.

**Location:** `crates/solverforge/`
**Workspace Release:** `0.19.2`

The CLI lives in the standalone `solverforge-cli` repository and is not part of this workspace or facade crate.

## Dependencies

- `solverforge-core` (path) — Score types, domain traits
- `solverforge-bridge` (path) — Dynamic host-language binding contracts
- `solverforge-macros` (path) — Attribute and derive macros
- `solverforge-scoring` (path) — Constraint API, Director
- `solverforge-solver` (path) — Solver engine, manager, phases
- `solverforge-config` (path) — Configuration types
- `solverforge-cvrp` (path) — CVRP profile helpers, traits, meters, and hook bundles
- `solverforge-console` (path, optional) — Console output (feature-gated)

## Features

| Feature | Effect |
|---------|--------|
| `default` | Empty default feature set |
| `decimal` | Forwards the currently empty `solverforge-core/decimal` feature; fixed-scale `HardSoftDecimalScore` is always available |
| `serde` | Enables `solverforge-core/serde` (Serialize/Deserialize for score types) |
| `console` | Enables `dep:solverforge-console` (terminal output) |
| `verbose-logging` | Enables verbose tracing output |

## File Map

```
src/
├── __internal.rs  — Hidden macro support re-exports and helpers
├── cvrp.rs        — CVRP facade re-exports
├── lib.rs         — Top-level crate re-exports and module declarations
├── planning.rs    — Planning target helpers for scalar groups and conflict repair
├── prelude.rs     — Prelude exports
└── stream.rs      — Fluent constraint stream facade
```

## Public Re-exports

### Model Macros (from `solverforge-macros`)

- `planning_model`
- `solverforge_constraints`
- `planning_entity`
- `planning_solution`
- `problem_fact`

### Score Types (from `solverforge-core`)

- `Score` (trait)
- `SoftScore`
- `HardSoftScore`
- `HardMediumSoftScore`
- `HardSoftDecimalScore`
- `BendableScore`

### Logical Descriptor IDs (from `solverforge-core::domain`)

- `EntityClassId`
- `ProblemFactClassId`
- `VariableId`

### Dynamic Bridge

- `solverforge::bridge` module alias, re-exporting `solverforge-bridge`

### Constraint API (from `solverforge-scoring`)

- `fixed_weight`
- `hard_weight`
- `FixedWeight`
- `HardWeight`
- `ConstraintSet` (trait)
- `ConstraintMetadata<'a>` (borrowed constraint identity view)
- `SharedNodeDiagnostics`, `SharedNodeId`, `SharedNodeOperation`
- `IncrementalConstraint` (trait)
- `IncrementalConstraintSealed` (marker trait required by direct custom `IncrementalConstraint` implementors)
- `IncrementalUniConstraint`
- `IncrementalBiConstraint`
- `ListPrecedenceMakespanConstraint`
- `Projection` (trait)
- `ProjectionSink` (trait)

### Score Director (from `solverforge-scoring`)

- `Director` (trait)
- `ScoreDirector`

### Configuration (from `solverforge-config`)

- `AcceptorConfig`
- `CandidateTraceConfig`
- `ConstructionHeuristicType`
- `ConstructionObligation`
- `EnvironmentMode`
- `ForagerConfig`
- `HardRegressionPolicyConfig`
- `MoveSelectorConfig`
- `MoveThreadCount`
- `PhaseConfig`
- `RecreateHeuristicType`
- `SolverConfig`
- `SolverConfigOverride`
- `UnionSelectionOrder`

### Solver (from `solverforge-solver`)

- `analyze` (free function)
- `Solvable` (trait)
- `Analyzable` (trait)
- `AppliedMoveTelemetry`
- `MoveTelemetry`
- `RepairLimits`
- `ConflictRepair`
- `RepairCandidate`
- `RepairProvider`
- `ScalarAssignmentRule`
- `ScalarCandidate`
- `ScalarEdit`
- `ScalarGroup`
- `ScalarGroupLimits`
- `ScalarTarget`
- `ScalarCandidateProvider`
- `SolverManager`
- `SolverRuntime`
- `SolverEvent`
- `SolverEventMetadata`
- `SolverLifecycleState`
- `SolverStatus`
- `SolverTelemetryDetail`
- `SolverManagerError`
- `SolverSnapshot`
- `SolverSnapshotAnalysis`
- `PhaseTelemetry`
- `SolverTelemetry`
- `SelectorTelemetry`
- `SolverTerminalReason`
- candidate-trace graph and payload types: `CandidatePullTelemetry`,
  `CandidateTraceCompositeIdentity`, `CandidateTraceConstructionTarget`,
  `CandidateTraceCoordinate`, `CandidateTraceDigest`,
  `CandidateTraceDisposition`, `CandidateTraceExecutionPolicy`,
  `CandidateTraceExternalDigest`, `CandidateTraceHeader`,
  `CandidateTraceIdentity`, `CandidateTraceInputAttestation`,
  `CandidateTraceInputProvenance`, `CandidateTraceInputProvenanceStatus`,
  `CandidateTraceOperationIdentity`, `CandidateTracePhaseAttribute`,
  `CandidateTracePhasePlan`, `CandidateTraceProvenanceStatus`,
  `CandidateTraceQualificationError`, `CandidateTraceQualificationStatus`,
  `CandidateTraceSource`, `CandidateTraceTelemetry`,
  `QualifiedCandidateTraceRunProvenance`, and
  `CANDIDATE_TRACE_FORMAT_VERSION`
- `ScoreAnalysis`
- `ConstraintAnalysis` (solver-level serializable analysis)
- `DefaultDistanceMeter`
- `CrossEntityDistanceMeter`
- `Search`
- `SearchContext`
- `CustomSearchPhase`
- `ExhaustiveSearchConfig`
- `ExhaustiveSearchPhase`
- `ExplorationType`
- `FunctionalPartitioner`
- `PartitionedSearchPhase`
- `SimpleDecider`
- `SolutionPartitioner`
- `ThreadCount`
- `local_search`

### Planning Helpers

Module: `solverforge::planning`

- `EntitySourceTargetExt` — extension trait for macro-generated model-owned
  planning entity sources. `scalar(&self, variable_name)` borrows the source
  and returns a `ScalarTarget<S>`, so one bound generated source can declare
  multiple scalar targets for grouped scalar construction or local search.
- Scalar helpers — `ScalarTarget<S>`, `ScalarEdit<S>`, `ScalarCandidate<S>`,
  `ScalarAssignmentRule<S>`, `ScalarGroup<S>`, and `ScalarGroupLimits`
  describe public grouped-scalar construction, stock scalar
  assignment, and local-search declarations. Assignment-backed scalar targets
  are owned by their named `ScalarGroup`; construction and local search reach
  those slots through grouped scalar configuration instead of generic scalar
  selectors.
- Conflict-repair helpers — `ConflictRepair<S>`, `RepairCandidate<S>`, and
  `RepairLimits` describe domain-provided repair candidates while the framework
  owns filtering, scoring, hard-improvement gates, and selector telemetry.

### CVRP Domain Helpers (from `solverforge-cvrp`)

Module: `solverforge::cvrp`

- `VrpSolution` (trait)
- `ProblemData`
- `UNREACHABLE`
- `MatrixDistanceMeter`
- `MatrixIntraDistanceMeter`
- `replace_route`, `get_route`
- hook modules: `route_hooks`, `savings_hooks`
- route-local helpers: `depot_for_entity`, `route_distance`, `route_feasible`
- Clarke-Wright construction adapters: `savings_depot_for_entity`, `savings_metric_class`, `savings_distance`, `savings_feasible`

Stock CVRP route lists should normally use
`#[planning_list_variable(element_collection = "...", domain = "cvrp")]`. The
domain profile expands to the helper modules above while keeping the
construction/local-search split out of ordinary model code: route hooks enforce
strict route-local CVRP capacity and time-window feasibility, while savings
hooks provide relaxed Clarke-Wright construction admissibility. Explicit hook
modules remain available for custom routing semantics when the `domain = "cvrp"`
profile is omitted.

### Console (feature-gated)

- `solverforge::console` module (re-exports `solverforge-console`)

## `prelude` Module

Convenient single import for user code:

```rust
pub use crate::local_search;
pub use crate::planning::EntitySourceTargetExt;
pub use crate::stream::collector::{collect_vec, consecutive_runs, count, indexed_presence, load_balance, sum, CollectedVec, IndexedPresence, Run, Runs};
pub use crate::stream::{joiner, ConstraintFactory};
pub use crate::{
    fixed_weight, hard_weight, planning_entity, planning_model, planning_solution, problem_fact,
    solverforge_constraints,
    BendableScore, ConflictRepair, ConstraintMetadata, ConstraintSet, CustomSearchPhase, Director,
    ExhaustiveSearchConfig, ExhaustiveSearchPhase, ExplorationType, FixedWeight,
    FunctionalPartitioner, HardMediumSoftScore, HardSoftDecimalScore, HardSoftScore, HardWeight,
    ListPrecedenceMakespanConstraint, PartitionedSearchPhase, Projection, ProjectionSink,
    RepairCandidate, RepairLimits, ScalarAssignmentRule, ScalarCandidate, ScalarEdit,
    ScalarGroup, ScalarGroupLimits, ScalarTarget, Score, ScoreDirector, Search, SearchContext,
    SharedNodeDiagnostics, SharedNodeId, SharedNodeOperation, SimpleDecider, SoftScore,
    SolutionPartitioner, ThreadCount,
};
```

## Typed Custom Search

Solutions can compile in custom search code with
`#[planning_solution(search = "search::search")]`. The search function returns
`impl Search<...>` and registers typed phases by name:

```rust
pub fn search<DM, IDM>(
    ctx: SearchContext<Schedule, usize, DM, IDM>,
) -> impl Search<Schedule, usize, DM, IDM>
where
    DM: CrossEntityDistanceMeter<Schedule> + Clone + std::fmt::Debug + Send + 'static,
    IDM: CrossEntityDistanceMeter<Schedule> + Clone + std::fmt::Debug + Send + 'static,
{
    ctx.defaults()
        .phase("weekend_repair", |ctx| WeekendRepair::new(ctx.model()))
        .phase("nurse_search", |ctx| {
            local_search(
                NurseMoves::new(ctx.model()),
                NurseAcceptor::new(ctx.seed()),
                NurseForager::new(),
            )
        })
}
```

TOML can order those compiled-in names with
`[[phases]] type = "custom" name = "weekend_repair"`. Custom phases implement
`CustomSearchPhase<S>`. SolverForge does not load arbitrary runtime classes or
use erased phase registries.

## `stream` Module

Re-exports the fluent constraint stream API:

```rust
pub use solverforge_scoring::stream::collection_extract::vec;
pub use solverforge_scoring::stream::collection_extract::{
    CollectionExtract, FlattenExtract, VecExtract,
};
pub use solverforge_scoring::stream::collector;
pub use solverforge_scoring::stream::{joiner, ConstraintFactory, FlattenedCollectionTarget};
```

Key stream API: `ConstraintFactory::new().for_each(extractor).filter(pred).penalize(weight).named("name")`. Use `.join(target)` for all join patterns: self-join, keyed cross-join, and predicate cross-join. Keyed and predicate cross-joins can take a right-hand `UniConstraintStream`, including one with its own `.filter(...)`. Keyed cross joins can either score pairs directly, group joined pairs with `.group_by(|left, right| key, collector)`, complement those grouped pairs with `.complement(source, key, default)`, or project joined pairs into rows with `.project(|left, right| row)`. `#[planning_solution]` also generates a solution-named convenience trait, such as `PlanConstraintStreams`, so callers can import the trait and write `ConstraintFactory::new().assignments()` for the same concrete source stream.

Collector helpers are available at `solverforge::stream::collector`, and the
prelude re-exports `collect_vec`, `count`, `sum`, `load_balance`,
`consecutive_runs`, `indexed_presence`, `CollectedVec`, `IndexedPresence`,
`Run`, and `Runs`. `collect_vec` owns mapped values once and exposes them as
`CollectedVec<T>`; `T` does not need `Copy`, `Clone`, or `PartialEq`. The
underlying `collector::Collector<Input>` trait is generic over the stream match
shape, so direct cross-join grouping uses the same collector protocol as unary
grouping and passes joined pairs as `(&A, &B)`.

## Workspace Examples

- `examples/scalar-graph-coloring` — scalar assignment using `planning_model!`, generated sources, `solver.toml`, and `SolverManager`
- `examples/minimal-shift-scheduling` — compact public solver path using assignment-backed `ScalarGroup`, `consecutive_runs`, generated sources, `solver.toml`, and `SolverManager`
- `examples/list-tsp` — list-variable route optimization
- `examples/mixed-job-shop` — mixed scalar/list planning model
- `examples/nqueens` — scalar assignment model

Extractor ergonomics: `for_each` accepts `CollectionExtract<S, Item = A>`. Raw keyed join targets accept any `CollectionExtract` target, including slice closures, `vec(|s| &s.field)`, custom extractor structs, and macro-generated solution source methods. Passing a right-hand `UniConstraintStream` remains the preferred form when the source needs its own `.filter(...)`.

Model-owned keyed joins use solution source methods generated by `#[planning_solution]`, preserving hidden descriptor/static metadata:
```rust
ConstraintFactory::<Plan, HardSoftScore>::new()
    .for_each(Plan::assignments())
    .join((
        Plan::furnaces(),
        equal_bi(|assignment| assignment.furnace_idx(), |furnace| Some(furnace.id)),
    ))
```

Generated existence ergonomics: `#[planning_solution]` generates inherent source methods such as `Plan::assignments()` and `Plan::furnaces()` with hidden descriptor/static metadata, plus the matching `PlanConstraintStreams` convenience trait. Localized incremental callbacks use entity indexes only for the owning planning-entity collection. Raw facade `for_each(...)` extractors do not carry localized source ownership. Flattened existence targets use `.flattened(...)` and `FlattenedCollectionTarget`.

Projected scoring ergonomics: `ConstraintFactory::new().for_each(Plan::assignments()).project(TaskShiftWorkEntries)` creates bounded scoring rows from a named `Projection<A>` type without materializing facts or entities. Keyed cross joins can group joined pairs directly with `.group_by(|left, right| key, collector)` and complement the grouped result, or project them with `.project(|assignment, capacity| Row { ... })` to emit one scoring row per retained joined pair. Projected streams can be filtered, self-joined, merged, grouped, complemented after grouping, and weighted as retained scoring state. Single-source projection implementations emit through `ProjectionSink` and declare `MAX_EMITS`; joined-pair closures do not need a helper type. Projected output rows, projected self-join keys, and grouped collector values do not need `Clone`. Projected self-join ordering is coordinate-stable by `RowCoordinate`; directed projected self-joins use `.join(joiner::equal_bi(left_key, right_key))`; low-level pair-filter indexes are primary owner entity indexes, not sparse storage row IDs.

Constraint authoring: `#[solverforge_constraints]` is the canonical attribute
for constraint factory functions. It preserves fluent stream syntax and gives
the macro crate a whole-function boundary for node sharing. Reusing the same
grouped stream binding across multiple `.penalize(...).named(...)` or
`.reward(...).named(...)` terminals emits one shared grouped node with separate
terminal scorers. The macro does not share separately written bindings by
source-text equivalence; reuse must be through the same local grouped-stream
binding inside the annotated function.

## `__internal` Module (`#[doc(hidden)]`)

Used exclusively by macro-generated code. Not public API.

### Re-exports

**Domain types (from `solverforge-core::domain`):**
- `PlanningEntity`, `PlanningSolution`, `PlanningId`, `ProblemFact`
- `EntityDescriptor`, `SolutionDescriptor`, `ProblemFactDescriptor`, `VariableDescriptor`
- `EntityClassId`, `ProblemFactClassId`, `VariableId`
- `EntityCollectionExtractor`
- `ShadowVariableKind`, `ValueRangeType`

**Scoring (from `solverforge-scoring`):**
- `Director`, `ScoreDirector`
- `SolvableSolution`
- Hidden constraint composition support: `ConstraintSetChain`, `OrderedConstraintSetChain`,
  `ConstraintSetSource`, `ConstraintWeight`, `ConstraintRef`, `ImpactType`,
  concrete stream source/filter types, and solver runtime helpers. Node sharing
  is finalized through the same fluent `.penalize(...).named(...)` /
  `.reward(...).named(...)` path as ordinary constraints; internal shared
  node-state types are not facade exports.

**Solver infrastructure (from `solverforge-solver`):**
- `bind_scalar_groups`, `local_search`, `CustomSearchPhase`, `Search`, `SearchContext`
- `ListVariableSlot`, `RuntimeModel`, `ScalarGroupBinding`, `ScalarGroupMemberBinding`, `ScalarVariableSlot`, `ValueSource`, `VariableSlot`, `usize_element_source_key`

Specialized list construction facades require an explicit `element_source_key`:
the stable unique `usize` identity shared by declared, assigned, and
precedence-successor element values. The solver binds it before construction
and uses source indexes rather than payload equality or hashing.
- `FromSolutionEntitySelector`, `DefaultCrossEntityDistanceMeter`, `DefaultDistanceMeter`
- `KOptPhaseBuilder`, `ListConstructionPhaseBuilder`
- `PhaseFactory`, `SolverFactory`
- `ProgressCallback`, `SolverScope`
- `SolverRuntime`, `SolverEvent`, `SolverTelemetry`
- `log_solve_start`, `RuntimeBuildError`, `RuntimeBuildResult`, `try_run_solver_with_config_and_search`
- `ListVariableEntity`, `ListVariableMetadata`
- `PlanningModelSupport`

Grouped scalar re-exports include the construction metadata surface on
`ScalarCandidate`, `ScalarAssignmentRule`, and the split
`ScalarGroupLimits` fields used by grouped construction and grouped
local-search selectors.

**Config (from `solverforge-config`):**
- `PhaseConfig`, `SolverConfig`

**Stream types for macro-generated source methods (from `solverforge-scoring`):**
- `ChangeSource`, `CollectionExtract`, `SourceExtract`
- `UniConstraintStream`, `UniConstraintBuilder`
- `TrueFilter`, `UniFilter`, `FnUniFilter`, `AndUniFilter`
- `source`, `UnassignedEntity`

**Macro support (hidden):**
- Attribute macros route their support derives through `__internal`; users should not import those derive macros directly.

### Functions

| Function | Signature | Note |
|----------|-----------|------|
| `init_console` | `fn()` | No-op unless `console` feature enabled |
| `load_solver_config` | `fn() -> SolverConfig` | Loads `solver.toml`, falling back to `SolverConfig::default()` |

## Architectural Notes

- **Facade crate.** Public modules are re-exports; implementation is limited to
  tiny hidden macro helpers such as `init_console()` and `load_solver_config()`.
- **`__internal` module** exists so that macro-generated code can reference types via `::solverforge::__internal::*` paths. This allows derive macros in `solverforge-macros` to generate code that compiles in user crates that only depend on `solverforge`.
- **Shape-aware startup telemetry.** Hidden runtime logging helpers under `__internal` emit `element_count` for list solves and average `candidate_count` for scalar solves so console startup output can label the scale correctly.
- **Macro-built runtime slots stay model-owned.** `planning_model!` generates
  the hidden `PlanningModelSupport` impl that attaches nearby hooks plus scalar
  construction order-key hooks from `#[planning_variable]` `Option<usize>`
  fields, and attaches list construction element-order and owner hooks from
  `#[planning_list_variable]`. Construction order-key hooks are
  construction-only and do not reorder local-search scalar candidate or list
  neighborhoods.
- **Partial fixed list ownership.** List variables can declare
  `element_owner_fn`; macro-generated runtime slots forward the hook to
  `solverforge-solver`, where `None` means unrestricted and valid `Some(owner)`
  fixes that element to one owner.
- **Retained lifecycle surface.** The facade re-exports the retained job / snapshot / checkpoint lifecycle contract from `solverforge-solver`, including exact pause/resume, lifecycle-complete events, and snapshot-bound analysis types.
- **Prelude** provides the common surface for generated and hand-written
  application code. Users import `use solverforge::prelude::*` and get
  attribute macros, score types, constraint traits, stream entry points,
  model-source scalar target helpers, and the public scalar-group and
  conflict-repair declaration types needed by generated applications.
- **Feature flags** propagate to sub-crates: `decimal` → `solverforge-core/decimal`, `serde` → `solverforge-core/serde`.
