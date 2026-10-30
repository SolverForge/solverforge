# solverforge WIREFRAME

Facade crate: re-exports the public API from all sub-crates under a single `solverforge` dependency.

**Location:** `crates/solverforge/`
**Workspace Release:** `0.23.0`

The CLI lives in the standalone `solverforge-cli` repository and is not part of this workspace or facade crate.

## Dependencies

- `solverforge-core` (path) — Score types, domain traits
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
| `serde` | Enables `solverforge-core/serde` (Serialize/Deserialize for score types) |
| `console` | Enables `dep:solverforge-console` (terminal output) |
| `verbose-logging` | Console output at DEBUG level; implies `console` |

## File Map

```
src/
├── __internal.rs  — Hidden macro support re-exports and helpers
├── cvrp.rs        — CVRP facade re-exports
├── lib.rs         — Crate docs (`//!`, Quick Start doctest), top-level re-exports and module declarations
├── planning.rs    — Planning target helpers for scalar groups and conflict repair
├── prelude.rs     — Prelude exports
└── stream.rs      — Fluent constraint stream facade
doc/quick_start/   — Quick Start model (employee, shift, schedule, solver.toml) included by the crate docs and compiled by its doctest
```

Every workspace crate (the facade, `solverforge-core`, `solverforge-config`, `solverforge-scoring`, `solverforge-solver`, `solverforge-console`, `solverforge-macros`, `solverforge-bridge` and `solverforge-cvrp`) sets `#![warn(missing_docs)]`; CI's clippy `-D warnings` makes an undocumented public item an error.

## Public Re-exports

### Layer Modules

Each module groups re-exports from one layer crate, so every type a facade signature uses is importable from `solverforge`. The solver groups list items explicitly, because `solverforge-solver` exports everything once at its root:

| Module | Re-exports |
|---|---|
| `config` | `solverforge_config::*` — every configuration type, including `TerminationConfig` and `ConfigError` |
| `domain` | `solverforge_core::domain::*` — `PlanningSolution`, `SolutionDescriptor`, entity and variable descriptors |
| `analysis` | `solverforge_scoring::api::analysis::*` — `ConstraintAnalysis`, `DetailedConstraintMatch`, score explanations |
| `constraint` | `solverforge_scoring::constraint::*` — the constraint implementations stream builders produce |
| `director` | `solverforge_scoring::director::*` — score directors, `DirectorScoreState`, weight modes |
| `builder` | the solver's model and search-declaration items — `RuntimeModel`, `Search`, `SearchContext`, `local_search` |
| `heuristic` | the solver's move, selector and cursor contracts and distance meters |
| `phase` | the solver's phase, acceptor, forager, exhaustive and partitioned-search items |
| `stats` | the solver's statistics, telemetry and candidate-trace types |

`scripts/check-facade-reachability.py` (`make facade-reachability`, CI Docs job) fails when a facade item's declaration or method signature links a type outside the facade's rustdoc.

### Model Macros (from `solverforge-macros`)

- `planning_model`
- `solverforge_constraints`
- `planning_entity`
- `planning_solution`
- `problem_fact`

### Score Types (from `solverforge-core`)

- `PlanningSolution` (trait), `ParseableScore` (trait), `ScoreLevel`, `ScoreParseError`

- `Score` (trait)
- `SoftScore`
- `HardSoftScore`
- `HardMediumSoftScore`
- `HardSoftDecimalScore`
- `BendableScore`

### Constraint Weights (from `solverforge-core`)

- `ConstraintWeights` — runtime per-constraint `u32` multipliers for a `#[constraint_weights]` solution field
- `ConstraintWeightError`
- `MAX_CONSTRAINT_WEIGHT_MULTIPLIER`

### Logical Descriptor IDs (from `solverforge-core::domain`)

- `EntityClassId`
- `ProblemFactClassId`
- `VariableId`

### Dynamic Bridge


### Constraint API (from `solverforge-scoring`)

- `fixed_weight`
- `hard_weight`
- `FixedWeight`
- `HardWeight`
- `ConstraintSet` (trait)
- `ConstraintMetadata<'a>` (borrowed constraint identity view), `ConstraintResult`
- `SharedNodeDiagnostics`, `SharedNodeId`, `SharedNodeOperation`
- `IncrementalConstraint` (trait)
- `SingleConstraint` (marker trait every hand-written `IncrementalConstraint` implements with an empty impl)
- `IncrementalUniConstraint`
- `ListPrecedenceMakespanConstraint`
- `Projection` (trait)
- `ProjectionSink` (trait)

### Score Director (from `solverforge-scoring`)

- `Director` (trait)
- `ScoreDirector`

### Constraint Identity (from `solverforge-core`)

- `ConstraintRef` (returned by `IncrementalConstraint::constraint_ref`)
- `ImpactType`

### Configuration (from `solverforge-config`)

The root keeps these; `solverforge::config` has every configuration type:

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

### Custom Phase Scope (from `solverforge-solver::scope`)

- `SolverScope` — the scope a `CustomSearchPhase` receives. Seeded draws go through `random_below(upper)` and `random_u64()`; the raw generator is not public.
- `ProgressCallback` (trait), `SolverProgressRef`, `SolverProgressKind`, `PhaseScope`, `StepScope`

### Solver (from `solverforge-solver`)

- `analyze` (free function)
- `Solvable` (trait)
- `RuntimeBuildError`, `RuntimeBuildResult` (the `Solvable::solve` error type)
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
- `SolverManager` (including `add_problem_change` for real-time changes)
- `ProblemChange` (real-time change trait; `apply(self: Box<Self>, &mut dyn Director<S>)`)
- `ClosureProblemChange` (one-shot closure change)
- `ProblemChangeId` (queued-change acknowledgement; applied once `telemetry.problem_changes_applied >= id.get()`)
- `SolverRuntime`
- `SolverEvent`
- `SolverEventMetadata`
- `SolverEventReceiver` (per-job event stream returned by `SolverManager::solve`: `recv().await`, `blocking_recv()`, `try_recv()`)
- `SolverEventTryRecvError` (`Empty` or `Disconnected`, from `SolverEventReceiver::try_recv`)
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

- `solverforge::console::init()` — starts console output (the SolverForge banner and solver lifecycle and progress events through a `tracing` layer). It is the only `solverforge-console` item the facade exposes, so the rest of that crate stays outside the facade's API.

## `prelude` Module

Convenient single import for user code:

```rust
pub use crate::local_search;
pub use crate::planning::EntitySourceTargetExt;
pub use crate::stream::collector::{
    collect_vec, consecutive_runs, count, indexed_presence, load_balance, sum, CollectedVec,
    IndexedPresence, Run, Runs,
};
pub use crate::stream::{joiner, ConstraintFactory};
pub use crate::{
    fixed_weight, hard_weight, planning_entity, planning_model, planning_solution, problem_fact,
    solverforge_constraints, BendableScore, ConflictRepair, ConstraintMetadata, ConstraintRef,
    ConstraintSet, ConstraintWeights, CustomSearchPhase, Director, ExhaustiveSearchConfig,
    ExhaustiveSearchPhase, ExplorationType, FixedWeight, FunctionalPartitioner,
    HardMediumSoftScore, HardSoftDecimalScore, HardSoftScore, HardWeight, ImpactType,
    IncrementalConstraint, ListPrecedenceMakespanConstraint, PartitionedSearchPhase, Projection,
    ProjectionSink, RepairCandidate, RepairLimits, ScalarAssignmentRule, ScalarCandidate,
    ScalarEdit, ScalarGroup, ScalarGroupLimits, ScalarTarget, Score, ScoreDirector, Search,
    SearchContext, SharedNodeDiagnostics, SharedNodeId, SharedNodeOperation, SimpleDecider,
    SingleConstraint, SoftScore, SolutionPartitioner, ThreadCount,
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

Re-exports the whole fluent constraint stream API of `solverforge-scoring`:

```rust
pub use solverforge_scoring::stream::collection_extract::vec;
pub use solverforge_scoring::stream::*;
```

`stream::cross` holds the cross-join `Bi`/`Tri`/`Quad`/`Penta` streams, `stream::projected` the projected streams and `Source`, `stream::chain` the arity-free `Chain` that `cross::Penta::join` returns, and `stream::filter` the filter traits and adapters the stream types are generic over.

Key stream API: `ConstraintFactory::new().for_each(extractor).filter(pred).penalize(weight).named("name")`. Use `.join(target)` for all join patterns: self-join, keyed cross-join, and predicate cross-join. Keyed and predicate cross-joins can take a right-hand `UniConstraintStream`, including one with its own `.filter(...)`. Keyed cross joins can either score pairs directly, group joined pairs with `.group_by(|left, right| key, collector)`, complement those grouped pairs with `.complement(source, key, default)`, or project joined pairs into rows with `.project(|left, right| row)`. A keyed cross Bi stream extends to a third source with `.join((extractor_c, key_c))`, yielding `stream::cross::Tri` rows `(A, B, C)` where `key_a(a) == key_b(b) == key_c(c)`; the bi stream's own keys stay authoritative for A and B. `#[planning_solution]` also generates a solution-named convenience trait, such as `PlanConstraintStreams`, so callers can import the trait and write `ConstraintFactory::new().assignments()` for the same concrete source stream.

For the first unary keyed equality cross-join, filters on unary sources exclude
rows before key extraction and pair creation while preserving original source
indexes and descriptor-local change metadata. Filters after the join operate on
joined rows. The left-source pushdown is not provided by comparison, overlap,
composed-condition, predicate, or same-source self-join dispatch.

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

Extractor ergonomics: Tri/Quad/Penta cross streams and same-source arity streams
also support `.group_by(key, collector)`. Key closures receive three/four/five
entity references; collector mappers receive borrowed tuples, and scoring
weights receive `(&group_key, &collector_result)`. These methods retain upstream
filters and incremental retraction semantics through the common grouping operator.

Extractor ergonomics: `for_each` accepts `CollectionExtract<S, Item = A>`. Stream sources and raw keyed join targets are planning-model collections: the source methods `#[planning_solution]` generates, or a `UniConstraintStream` over one. A raw closure, `fn` item or `Vec` is not a `CollectionExtract` and fails to compile. Passing a right-hand `UniConstraintStream` remains the preferred form when the source needs its own `.filter(...)`.

Model-owned keyed joins use solution source methods generated by `#[planning_solution]`, preserving hidden descriptor/static metadata:
```rust
ConstraintFactory::<Plan, HardSoftScore>::new()
    .for_each(Plan::assignments())
    .join((
        Plan::furnaces(),
        equal_bi(|assignment| assignment.furnace_idx(), |furnace| Some(furnace.id)),
    ))
```

Generated existence ergonomics: `#[planning_solution]` generates inherent source methods such as `Plan::assignments()` with hidden descriptor/static metadata, plus the matching `PlanConstraintStreams` convenience trait. Localized incremental callbacks use entity indexes only for the owning planning-entity collection. Raw facade `for_each(...)` extractors do not carry localized source ownership. Flattened existence targets use `.flattened(...)` and `FlattenedCollectionTarget`.

Projected scoring ergonomics: `ConstraintFactory::new().for_each(Plan::assignments()).project(entries)`, where `entries` is a value of a named `Projection<A>` type, creates bounded scoring rows from a named `Projection<A>` type without materializing facts or entities. Keyed cross joins can group joined pairs directly with `.group_by(|left, right| key, collector)` and complement the grouped result, or project them with `.project(|assignment, capacity| Row { ... })` to emit one scoring row per retained joined pair. Projected streams can be filtered, self-joined, merged, grouped, complemented after grouping, and weighted as retained scoring state. Single-source projection implementations emit through `ProjectionSink` and declare `MAX_EMITS`; joined-pair closures do not need a helper type. Projected output rows, projected self-join keys, and grouped collector values do not need `Clone`. Projected self-join ordering is coordinate-stable by `RowCoordinate`; directed projected self-joins use `.join(joiner::equal_bi(left_key, right_key))`; low-level pair-filter indexes are primary owner entity indexes, not sparse storage row IDs.

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

Exactly the items generated code names, and nothing else:

**Derive support (from `solverforge-macros`):** `PlanningEntityImpl`,
`PlanningSolutionImpl`, `ProblemFactImpl`. Attribute macros route their
support derives through `__internal`; users should not import them directly.

**Domain types (from `solverforge-core::domain`):** `EntityCollectionExtractor`,
`EntityDescriptor`, `PlanningEntity`, `PlanningId`, `PlanningSolution`,
`ProblemFact`, `ProblemFactDescriptor`, `ShadowVariableKind`,
`SolutionDescriptor`, `ValueRangeType`, `VariableDescriptor`.

**Scoring (from `solverforge-scoring`):** `ConstraintSet`, `ConstraintSetChain`,
`ConstraintSetSource`, `OrderedConstraintSetChain`, `Director`, `ScoreDirector`,
`SolvableSolution`. Node sharing is finalized through the same fluent
`.penalize(...).named(...)` / `.reward(...).named(...)` path as ordinary
constraints; internal shared node-state types are not facade exports.

**Solver (from `solverforge-solver`):** `bind_scalar_groups`,
`CustomSearchPhase`, `DefaultCrossEntityDistanceMeter`, `ListVariableEntity`,
`ListVariableMetadata`, `ListVariableSlot`, `log_solve_start`,
`PlanningModelSupport`, `ProgressCallback`, `RuntimeBuildResult`,
`RuntimeModel`, `ScalarGroupBinding`, `ScalarVariableSlot`, `Search`,
`SearchContext`, `SolverScope`, `try_run_solver_with_config_and_search`,
`try_run_weighted_solver_with_config_and_search`, `ValueSource`,
`VariableSlot`.

**Config (from `solverforge-config`):** `SolverConfig`.

**Stream types for macro-generated source methods (from `solverforge-scoring`):**
`ChangeSource`, `source`, `SourceExtract`, `TrueFilter`, `UnassignedEntity`,
`UniConstraintStream`.

### Functions

| Function | Signature | Note |
|----------|-----------|------|
| `init_console` | `fn()` | No-op unless `console` feature enabled |

## Architectural Notes

- **Facade crate.** Public modules are re-exports; implementation is limited to
  tiny hidden macro helpers such as `init_console()`.
- **`__internal` module** exists so that macro-generated code can reference types via `::solverforge::__internal::*` paths. This allows derive macros in `solverforge-macros` to generate code that compiles in user crates that only depend on `solverforge`.
- **Shape-aware startup telemetry.** Hidden runtime logging helpers under `__internal` emit `element_count` for list solves and average `candidate_count` for scalar solves so console startup output can label the scale correctly.
- **Macro-built runtime slots stay model-owned.** `planning_model!` generates
  the hidden `PlanningModelSupport` impl that attaches nearby hooks plus scalar
  construction order-key hooks from `#[planning_variable]` `Option<usize>`
  fields, and attaches list construction element-order and owner hooks from
  `#[planning_list_variable]`. Construction order-key hooks are
  construction-only and do not reorder local-search scalar candidate or list
  neighborhoods.
- **Entity pinning.** `#[planning_pin]` preserves a planning entity's input
  genuine scalar or list variables during solver-generated construction and
  search; a pinned required scalar must already be assigned.
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
- **Feature flags** propagate to sub-crates: `serde` → `solverforge-core/serde`, `console` → `solverforge-console`, `verbose-logging` → `console` + `solverforge-console/verbose-logging`. `HardSoftDecimalScore` needs no feature.
