# solverforge-solver WIREFRAME

Solver engine: phases, moves, selectors, acceptors, foragers, termination, and solver management.

**Location:** `crates/solverforge-solver/`
**Workspace Release:** `0.14.0`

## Dependencies

- `solverforge-core` (path) — Score types, domain traits, descriptors
- `solverforge-scoring` (path) — Director trait, constraint scoring
- `solverforge-config` (path) — SolverConfig, PhaseConfig, AcceptorConfig
- `thiserror` (workspace) — Error derivation
- `tracing` (workspace) — Logging
- `rand` / `rand_chacha` (workspace) — RNG
- `rayon` (workspace) — Parallel computation
- `smallvec` (workspace) — Stack-allocated small vectors
- `serde` (workspace) — Serialization
- `tokio` (sync feature) — `mpsc` channels for solution streaming

## File Map

```
src/
├── lib.rs                               — Crate root; module declarations, re-exports
├── solver.rs                            — Solver struct, SolveResult, impl_solver! macro
├── runtime.rs                           — Runtime assembly and target matching over `RuntimeModel`; routes scalar-only construction through the descriptor boundary, routes named grouped scalar construction through the atomic grouped-scalar builder, uses capability-validated routing for scalar/list/mixed construction, and delegates specialized list phases
├── model_support.rs                     — Hidden `PlanningModelSupport` bridge implemented by `planning_model!` for model-owned scalar hook attachment, scalar group attachment, model validation, and shadow updates
├── runtime/
│   ├── tests.rs                         — Runtime construction routing and target-validation tests
│   ├── tests/*.rs                       — Runtime test fixtures split by scalar, queue, revision, multi-owner, mixed-target, grouped-scalar, and scalar-assignment behavior
│   └── list_tests.rs                    — Specialized list-construction runtime tests
├── descriptor.rs                        — Re-exports descriptor bindings, selectors, move types, and internal construction/runtime helpers
├── descriptor/
│   ├── bindings.rs                      — Scalar-variable binding discovery, nearby hooks, scalar construction order keys, and frontier-aware work checks
│   ├── move_types.rs                    — DescriptorChangeMove<S>, DescriptorSwapMove<S>, DescriptorPillarChangeMove<S>, DescriptorPillarSwapMove<S>, DescriptorRuinRecreateMove<S>, DescriptorMoveUnion<S>
│   ├── move_types/*.rs                  — Descriptor move implementations split by move family
│   ├── selectors.rs                     — DescriptorChangeMoveSelector<S>, DescriptorSwapMoveSelector<S>, DescriptorLeafSelector<S>, DescriptorFlatSelector<S>, DescriptorSelectorNode<S>, DescriptorSelector<S>, build_descriptor_move_selector(config, descriptor, random_seed); nearby scalar selectors require descriptor-provided nearby candidate hooks, optional meters rank/filter those candidates, optional assigned variables can emit one `Some(v) -> None` change, top-level cartesian selectors expose borrowable sequential candidates, and scalar ruin-recreate uses the configured seed when provided
│   ├── selectors/*.rs                   — Descriptor selector legality, leaf, dispatch, and builder chunks split by neighborhood family
│   ├── construction.rs                  — DescriptorConstruction<S>, DescriptorEntityPlacer<S>; runtime-only descriptor construction assembly from resolved scalar bindings with optional keep-current legality and slot identity
│   └── tests.rs                         — Descriptor test root with support/construction/selector/ruin-recreate chunks under `tests/mod/`
├── run.rs                               — AnyTermination, build_termination, run_solver(), run_solver_with_config()
├── run_tests.rs                         — Tests
├── builder/
│   ├── mod.rs                           — Re-exports from all builder submodules
│   ├── acceptor.rs                      — AnyAcceptor<S> enum, AcceptorBuilder
│   ├── acceptor/tests.rs                — Tests
│   ├── forager.rs                       — AnyForager<S> enum, ForagerBuilder
│   ├── context.rs                       — RuntimeModel<S, V, DM, IDM>, VariableSlot<S, V, DM, IDM>, IntraDistanceAdapter<T>, index-addressed scalar slots, internal ScalarGroupBinding<S>, scalar assignment metadata, expanded scalar/list construction capability hooks
│   ├── context/*.rs                     — Model, list, conflict-repair, and scalar slot implementation chunks
│   ├── context/scalar/mod.rs            — Scalar slot module root and internal re-exports
│   ├── context/scalar/*.rs              — Scalar value-source, scalar variable-slot, and grouped scalar binding definitions
│   ├── scalar_selector.rs               — Canonical scalar selector assembly over index-addressed scalar slots, nearby scalar leaves, pillar legality filtering, ruin-recreate, and cartesian composition
│   ├── scalar_selector/*.rs             — Scalar value, leaf, dispatch, and build chunks
│   ├── scalar_selector/tests.rs         — Scalar selector test root with change/swap, nearby/ruin, pillar, and cartesian chunks
│   ├── search.rs                        — Typed custom-search surface: SearchContext, Search, CustomSearchPhase, local_search(), and typed custom phase registration
│   ├── search/*.rs                      — Model-aware default search profile and recursive typed custom phase registry
│   ├── selector.rs                      — Selector<S, V, DM, IDM>, Neighborhood<S, V, DM, IDM>, build_move_selector() over published RuntimeModel variable slots
│   ├── selector/*.rs                    — Mixed scalar/list neighborhood move, grouped scalar selector, conflict repair selector, family classification, and explicit selector builder chunks
│   ├── selector/types/*.rs              — Neighborhood leaf, composite, cursor, and union types used by selector assembly
│   ├── selector/tests.rs                — Mixed selector test root with support, defaults, grouped scalar, cartesian, and phase chunks
│   ├── list_selector.rs                 — Re-exports list selector leaf and builder modules
│   └── list_selector/
│       ├── builder_impl.rs              — ListMoveSelectorBuilder
│       ├── builder_impl/*.rs            — List selector node/cursor and builder implementation chunks
│       ├── leaf.rs                      — ListLeafSelector<S, V, DM, IDM> enum
│       └── tests.rs                     — Tests
├── stats.rs                             — SolverStats, PhaseStats, SolverTelemetry, SelectorTelemetry
├── test_utils.rs                        — TestSolution, TestDirector, NQueens helpers
├── test_utils_tests.rs                  — Tests
│
├── heuristic/
│   ├── mod.rs                           — Re-exports from move/ and selector/
│   │
│   ├── move/
│   │   ├── mod.rs                       — Module declarations, re-exports
│   │   ├── traits.rs                    — Move<S> trait definition
│   │   ├── arena.rs                     — MoveArena<M> allocator
│   │   ├── change.rs                    — ChangeMove<S, V>
│   │   ├── swap.rs                      — SwapMove<S, V>
│   │   ├── list_change.rs              — ListChangeMove<S, V>
│   │   ├── list_swap.rs                — ListSwapMove<S, V>
│   │   ├── list_reverse.rs             — ListReverseMove<S, V>
│   │   ├── list_ruin.rs                — ListRuinMove<S, V>
│   │   ├── metadata.rs                 — MoveTabuSignature, scoped entity/value tabu tokens, exact move identities
│   │   ├── sublist_change.rs           — SublistChangeMove<S, V>
│   │   ├── sublist_swap.rs             — SublistSwapMove<S, V>
│   │   ├── segment_layout.rs           — Post-move segment coordinate derivation and reverse-identity helpers for sublist moves
│   │   ├── pillar_change.rs            — PillarChangeMove<S, V>
│   │   ├── pillar_swap.rs              — PillarSwapMove<S, V>
│   │   ├── ruin.rs                      — RuinMove<S, V>
│   │   ├── ruin_recreate.rs             — RuinRecreateMove<S> and ScalarRecreateValueSource<S>
│   │   ├── k_opt.rs                     — KOptMove<S, V>, CutPoint
│   │   ├── k_opt_reconnection.rs       — KOptReconnection patterns
│   │   ├── k_opt_reconnection_tests.rs — Tests
│   │   ├── compound_scalar.rs          — CompoundScalarMove<S> for atomic multi-scalar edits with exact undo, tabu identity, and affected-entity reporting
│   │   ├── composite.rs                — CompositeMove<S, M1, M2>, SequentialCompositeMove<S, M>
│   │   ├── scalar_union.rs             — ScalarMoveUnion<S, V> enum
│   │   ├── list_union.rs               — ListMoveUnion<S, V> enum
│   │   └── tests/                       — Additional test modules
│   │       ├── mod.rs
│   │       ├── arena.rs
│   │       ├── change.rs
│   │       ├── swap.rs
│   │       ├── compound_scalar.rs
│   │       ├── conflict_repair.rs
│   │       ├── list_change.rs
│   │       ├── list_swap.rs
│   │       ├── list_reverse.rs
│   │       ├── list_ruin.rs
│   │       ├── pillar_change.rs
│   │       ├── pillar_swap.rs
│   │       ├── ruin.rs
│   │       ├── sublist_change.rs
│   │       ├── sublist_swap.rs
│   │       └── k_opt.rs
│   │
│   └── selector/
│       ├── mod.rs                       — Re-exports
│       ├── entity.rs                    — EntitySelector trait, FromSolutionEntitySelector, AllEntitiesSelector
│       ├── value_selector.rs              — ValueSelector trait, StaticValueSelector, FromSolutionValueSelector
│       ├── move_selector.rs             — MoveSelector trait, MoveCursor, MoveCandidateRef, ChangeMoveSelector, SwapMoveSelector, scalar union helpers; `ChangeMoveSelector::with_allows_unassigned()` enables `Some(v) -> None` generation for assigned optional variables
│       ├── move_selector/*.rs           — Borrowed candidate cursor, iterator adapter, change selector, and swap selector implementation chunks
│       ├── move_selector/scalar_union.rs — ScalarChangeMoveSelector, ScalarSwapMoveSelector
│       ├── list_change.rs              — ListChangeMoveSelector<S, V, ES>
│       ├── list_support.rs             — Private selected-entity snapshots and exact list-neighborhood counting
│       ├── list_swap.rs                — ListSwapMoveSelector<S, V, ES>
│       ├── list_reverse.rs             — ListReverseMoveSelector<S, V, ES>
│       ├── list_ruin.rs                — ListRuinMoveSelector<S, V>
│       ├── sublist_change.rs           — SublistChangeMoveSelector<S, V, ES>
│       ├── sublist_support.rs          — Private sublist segment enumeration and exact counting helpers
│       ├── sublist_swap.rs             — SublistSwapMoveSelector<S, V, ES>
│       ├── pillar.rs                    — PillarSelector trait, DefaultPillarSelector, Pillar, SubPillarConfig
│       ├── pillar_support.rs            — Deterministic pillar grouping, legal-domain intersection, and mutual swap-compatibility helpers
│       ├── ruin.rs                      — RuinMoveSelector<S, V>, RuinVariableAccess<S, V>
│       ├── seed.rs                      — Scoped deterministic selector seed derivation from SolverConfig random_seed
│       ├── mimic.rs                     — MimicRecorder, MimicRecordingEntitySelector, MimicReplayingEntitySelector
│       ├── selection_order.rs          — SelectionOrder enum
│       ├── selection_order_tests.rs    — Tests
│       ├── entity_tests.rs              — Tests
│       ├── value_selector_tests.rs     — Tests
│       ├── nearby.rs                    — NearbyDistanceMeter trait, DynDistanceMeter, NearbyEntitySelector, NearbySelectionConfig
│       ├── nearby_list_change.rs       — CrossEntityDistanceMeter trait, NearbyListChangeMoveSelector
│       ├── nearby_list_support.rs      — Private selected-entity snapshots and nearby candidate ordering
│       ├── nearby_list_swap.rs         — NearbyListSwapMoveSelector
│       ├── nearby_support.rs           — Shared nearest-candidate ordering helpers for bounded nearby neighborhoods
│       ├── decorator/
│       │   ├── mod.rs                   — Re-exports
│       │   ├── cartesian_product.rs    — CartesianProductArena<S, M1, M2>, CartesianProductCursor<S, M>, CartesianProductSelector<S, M, Left, Right>
│       │   ├── cartesian_product/tests.rs — Tests
│       │   ├── filtering.rs            — FilteringMoveSelector<S, M, Inner>
│       │   ├── filtering/tests.rs      — Tests
│       │   ├── indexed_cursor.rs       — Shared indexed cursor adapter
│       │   ├── limited.rs              — Candidate-limit cursor decorator
│       │   ├── mapped_cursor.rs        — Shared mapped cursor adapter
│       │   ├── probability.rs          — ProbabilityMoveSelector<S, M, Inner>
│       │   ├── probability/tests.rs    — Tests
│       │   ├── shuffling.rs            — ShufflingMoveSelector<S, M, Inner>
│       │   ├── shuffling/tests.rs      — Tests
│       │   ├── sorting.rs              — SortingMoveSelector<S, M, Inner>
│       │   ├── sorting/tests.rs        — Tests
│       │   ├── union.rs                — UnionMoveSelector<S, M, A, B>
│       │   ├── union/tests.rs          — Tests
│       │   ├── vec_union.rs            — VecUnionSelector<S, M, Leaf> (Vec-backed union for config-driven composition)
│       │   └── test_utils.rs           — Test helpers
│       ├── k_opt/
│       │   ├── mod.rs                   — Re-exports
│       │   ├── config.rs               — KOptConfig
│       │   ├── cuts.rs                 — CutCombinationIterator (pub(crate))
│       │   ├── iterators.rs            — CutCombinationIterator (pub), binomial(), count_cut_combinations()
│       │   ├── distance_meter.rs       — ListPositionDistanceMeter trait, DefaultDistanceMeter
│       │   ├── distance.rs             — (duplicate of distance_meter.rs)
│       │   ├── nearby.rs               — NearbyKOptMoveSelector<S, V, D, ES>
│       │   ├── selector.rs             — KOptMoveSelector<S, V, ES>
│       │   └── tests.rs                — Tests
│       └── tests/
│           ├── mod.rs
│           ├── k_opt.rs
│           ├── list_neighborhood.rs
│           ├── list_ruin.rs
│           ├── mimic.rs
│           ├── nearby.rs
│           ├── nearby_list.rs
│           ├── pillar.rs
│           ├── sublist_neighborhood.rs
│           └── move_selector.rs
│
├── phase/
│   ├── mod.rs                           — Phase<S, D> trait, tuple impls
│   ├── control.rs                       — Internal prompt/control settlement helpers for runtime-owned pause and cancellation boundaries
│   ├── hard_delta.rs                    — Internal hard-score delta classification shared by local search and VND gates
│   ├── hard_delta_tests.rs              — Hard-score delta tests
│   ├── hard_delta_phase_tests.rs        — Phase-level hard-improvement gate tests
│   ├── construction/
│   │   ├── mod.rs                       — Construction module declarations and re-exports
│   │   ├── config.rs                    — ForagerType enum and ConstructionHeuristicConfig
│   │   ├── decision.rs                  — Shared baseline/tie-breaking helpers for construction choice resolution
│   │   ├── evaluation.rs                — Trial-move evaluation via typed move undo and score-state snapshots
│   │   ├── frontier.rs                  — Revision-scoped ConstructionFrontier shared by generic scalar and list work
│   │   ├── phase.rs                     — ConstructionHeuristicPhase<S, M, P, Fo>
│   │   ├── phase/*.rs                   — Construction phase type and selection helpers
│   │   ├── phase/tests.rs               — Construction phase test root with support, selection, and lifecycle chunks
│   │   ├── forager.rs                   — ConstructionChoice enum, ConstructionForager trait, FirstFit/BestFit/FirstFeasible/WeakestFit/StrongestFit forager types
│   │   ├── forager_impl.rs              — Stock construction forager strategy implementations
│   │   ├── forager_step.rs              — Step-aware stock construction selection with telemetry and prompt/control polling
│   │   ├── forager/tests.rs             — Tests
│   │   ├── placer.rs                    — EntityPlacer trait, Placement, QueuedEntityPlacer, SortedEntityPlacer; queued placements expose optional keep-current legality
│   │   ├── placer/tests.rs              — Tests
│   │   ├── slot.rs                      — ConstructionSlotId, exact-keyed ConstructionGroupSlotId, ConstructionGroupSlotKey, and ConstructionListElementId for construction frontier tracking
│   │   ├── capabilities.rs              — Shared heuristic-to-capability routing and early validation for scalar/list/grouped-scalar construction
│   │   ├── grouped_scalar/mod.rs        — Atomic grouped scalar construction module root over declared ScalarGroup candidates and assignment groups bound to runtime scalar slots
│   │   ├── grouped_scalar/assignment_candidate.rs — Assignment move options, required assignment moves, capacity-conflict moves, reassignment moves, and remaining-required telemetry
│   │   ├── grouped_scalar/assignment_cycle.rs — Bounded augmenting cycle and ejection/reinsert move construction
│   │   ├── grouped_scalar/assignment_edge.rs — Small value objects for assignment-rule sequence-edge checks
│   │   ├── grouped_scalar/assignment_entity.rs — Entity-local required, optional, capacity, and reassignment move construction
│   │   ├── grouped_scalar/assignment_family.rs — Shared assignment candidate-family deduplication
│   │   ├── grouped_scalar/assignment_index.rs — Indexed assignment-state map helpers
│   │   ├── grouped_scalar/assignment_pair.rs — Deterministic bounded pair, rematch, and sequence-window move generation
│   │   ├── grouped_scalar/assignment_path.rs — Bounded augmenting-path move construction for required and optional scalar assignments
│   │   ├── grouped_scalar/assignment_state.rs — Assignment occupancy, capacity, rollback, and conflict bookkeeping
│   │   ├── grouped_scalar/assignment_stream.rs — Cursor-backed assignment move streaming for construction and selectors
│   │   ├── grouped_scalar/move_build.rs — CompoundScalarMove construction from public ScalarCandidate edits
│   │   ├── grouped_scalar/phase.rs      — ScalarGroupConstruction builder that feeds grouped scalar placements into stock ConstructionHeuristicPhase
│   │   ├── grouped_scalar/placer.rs     — ScalarGroupPlacer adapter that emits stock Placement<CompoundScalarMove> values for provider and assignment groups
│   │   ├── grouped_scalar/placer_stream.rs — Concrete candidate and assignment placement stream helpers
│   │   ├── engine.rs                    — Canonical generic scalar/list/mixed construction engine used by runtime assembly
│   │   └── engine/*.rs                  — Generic construction candidate, scan, commit, and target-matching chunks
│   ├── localsearch/
│   │   ├── mod.rs                       — LocalSearchConfig, AcceptorType, re-exports
│   │   ├── phase.rs                     — LocalSearchPhase<S, M, MS, A, Fo>
│   │   ├── phase/tests.rs               — Tests
│   │   ├── forager.rs                   — LocalSearchForager trait, AcceptedCountForager, FirstAcceptedForager, BestScoreForager, re-exports
│   │   ├── forager/improving.rs        — FirstBestScoreImprovingForager, FirstLastStepScoreImprovingForager
│   │   ├── forager/tests.rs             — Tests
│   │   └── acceptor/
│   │       ├── mod.rs                   — Acceptor module declarations and re-exports
│   │       ├── traits.rs                — Acceptor<S> trait
│   │       ├── hill_climbing.rs        — HillClimbingAcceptor
│   │       ├── late_acceptance.rs      — LateAcceptanceAcceptor<S>
│   │       ├── simulated_annealing.rs  — SimulatedAnnealingAcceptor
│   │       ├── simulated_annealing/tests.rs — Tests
│   │       ├── tabu_search.rs          — TabuSearchAcceptor<S>
│   │       ├── entity_tabu.rs          — EntityTabuAcceptor
│   │       ├── value_tabu.rs           — ValueTabuAcceptor
│   │       ├── value_tabu/tests.rs     — Tests
│   │       ├── move_tabu.rs            — MoveTabuAcceptor
│   │       ├── move_tabu/tests.rs      — Tests
│   │       ├── great_deluge.rs         — GreatDelugeAcceptor<S>
│   │       ├── great_deluge/tests.rs   — Tests
│   │       ├── step_counting.rs        — StepCountingHillClimbingAcceptor<S>
│   │       ├── step_counting/tests.rs  — Tests
│   │       ├── diversified_late_acceptance.rs — DiversifiedLateAcceptanceAcceptor<S>
│   │       ├── diversified_late_acceptance/tests.rs — Tests
│   │       └── tests.rs                — Tests
│   ├── exhaustive/
│   │   ├── mod.rs                       — ExhaustiveSearchPhase, ExhaustiveSearchConfig, ExplorationType
│   │   ├── bounder.rs                   — ScoreBounder trait, SoftScoreBounder, FixedOffsetBounder
│   │   ├── bounder_tests.rs             — Tests
│   │   ├── decider.rs                   — ExhaustiveSearchDecider trait and SimpleDecider
│   │   ├── decider_tests.rs             — Tests
│   │   ├── node.rs                      — ExhaustiveSearchNode
│   │   ├── node_tests.rs                — Tests
│   │   ├── phase.rs                     — ExhaustiveSearchPhase<Dec>
│   │   ├── phase_tests.rs               — Tests
│   │   └── priority_node.rs             — PriorityNode<S>
│   ├── partitioned/
│   │   ├── mod.rs                       — Partitioned module declarations and re-exports
│   │   ├── child_phases.rs              — ChildPhases trait and tuple impls
│   │   ├── config.rs                    — PartitionedSearchConfig
│   │   ├── partitioner.rs              — SolutionPartitioner trait, FunctionalPartitioner, ThreadCount
│   │   ├── partitioner_tests.rs        — Tests
│   │   ├── phase.rs                    — PartitionedSearchPhase<P, Part>
│   │   └── phase_tests.rs              — Tests
│   ├── sequence.rs                      — PhaseSequence<P>
│   └── localsearch/vnd/
│       ├── mod.rs                       — Internal VND module declarations
│       ├── phase.rs                     — Internal Vec-backed VndPhase used by LocalSearchStrategy
│       ├── telemetry.rs                 — Internal VND progress and selector-label helpers
│       └── tests.rs                     — Internal VND tests
│
├── manager/
│   ├── mod.rs                           — PhaseFactory trait, re-exports
│   ├── config.rs                        — LocalSearchAcceptorType, ConstructionType, PhaseConfig enums
│   ├── builder.rs                       — SolverFactoryBuilder, SolverBuildError
│   ├── solver_factory.rs               — SolverFactory, solver_factory_builder() free fn
│   ├── solver_manager.rs               — Re-exports retained lifecycle manager surface
│   ├── solver_manager/types.rs         — SolverLifecycleState, SolverTerminalReason, SolverStatus, SolverEvent, SolverSnapshot, SolverManagerError
│   ├── solver_manager/runtime.rs       — SolverRuntime retained lifecycle publisher
│   ├── solver_manager/slot.rs          — Internal retained-job slots and snapshot records
│   ├── solver_manager/manager.rs       — MAX_JOBS, Solvable trait, SolverManager
│   ├── solution_manager.rs             — analyze() free fn, Analyzable trait, ScoreAnalysis, ConstraintAnalysis
│   ├── phase_factory/
│   │   ├── mod.rs                       — Re-exports
│   │   ├── construction.rs             — ConstructionPhaseFactory
│   │   ├── list_construction.rs        — Re-exports
│   │   ├── list_construction/round_robin.rs — ListConstructionPhaseBuilder, ListConstructionPhase
│   │   ├── list_construction/state.rs  — Shared scored insertion state
│   │   ├── list_construction/cheapest.rs — ListCheapestInsertionPhase
│   │   ├── list_construction/regret.rs — ListRegretInsertionPhase
│   │   ├── list_clarke_wright.rs       — ListClarkeWrightPhase
│   │   ├── list_clarke_wright/tests.rs — Tests
│   │   ├── list_k_opt.rs               — ListKOptPhase
│   │   ├── local_search.rs             — LocalSearchPhaseFactory
│   │   └── k_opt.rs                     — KOptPhaseBuilder, KOptPhase
│   ├── builder_tests.rs                — Tests
│   ├── mod_tests.rs                    — Tests
│   ├── mod_tests_integration.rs        — Integration test module declarations
│   ├── mod_tests_integration/basic.rs  — Builder/factory integration tests
│   ├── mod_tests_integration/common.rs — Shared no-op phase fixture
│   ├── mod_tests_integration/gates.rs  — Shared retained-job gates and blockers
│   ├── mod_tests_integration/lifecycle_solutions.rs — Retained lifecycle fixtures
│   ├── mod_tests_integration/lifecycle_tests.rs — Retained lifecycle tests
│   ├── mod_tests_integration/prompt_support.rs — Prompt-settlement fixtures
│   ├── mod_tests_integration/prompt_tests.rs — Prompt-settlement tests
│   ├── mod_tests_integration/resume_support.rs — Resume and snapshot fixtures
│   ├── mod_tests_integration/resume_tests.rs — Resume determinism tests
│   ├── mod_tests_integration/analysis_tests.rs — Snapshot analysis retention tests
│   └── mod_tests_integration/runtime_helpers.rs — Shared telemetry helpers
│
├── scope/
│   ├── mod.rs                           — Re-exports
│   ├── solver.rs                        — SolverScope<'t, S, D, ProgressCb = ()>, ProgressCallback trait, lifecycle-aware SolveResult
│   ├── solver/*.rs                      — Solver progress callback and scope implementation chunks
│   ├── phase.rs                         — PhaseScope<'t, 'a, S, D, BestCb = ()>
│   ├── step.rs                          — StepScope<'t, 'a, 'b, S, D, BestCb = ()>
│   └── tests.rs                         — Tests
│
├── termination/
│   ├── mod.rs                           — Termination<S, D, BestCb = ()> trait, re-exports
│   ├── time.rs                          — TimeTermination
│   ├── step_count.rs                    — StepCountTermination
│   ├── best_score.rs                    — BestScoreTermination<Sc>, BestScoreFeasibleTermination<S, F>
│   ├── unimproved.rs                    — UnimprovedStepCountTermination<S>, UnimprovedTimeTermination<S>
│   ├── composite.rs                     — OrTermination<T, S, D>, AndTermination<T, S, D> (tuple impls up to 8)
│   ├── move_count.rs                    — MoveCountTermination<S>
│   ├── score_calculation_count.rs      — ScoreCalculationCountTermination<S>
│   ├── diminished_returns.rs           — DiminishedReturnsTermination<S>
│   ├── diminished_returns_tests.rs     — Tests
│   └── tests.rs                         — Tests
│
└── realtime/
    ├── mod.rs                           — Re-exports
    ├── problem_change.rs               — ProblemChange trait, BoxedProblemChange, ClosureProblemChange
    ├── problem_change_tests.rs         — Tests
    ├── solver_handle.rs                — SolverHandle<S>, ProblemChangeReceiver<S>, ProblemChangeResult
    └── solver_handle_tests.rs          — Tests
```

## Core Traits

### `Move<S: PlanningSolution>` — `traits.rs`

Requires: `Send + Sync + Debug`.

| Item | Signature |
|------|-----------|
| `type Undo` | `Send` |
| `is_doable` | `fn<D: Director<S>>(&self, score_director: &D) -> bool` |
| `do_move` | `fn<D: Director<S>>(&self, score_director: &mut D) -> Self::Undo` |
| `undo_move` | `fn<D: Director<S>>(&self, score_director: &mut D, undo: Self::Undo)` |
| `descriptor_index` | `fn(&self) -> usize` |
| `entity_indices` | `fn(&self) -> &[usize]` |
| `variable_name` | `fn(&self) -> &str` |
| `tabu_signature` | `fn<D: Director<S>>(&self, score_director: &D) -> MoveTabuSignature` |
| `for_each_affected_entity` | `fn(&self, visitor: &mut dyn FnMut(MoveAffectedEntity<'_>))` |

Moves are **never cloned**. Ownership transfers via `MoveArena` indices.
Move rollback is move-owned and typed: speculative evaluation snapshots the
director score state, calls `do_move`, scores the trial state, calls
`undo_move` with the returned `Self::Undo`, then restores the score-state
snapshot.

**`MoveAffectedEntity<'a>`** — `{ descriptor_index, entity_index, variable_name }`. Multi-edit moves report each edited descriptor/entity/variable pair directly instead of compressing metadata into a single descriptor scope.

### `MoveTabuSignature` and Scoped Tokens — `heuristic/move/metadata.rs`

- `MoveTabuScope { descriptor_index, variable_name }`
- `ScopedEntityTabuToken { scope, entity_id }`
- `ScopedValueTabuToken { scope, value_id }`
- `MoveTabuSignature { scope, entity_tokens, destination_value_tokens, move_id, undo_move_id }`

Entity and destination-value tabu memories compare scoped tokens directly, so equal raw ids from different descriptors or variables do not collide. Exact move memories still store ordered `move_id` and `undo_move_id` sequences without hashing away structure. Sequential composite moves use one shared selector-order composition rule for both fields, so cartesian reversals remain visible to move tabu and undo-move tabu. True self-inverse coordinate moves, such as scalar swaps, pillar swaps, list swaps, and list reversals, use canonical coordinate identities for both fields so default move-tabu blocks non-aspirational immediate reversals while value tabu remains value-sensitive through scoped destination-value tokens.

### `Phase<S: PlanningSolution, D: Director<S>, ProgressCb: ProgressCallback<S> = ()>` — `phase/mod.rs`

Requires: `Send + Debug`.

| Method | Signature |
|--------|-----------|
| `solve` | `fn(&mut self, solver_scope: &mut SolverScope<'_, S, D, ProgressCb>)` |
| `phase_type_name` | `fn(&self) -> &'static str` |

All concrete phase types implement `Phase<S, D, ProgressCb>` for all `ProgressCb: ProgressCallback<S>`. Tuple implementations via `tuple_impl.rs`.

### `Termination<S: PlanningSolution, D: Director<S>, ProgressCb: ProgressCallback<S> = ()>` — `termination/mod.rs`

Requires: `Send + Debug`.

| Method | Signature | Default |
|--------|-----------|---------|
| `is_terminated` | `fn(&self, solver_scope: &SolverScope<S, D, ProgressCb>) -> bool` | — |
| `install_inphase_limits` | `fn(&self, solver_scope: &mut SolverScope<S, D, ProgressCb>)` | no-op |

All concrete termination types implement `Termination<S, D, ProgressCb>` for all `ProgressCb: ProgressCallback<S>`.

### `Acceptor<S: PlanningSolution>` — `acceptor/mod.rs`

Requires: `Send + Debug`.

| Method | Signature | Default |
|--------|-----------|---------|
| `requires_move_signatures` | `fn(&self) -> bool` | `false` |
| `is_accepted` | `fn(&mut self, last_step_score: &S::Score, move_score: &S::Score, move_signature: Option<&MoveTabuSignature>) -> bool` | — |
| `phase_started` | `fn(&mut self, initial_score: &S::Score)` | no-op |
| `phase_ended` | `fn(&mut self)` | no-op |
| `step_started` | `fn(&mut self)` | no-op |
| `step_ended` | `fn(&mut self, step_score: &S::Score, accepted_move_signature: Option<&MoveTabuSignature>)` | no-op |

### `EntitySelector<S: PlanningSolution>` — `entity.rs`

| Method | Signature |
|--------|-----------|
| `iter` | `fn<'a, D: Director<S>>(&'a self, score_director: &'a D) -> impl Iterator<Item = EntityReference> + 'a` |
| `size` | `fn<D: Director<S>>(&self, score_director: &D) -> usize` |
| `is_never_ending` | `fn(&self) -> bool` |

### `MoveSelector<S: PlanningSolution, M: Move<S>>` — `move_selector.rs`

Selectors now expose cursor-owned storage plus borrowable candidates. The solver
evaluates candidates by reference and only takes ownership of the chosen move by
stable index. Convenience owned-stream helpers exist for arena-backed selectors,
but cartesian composition is intentionally cursor-native and selected-winner
materialization only. Cartesian remains two-child sequential composition over a
preview state; it is not the grouped atomic scalar-search primitive.

| Method | Signature |
|--------|-----------|
| `Cursor<'a>` | `type Cursor<'a>: MoveCursor<S, M> + 'a where Self: 'a` |
| `open_cursor` | `fn<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a>` |
| `open_cursor_with_context` | `fn<'a, D: Director<S>>(&'a self, score_director: &D, context: MoveStreamContext) -> Self::Cursor<'a>` |
| `iter_moves` | `fn<'a, D: Director<S>>(&'a self, score_director: &D) -> MoveSelectorIter<S, M, Self::Cursor<'a>>` |
| `size` | `fn<D: Director<S>>(&self, score_director: &D) -> usize` |
| `append_moves` | `fn<D: Director<S>>(&self, score_director: &D, arena: &mut MoveArena<M>)` |
| `is_never_ending` | `fn(&self) -> bool` |

`MoveStreamContext` is a small copy context passed by runtime streaming
phases. It carries `step_index`, `step_seed`, and the finite
`accepted_count_limit` when the forager has one. Canonical `open_cursor()`
uses the default context for deterministic explicit scans; local search and
explicit VND call `open_cursor_with_context()` so typed selectors can rotate
entity/value/child order without boxing cursors or erasing selector types.

`MoveCursor::selector_index()` reports the stable child index for telemetry
through union selectors. Cursor implementations store only discovered
candidates, and broad scalar/list selectors generate the next candidate from
cursor-native loop state instead of prebuilding full move vectors.

### `ValueSelector<S: PlanningSolution, V>` — `value_selector.rs`

| Method | Signature |
|--------|-----------|
| `iter` | `fn<'a, D: Director<S>>(&'a self, score_director: &'a D, descriptor_index: usize, entity_index: usize) -> impl Iterator<Item = V> + 'a` |
| `size` | `fn<D: Director<S>>(&self, score_director: &D, descriptor_index: usize, entity_index: usize) -> usize` |
| `is_never_ending` | `fn(&self) -> bool` |

### `PillarSelector<S: PlanningSolution>` — `pillar.rs`

| Method | Signature |
|--------|-----------|
| `iter` | `fn<'a, D: Director<S>>(&'a self, score_director: &'a D) -> impl Iterator<Item = Pillar> + 'a` |
| `size` | `fn<D: Director<S>>(&self, score_director: &D) -> usize` |
| `is_never_ending` | `fn(&self) -> bool` |
| `descriptor_index` | `fn(&self) -> usize` |

### `ConstructionForager<S, M>` — `construction/forager.rs`

Requires: `Send + Debug`. Bounds: `S: PlanningSolution, M: Move<S>`.

| Method | Signature |
|--------|-----------|
| `pick_move_index` | `fn<D: Director<S>>(&self, placement: &Placement<S, M>, score_director: &mut D) -> ConstructionChoice` |
| `select_move_index` | `fn<D, BestCb>(&self, placement: &Placement<S, M>, construction_obligation: ConstructionObligation, step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>) -> Option<ConstructionChoice>` |

`select_move_index` has a default implementation that delegates to
`pick_move_index`. Stock construction foragers override it so
`ConstructionHeuristicPhase` gets telemetry-aware evaluation polling through
static dispatch without runtime type inspection.

### `LocalSearchForager<S, M>` — `localsearch/forager.rs`

Requires: `Send + Debug`. Bounds: `S: PlanningSolution, M: Move<S>`.

| Method | Signature |
|--------|-----------|
| `step_started` | `fn(&mut self, best_score: S::Score, last_step_score: S::Score)` |
| `add_move_index` | `fn(&mut self, index: CandidateId, score: S::Score)` |
| `is_quit_early` | `fn(&self) -> bool` |
| `accepted_count_limit` | `fn(&self) -> Option<usize>` |
| `pick_move_index` | `fn(&mut self) -> Option<(CandidateId, S::Score)>` |

`AcceptedCountForager` is the default finite-horizon forager for broad stock
models. It means "select the best among the first N accepted moves", not
"scan the whole neighborhood and retain N". `BestScoreForager` remains
available for explicit full-neighborhood scans.

### `EntityPlacer<S, M>` — `construction/placer.rs`

Requires: `Send + Debug`. Bounds: `S: PlanningSolution, M: Move<S>`.

| Method | Signature |
|--------|-----------|
| `get_placements` | `fn<D: Director<S>>(&self, score_director: &D) -> Vec<Placement<S, M>>` |

### `ScoreBounder<S, D>` — `exhaustive/bounder.rs`

Requires: `Send + Debug`. Bounds: `S: PlanningSolution, D: Director<S>`.

| Method | Signature | Default |
|--------|-----------|---------|
| `calculate_optimistic_bound` | `fn(&self, score_director: &D) -> Option<S::Score>` | — |
| `calculate_pessimistic_bound` | `fn(&self, score_director: &D) -> Option<S::Score>` | `None` |

### `ExhaustiveSearchDecider<S, D>` — `exhaustive/decider.rs`

Requires: `Send + Debug`. Bounds: `S: PlanningSolution, D: Director<S>`.

| Method | Signature |
|--------|-----------|
| `expand` | `fn(&self, parent_index: usize, parent: &ExhaustiveSearchNode<S>, score_director: &mut D) -> Vec<ExhaustiveSearchNode<S>>` |
| `reset_assignments` | `fn(&self, score_director: &mut D)` |
| `apply_assignment` | `fn(&self, node: &ExhaustiveSearchNode<S>, score_director: &mut D)` |
| `total_entities` | `fn(&self, score_director: &D) -> usize` |

### `SolutionPartitioner<S>` — `partitioned/partitioner.rs`

Requires: `Send + Sync + Debug`. Bounds: `S: PlanningSolution`.

| Method | Signature | Default |
|--------|-----------|---------|
| `partition` | `fn(&self, solution: &S) -> Vec<S>` | — |
| `merge` | `fn(&self, original: &S, partitions: Vec<S>) -> S` | — |
| `recommended_partition_count` | `fn(&self) -> Option<usize>` | `None` |

### `NearbyDistanceMeter<Origin, Destination>` — `nearby.rs`

| Method | Signature |
|--------|-----------|
| `distance` | `fn(&self, origin: &Origin, destination: &Destination) -> f64` |

### `ListPositionDistanceMeter<S>` — `k_opt/distance_meter.rs`

Requires: `Send + Sync + Debug`.

| Method | Signature |
|--------|-----------|
| `distance` | `fn(&self, solution: &S, entity_idx: usize, pos_a: usize, pos_b: usize) -> f64` |

### `CrossEntityDistanceMeter<S>` — `nearby_list_change.rs`

| Method | Signature |
|--------|-----------|
| `distance` | `fn(&self, solution: &S, src_entity: usize, src_pos: usize, dst_entity: usize, dst_pos: usize) -> f64` |

### `PhaseFactory<S, D>` — `manager/mod.rs`

Requires: `Send + Sync`. Bounds: `S: PlanningSolution, D: Director<S>`.

| Associated Type | Bound |
|----------------|-------|
| `Phase` | `Phase<S, D>` |

| Method | Signature |
|--------|-----------|
| `create` | `fn(&self) -> Self::Phase` |

### `Solvable` — `manager/solver_manager.rs`

Requires: `PlanningSolution + Send + 'static`.

| Method | Signature |
|--------|-----------|
| `solve` | `fn(self, runtime: SolverRuntime<Self>)` |

### `SolverRuntime<S>` — `manager/solver_manager.rs`

Retained-job runtime context passed into `Solvable::solve()`. This is the public lifecycle emitter surface for manual downstream `Solvable` implementations.

| Method | Signature |
|--------|-----------|
| `job_id` | `fn(&self) -> usize` |
| `is_cancel_requested` | `fn(&self) -> bool` |
| `emit_progress` | `fn(&self, current_score: Option<S::Score>, best_score: Option<S::Score>, telemetry: SolverTelemetry)` |
| `emit_best_solution` | `fn(&self, solution: S, current_score: Option<S::Score>, best_score: S::Score, telemetry: SolverTelemetry)` |
| `pause_with_snapshot` | `fn(&self, solution: S, current_score: Option<S::Score>, best_score: Option<S::Score>, telemetry: SolverTelemetry) -> bool` |
| `emit_completed` | `fn(&self, solution: S, current_score: Option<S::Score>, best_score: S::Score, telemetry: SolverTelemetry, terminal_reason: SolverTerminalReason)` |
| `emit_cancelled` | `fn(&self, current_score: Option<S::Score>, best_score: Option<S::Score>, telemetry: SolverTelemetry)` |
| `emit_failed` | `fn(&self, error: String)` |

### `Analyzable` — `manager/solution_manager.rs`

Requires: `PlanningSolution + Clone + Send + 'static`.

| Method | Signature |
|--------|-----------|
| `analyze` | `fn(&self) -> ScoreAnalysis<Self::Score>` |

### `ProblemChange<S: PlanningSolution>` — `realtime/problem_change.rs`

Requires: `Send + Debug`.

| Method | Signature |
|--------|-----------|
| `apply` | `fn(&self, score_director: &mut dyn Director<S>)` |

## Move Types

All moves are generic over `S` (solution) and `V` (value). All use concrete `fn` pointers for zero-erasure access.

| Move | Type Params | Key Fields | Clone | Copy |
|------|-------------|------------|-------|------|
| `ChangeMove` | `<S, V>` | entity_index, to_value, getter/setter fn ptrs | Yes (V: Clone) | Yes (V: Copy) |
| `SwapMove` | `<S, V>` | left/right entity indices, getter/setter fn ptrs | Yes | Yes |
| `ListChangeMove` | `<S, V>` | src/dst entity+position, list_len/remove/insert fn ptrs | Yes | Yes |
| `ListSwapMove` | `<S, V>` | first/second entity+position, list_len/get/set fn ptrs | Yes | Yes |
| `ListReverseMove` | `<S, V>` | entity_index, start/end, list_len/reverse fn ptrs | Yes | Yes |
| `ListRuinMove` | `<S, V>` | entity_index, SmallVec element_indices, fn ptrs | Yes (manual) | No |
| `SublistChangeMove` | `<S, V>` | src entity+start/end, dst entity+position, fn ptrs | Yes | Yes |
| `SublistSwapMove` | `<S, V>` | first/second entity+start/end, fn ptrs | Yes | Yes |
| `PillarChangeMove` | `<S, V>` | Vec entity_indices, to_value, getter/setter fn ptrs | Yes (manual) | No |
| `PillarSwapMove` | `<S, V>` | Vec left/right indices, getter/setter fn ptrs | Yes (manual) | No |
| `RuinMove` | `<S, V>` | SmallVec entity_indices, getter/setter fn ptrs | Yes (manual) | No |
| `RuinRecreateMove` | `<S>` | SmallVec ruined entities, bounded recreate value source, getter/setter fn ptrs | Yes (manual) | No |
| `CompoundScalarMove` | `<S>` | provider/group reason plus N scalar edits with per-edit descriptor/entity/variable/from/to scope | Yes (manual) | No |
| `ConflictRepairMove` | `<S>` | thin wrapper over `CompoundScalarMove` for provider repair edits | Yes (manual) | No |
| `KOptMove` | `<S, V>` | [CutPoint; 5], KOptReconnection, fn ptrs | Yes (manual) | No |
| `CompositeMove` | `<S, M1, M2>` | index_1, index_2, PhantomData | Yes | Yes |
| `SequentialCompositeMove` | `<S, M>` | owned two-move arena plus cached descriptor/entity/tabu metadata | Yes (M: Clone) | No |

### Move Union Enums

**`ScalarMoveUnion<S, V>`** — Scalar variable union:
- `Change(ChangeMove<S, V>)`, `Swap(SwapMove<S, V>)`, `PillarChange(PillarChangeMove<S, V>)`, `PillarSwap(PillarSwapMove<S, V>)`, `RuinRecreate(RuinRecreateMove<S>)`, `ConflictRepair(ConflictRepairMove<S>)`, `CompoundScalar(CompoundScalarMove<S>)`, `Composite(SequentialCompositeMove<S, ScalarMoveUnion<S, V>>)`

**`ListMoveUnion<S, V>`** — List variable union:
- `ListChange`, `ListSwap`, `SublistChange`, `SublistSwap`, `ListReverse`, `KOpt`, `ListRuin`, `Composite`

### Supporting Types

**`MoveArena<M>`** — O(1) arena allocator. `push()`, `take(index)`, `reset()`, `shuffle()`, `extend()`. Panics on double-take.

**`MoveCursor<S, M>`** — cursor contract with `next_candidate()`, `candidate(index)`, and `take_candidate(index)`.

**`MoveCandidateRef<'a, S, M>`** — borrowable move view: either `Borrowed(&M)` or `Sequential(SequentialCompositeMoveRef<'a, S, M>)`.

**`CutPoint`** — `{ entity_index: usize, position: usize }`. Derives: Clone, Copy, Debug, Default, PartialEq, Eq.

**`KOptReconnection`** — `{ segment_order: [u8; 6], reverse_mask: u8, len: u8 }`. Derives: Clone, Copy, Debug, PartialEq, Eq.

## Selector Types

### Entity Selectors

| Selector | Note |
|----------|------|
| `FromSolutionEntitySelector` | Iterates entities from descriptor. `with_skip_pinned()`, `with_is_pinned_fn()` |
| `AllEntitiesSelector` | Iterates all entities across all descriptors |
| `NearbyEntitySelector<S, M, ES>` | Distance-pruned entity selection |
| `MimicRecordingEntitySelector<S, ES>` | Records selections for replay |
| `MimicReplayingEntitySelector` | Replays recorded selections |

### Value Selectors

| Selector | Note |
|----------|------|
| `StaticValueSelector<S, V>` | Fixed value list |
| `FromSolutionValueSelector<S, V>` | Extracts values from solution via `fn(&S) -> Vec<V>` |
| `RangeValueSelector<S>` | Generates 0..count_fn(solution) |

### Move Selectors

| Selector | Produces | Note |
|----------|----------|------|
| `ChangeMoveSelector<S, V, ES, VS>` | `ChangeMove<S, V>` | Scalar variable change; `.with_allows_unassigned(true)` adds exactly one assigned-entity `Some(v) -> None` move |
| `SwapMoveSelector<S, V, LES, RES>` | `SwapMove<S, V>` | Scalar variable swap |
| `ScalarChangeMoveSelector<S, V, ES, VS>` | `ScalarMoveUnion<S, V>` | Wraps ChangeMoveSelector |
| `ScalarSwapMoveSelector<S, V, LES, RES>` | `ScalarMoveUnion<S, V>` | Wraps SwapMoveSelector |
| `ListChangeMoveSelector<S, V, ES>` | `ListChangeMove<S, V>` | List element relocation; canonical order, exact `size()` |
| `ListSwapMoveSelector<S, V, ES>` | `ListSwapMove<S, V>` | List element swap; canonical pair order, exact `size()` |
| `ListReverseMoveSelector<S, V, ES>` | `ListReverseMove<S, V>` | Segment reversal (2-opt) |
| `ListRuinMoveSelector<S, V>` | `ListRuinMove<S, V>` | LNS element removal |
| `SublistChangeMoveSelector<S, V, ES>` | `SublistChangeMove<S, V>` | Segment relocation (Or-opt); canonical order, exact `size()` |
| `SublistSwapMoveSelector<S, V, ES>` | `SublistSwapMove<S, V>` | Segment swap; canonical pair order, exact `size()` |
| `KOptMoveSelector<S, V, ES>` | `KOptMove<S, V>` | K-opt tour optimization |
| `NearbyKOptMoveSelector<S, V, D, ES>` | `KOptMove<S, V>` | Distance-pruned k-opt |
| `NearbyListChangeMoveSelector<S, V, D, ES>` | `ListChangeMove<S, V>` | Distance-pruned relocation with stable tie ordering |
| `NearbyListSwapMoveSelector<S, V, D, ES>` | `ListSwapMove<S, V>` | Distance-pruned swap with canonical pair ordering |
| `RuinMoveSelector<S, V>` | `RuinMove<S, V>` | Scalar variable LNS using `RuinVariableAccess<S, V>` |
| `ConflictRepairSelector<S>` | `ConflictRepairMove<S>` | Provider-backed scalar repair for configured scoring constraints |

List-selector lifting is direct union assembly. The canonical list builder opens
concrete list leaves straight into `ListMoveUnion<S, V>` at leaf-open time, so
cartesian-safe decorators stay same-type and cursor-native instead of relying
on a generic type-lifting map adapter.

### Selector Decorators

| Decorator | Type Params | Note |
|-----------|-------------|------|
| `UnionMoveSelector<S, M, A, B>` | Two selectors | Sequential combination |
| `VecUnionSelector<S, M, Leaf>` | Any number of same-type selectors | Concrete child dispatch with `Sequential`, `RoundRobin`, `RotatingRoundRobin`, or `StratifiedRandom` order and stable selector-index telemetry |
| `CartesianProductArena<S, M1, M2>` | Two move types | Cross-product iteration arena |
| `CartesianProductCursor<S, M>` | One move type | Cursor-backed sequential preview rows with stable pair indices |
| `CartesianProductSelector<S, M, Left, Right>` | Two selectors plus a wrapping function | Preview-state sequential composition with borrowable candidates, selected-winner materialization, optional hard-improvement gating, and pure upper-bound `size()` |
| `FilteringMoveSelector<S, M, Inner>` | Predicate `for<'a> fn(MoveCandidateRef<'a, S, M>) -> bool` | Filters moves without reopening cartesian children |
| `ShufflingMoveSelector<S, M, Inner>` | RNG | Randomizes order without type-lifting moves |
| `SortingMoveSelector<S, M, Inner>` | Comparator `for<'a> fn(MoveCandidateRef<'a, S, M>, MoveCandidateRef<'a, S, M>) -> Ordering` | Sorts borrowable candidates without reopening cartesian children |
| `ProbabilityMoveSelector<S, M, Inner>` | Weight `for<'a> fn(MoveCandidateRef<'a, S, M>) -> f64` | Probabilistic filtering without reopening cartesian children |

Cartesian preview state uses `SequentialPreviewDirector`: it owns a cloned working solution for right-child selector generation, updates shadows for previewed left moves, borrows immutable descriptor and constraint metadata from the source director, and intentionally panics on `calculate_score()`.

Conflict repair constraint keys resolve against scoring metadata by identity:
package-qualified constraints must be configured with `ConstraintRef::full_name()`
strings such as `package/name`, package-less constraints use their short name,
and provider registration keys must match the configured key exactly.

### Supporting Types

**`EntityReference`** — `{ descriptor_index: usize, entity_index: usize }`.

**`Pillar`** — `{ entities: Vec<EntityReference> }`. Methods: `size()`, `is_empty()`, `first()`, `iter()`. Canonical public pillar semantics exclude unassigned entities and singleton pillars; entity order within a pillar is deterministic by `entity_index`.

**`SubPillarConfig`** — `{ enabled: bool, minimum_size: usize, maximum_size: usize }`. Methods: `none()`, `all()`, `with_minimum_size()`, `with_maximum_size()`.

**`SelectionOrder`** — Enum: `Inherit`, `Original`, `Random`, `Shuffled`, `Sorted`, `Probabilistic`. Methods: `resolve()`, `is_random()`, `requires_caching()`.

**`NearbySelectionConfig`** — Builder: `with_distribution_type()`, `with_max_nearby_size()`, `with_min_distance()`.

**`KOptConfig`** — `{ k: usize, min_segment_len: usize, limited_patterns: bool }`. Methods: `new(k)`, `with_min_segment_len()`, `with_limited_patterns()`.

**`RuinVariableAccess<S, V>`** — `selector/ruin.rs`. Scalar-variable access bundle for `RuinMoveSelector::new(min, max, access)`: entity count, getter, setter, variable index, variable name, and descriptor index.

**`ScalarVariableSlot<S>`** — `builder/context.rs`. Canonical scalar-variable metadata used by the monomorphized runtime. The compact scalar `variable_index` is the generated getter/setter dispatch index; hook attachment, descriptor ordering, and user-facing target matching use descriptor index plus variable name, with the canonical entity type name kept for target matching and diagnostics. Getter, setter, and entity-local value sources receive the scalar variable index so selector hot paths do not need descriptor-erased access. In addition to value-source hooks it carries optional nearby hooks and scalar construction order-key hooks via builder-style methods:
- `with_candidate_values(for<'a> fn(&'a S, usize, usize) -> &'a [usize])` for bounded scalar value candidates
- `with_nearby_value_candidates(for<'a> fn(&'a S, usize, usize) -> &'a [usize])` for nearby scalar change
- `with_nearby_entity_candidates(for<'a> fn(&'a S, usize, usize) -> &'a [usize])` for nearby scalar swap
- `with_nearby_value_distance_meter(fn(&S, usize, usize, usize) -> Option<f64>)` to rank/filter nearby value candidates
- `with_nearby_entity_distance_meter(fn(&S, usize, usize, usize) -> Option<f64>)` to rank/filter nearby entity candidates
- `with_construction_entity_order_key(fn(&S, usize, usize) -> Option<i64>)` for decreasing or queue-style entity ordering
- `with_construction_value_order_key(fn(&S, usize, usize, usize) -> Option<i64>)` for weakest-fit, strongest-fit, or queue-style value ordering

Runtime scalar construction resolves one canonical binding set per variable by
overlaying these runtime hooks onto descriptor-discovered scalar bindings by
descriptor index and variable name. Validation and execution use that
same resolved binding set. Construction order-key hooks are construction-only:
ordinary local-search change, pillar, and ruin/recreate selectors use canonical
bounded candidate order and do not reorder candidates from
`construction_value_order_key`.

**`ScalarGroup<S>` / `ScalarGroupBinding<S>`** — `planning/scalar/*` and
`builder/context.rs`. `ScalarGroup<S>` is the public model-owned declaration
used by grouped construction and grouped local-search selectors. It declares
real scalar targets through `ScalarTarget<S>` and a candidate provider returning
`ScalarCandidate<S>` values. Macro/runtime assembly binds those public targets
to internal `ScalarGroupBinding<S>` values before phase construction. A candidate is
applied as one `CompoundScalarMove<S>` after framework legality, duplicate, and
not-doable checks. Grouped construction is opt-in by `ConstructionHeuristicConfig
{ group_name }`; without a group name scalar construction remains single-slot.
`ScalarGroupLimits` separates `value_candidate_limit`,
`group_candidate_limit`, and `max_moves_per_step`. Grouped construction uses
config values first and falls back to model-owned `ScalarGroup::with_limits`
values; it passes effective limits to providers or stock assignment candidate
generation and applies `group_candidate_limit` after framework normalization.
Grouped local search uses config values first, falls back to model-owned
`value_candidate_limit` and `max_moves_per_step`, and does not apply
construction-only `group_candidate_limit`.
`ScalarCandidate` exposes construction metadata with
`with_construction_slot_key(usize)`,
`with_construction_entity_order_key(i64)`, and
`with_construction_value_order_key(i64)`. Grouped construction uses an explicit
slot key when supplied; otherwise it keys frontier completion by the exact
sorted set of scalar target slots touched by the candidate.

**Assignment-backed `ScalarGroup<S>` / `ScalarAssignmentBinding<S>`** —
`planning/scalar/assignment.rs`, `planning/scalar/group.rs`, and
`builder/context/scalar/group.rs`.
`ScalarGroup::assignment` is the stock nullable scalar assignment declaration
over one scalar target. It declares required-entity, capacity-key,
assignment-rule, position-key, sequence-key, entity-order, value-order, and
limit hooks. Assignment rules are adjacent-edge checks over post-edit
assignment pairs; tentative assignment topology stays internal to grouped
assignment state.
Macro/runtime assembly binds public `ScalarGroup<S>` values to internal scalar
group bindings before phase or selector construction. Assignment-backed
construction generates stock grouped candidates and uses the same grouped
selection engine as candidate-backed groups.

**`IntraDistanceAdapter<T>`** — `builder/context.rs`. Newtype wrapping `T: CrossEntityDistanceMeter<S>`. Implements `ListPositionDistanceMeter<S>` by forwarding to `T::distance` with `src_entity_idx == dst_entity_idx`. Used by `ListMoveSelectorBuilder::push_kopt` when `max_nearby > 0`.

**`MimicRecorder`** — Shared state for recording/replaying entity selections. Methods: `new(id)`, `get_has_next()`, `get_recorded_entity()`, `reset()`.

## Phase Types

### Construction Heuristic

**`ConstructionHeuristicPhase<S, M, P, Fo>`** — Bounds: `P: EntityPlacer<S, M>`, `Fo: ConstructionForager<S, M>`. `with_live_placement_refresh()` switches order-sensitive scalar heuristics from phase-start placement snapshots to per-step recomputation. Forager step selection is dispatched through the concrete `Fo` type; stock foragers provide prompt/control-aware selection without `dyn Any` routing.

Runtime routing is capability-driven:
- scalar-only `FirstFit` and `CheapestInsertion` use the descriptor boundary
- named grouped scalar construction uses explicit `ScalarGroup` declarations bound to runtime scalar slots and applies all candidate edits atomically
- assignment-backed `ScalarGroup` declarations run hard-first required
  assignment allocation before optional slots; omitted-phase defaults use
  grouped `CheapestInsertion` for both assignment passes
- explicit scalar construction targets that name assignment-owned scalar
  variables must use the owning `group_name`; ungrouped construction for those
  targets is rejected before phase execution
- scalar-only heuristics validate required scalar order-key hooks from the resolved descriptor-plus-runtime binding set before phase build
- list-only route heuristics validate required owner-aware route hooks before phase build
- generic mixed construction stays in the canonical engine

Grouped scalar construction is slot-first: candidates are normalized into exact
group slots, the next grouped slot is selected, and then a candidate inside
that slot is chosen. Supported grouped scalar heuristics are `FirstFit`,
`FirstFitDecreasing`, `CheapestInsertion`, `WeakestFit`,
`WeakestFitDecreasing`, `StrongestFit`, and `StrongestFitDecreasing`. The
decreasing variants require construction entity-order metadata; weakest and
strongest variants require construction value-order metadata. Candidate-backed
groups provide this metadata on `ScalarCandidate`; assignment-backed groups
provide it through `ScalarGroup::with_entity_order` and
`ScalarGroup::with_value_order`. Queue-based construction heuristics are
rejected when `group_name` is set because they do not yet have a faithful
grouped queue contract.

Grouped scalar local-search selectors emit `CompoundScalarMove` candidates.
When `GroupedScalarMoveSelectorConfig.require_hard_improvement` is true, those
candidates carry the shared hard-improvement requirement enforced by local
search, VND, and cartesian composition.

Assignment-backed scalar construction generates compound scalar moves from a
named scalar group. Required entities are handled before optional entities. A
required assignment pass builds one hard-first allocation candidate for dense
multi-slot coverage, while single-slot required construction still exposes
bounded candidates to the grouped selector so `cheapest_insertion` and
weakest/strongest variants preserve their scoring and value-order semantics.
Required assignments may displace optional occupants, move required blockers,
and use assignment-rule legality through the shared assignment state. With
`construction_obligation = assign_when_candidate_exists`, required assignment
construction may commit a doable candidate even when the unassigned baseline
scores better. Optional assignments remain score-improving only.

Assignment-backed grouped scalar selectors emit `CompoundScalarMove`
candidates for unassigned required entities, capacity conflicts, bounded
same-sequence rematches, and bounded reassignments. The selector uses
`GroupedScalarMoveSelectorConfig` values first, falls back to model-owned
`ScalarGroup::with_limits` values for `max_moves_per_step` and
`value_candidate_limit`, and carries `require_hard_improvement` from selector
config.

Construction foragers:

| Forager | Strategy |
|---------|----------|
| `FirstFitForager<S, M>` | First doable move |
| `BestFitForager<S, M>` | Best scoring move |
| `FirstFeasibleForager<S, M>` | First feasible move |
| `WeakestFitForager<S, M>` | Lowest live strength on the current working solution; when optional keep-current legality is enabled, keeps `None` unless the selected move strictly beats the current legal baseline |
| `StrongestFitForager<S, M>` | Highest live strength on the current working solution; when optional keep-current legality is enabled, keeps `None` unless the selected move strictly beats the current legal baseline |

Entity placers:

| Placer | Note |
|--------|------|
| `QueuedEntityPlacer<S, V, ES, VS>` | Iterates entities, generates ChangeMove per value, and can mark keep-current as legal for optional variables via `.with_allows_unassigned(true)` so weakest-fit and strongest-fit may legally keep `None` |
| `SortedEntityPlacer<S, M, Inner>` | Wraps placer, sorts entities by comparator |

**`Placement<S, M>`** — public fields `{ entity_ref: EntityReference, moves: Vec<M> }`; methods `is_empty()`, `with_keep_current_legal()`, `keep_current_legal()`, `take_move()`.

### Local Search

**`LocalSearchPhase<S, M, MS, A, Fo>`** — Bounds: `MS: MoveSelector<S, M>`, `A: Acceptor<S>`, `Fo: LocalSearchForager<S, M>`.

Omitted runtime configuration builds state-aware construction followed by one
model-aware streaming local-search phase. Omitted local-search configuration
uses typed selector defaults: list variables receive nearby list change/swap,
sublist change/swap, reverse, k-opt when k-opt hooks exist, and list ruin when
the list runtime supports ruin moves. Scalar variables with nearby hooks receive
targeted nearby scalar change/swap neighborhoods first; every
non-assignment-owned scalar slot keeps targeted plain change/swap fallback
neighborhoods after nearby coverage. Scalar groups add grouped-scalar
neighborhoods, and registered conflict repair
providers add compound conflict repair neighborhoods. Broad stock unions use
fair selection order and finite accepted-count horizons so search can improve
incumbents under short budgets. VND remains available only through explicit
local-search config; explicit configs own their acceptor, forager, selector, VND
neighborhoods, and union order exactly.

Assignment-owned scalar variables stay on the grouped scalar path. Default
plain scalar neighborhoods and default conflict-repair neighborhoods exclude
scalar slots covered by assignment-backed `ScalarGroup` declarations, and
explicit scalar selector targets that name an assignment-owned variable are
rejected with a diagnostic pointing at the owning grouped scalar selector.

Typed custom search is compiled into the solution, not loaded from a runtime
registry. A solution can declare `#[planning_solution(search = "path::to::search")]`.
The search function receives a `SearchContext<S, V, DM, IDM>`, calls
`ctx.defaults()`, and registers named phases with `.phase("name", |ctx| ...)`.
TOML can then order those compiled-in names with `[[phases]] type = "custom"
name = "..."`. Custom phases implement `CustomSearchPhase<S>` or use the
typed `local_search(selector, acceptor, forager)` helper. Generated code lowers
the result into concrete enums over known phase types; there is no
`Box<dyn Phase>` registry.

Local search foragers:

| Forager | Strategy |
|---------|----------|
| `AcceptedCountForager<S>` | Stop after N accepted moves and pick the best among them |
| `FirstAcceptedForager<S>` | First accepted |
| `BestScoreForager<S>` | Full-neighborhood greedy best accepted move |
| `FirstBestScoreImprovingForager<S>` | First improving best |
| `FirstLastStepScoreImprovingForager<S>` | First improving last step |

### Acceptors

| Acceptor | Type Param | Key Config |
|----------|------------|------------|
| `HillClimbingAcceptor` | — | — |
| `LateAcceptanceAcceptor<S>` | `S: PlanningSolution` | `late_acceptance_size` |
| `SimulatedAnnealingAcceptor` | — | `level_temperatures`, `decay_rate`, `hill_climbing_temperature`, `hard_regression_policy`, `calibration` |
| `TabuSearchAcceptor<S>` | `S: PlanningSolution` | `entity_tabu_size`, `value_tabu_size`, `move_tabu_size`, `undo_move_tabu_size`, `aspiration_enabled`; config with all four sizes omitted normalizes to move-tabu-only with `move_tabu_size = 10` |
| `EntityTabuAcceptor` | — | `entity_tabu_size` |
| `ValueTabuAcceptor` | — | `value_tabu_size` |
| `MoveTabuAcceptor` | — | `move_tabu_size`, `aspiration_enabled` |
| `GreatDelugeAcceptor<S>` | `S: PlanningSolution` | `rain_speed` |
| `StepCountingHillClimbingAcceptor<S>` | `S: PlanningSolution` | `step_count_limit` |
| `DiversifiedLateAcceptanceAcceptor<S>` | `S: PlanningSolution` | `late_acceptance_size`, `tolerance` |
| `AnyAcceptor<S>` | `S: PlanningSolution` | Enum over all built-in acceptors; returned by `AcceptorBuilder::build()` |

### Exhaustive Search

**`ExhaustiveSearchPhase<Dec>`** — Bounds: `Dec: ExhaustiveSearchDecider<S, D>`.

**`ExplorationType`** — `DepthFirst`, `BreadthFirst`, `ScoreFirst`, `OptimisticBoundFirst`.

**`ExhaustiveSearchConfig`** — `{ exploration_type, node_limit, depth_limit, enable_pruning }`.

`ExhaustiveSearchPhase` is cooperative with solver lifecycle control: every explored node advances the phase step count and the frontier loop polls pause, cancel, time, and in-phase limits before applying the next partial assignment. Finite `node_limit` or `depth_limit` bounds that leave frontier work unexplored terminate as `TerminatedByConfig`; an exhausted frontier remains `Completed`.

**`ExhaustiveSearchNode<S>`** — Tree node: depth, score, optimistic_bound, descriptor/variable/entity/candidate indices, parent_index. A node can reconstruct its scalar assignment path from stored parent indices.

**`SimpleDecider<S, V, B>`** — Generic decider with values and optional bounder.

Score bounders: `SoftScoreBounder`, `FixedOffsetBounder<S>`, `()` (no-op).

### Partitioned Search

**`PartitionedSearchPhase<S, PD, Part, SDF, PF, CP>`** — Generic over partitioner, score director factory, phase factory, child phases. Child scopes inherit runtime control, environment mode, remaining time limit, in-phase limits, and deterministic child seeds, but retained-job publication stays on the parent scope. Pause checkpoints are emitted only from the parent full-solution boundary; child pause/cancel/config termination outcomes prevent partition merge.

**`FunctionalPartitioner<S, PF, MF>`** — Closure-based partitioner.

**`ThreadCount`** — `Auto`, `Unlimited`, `Specific(usize)`. `PartitionedSearchPhase` solves child partitions sequentially when the resolved count is `1`, otherwise it installs a dedicated Rayon pool whose worker count matches the resolved value.

### Variable Neighborhood Descent

Variable Neighborhood Descent is an internal `local_search_type`, not a public
standalone phase API. Configured runtime solving reaches it through
`LocalSearchStrategy`. It scans a neighborhood to completion only while no
timeout, pause, or cancel is pending; interruption returns the last committed
incumbent and never applies a partial best move discovered during the abandoned
scan.

## Scope Hierarchy

### `ProgressCallback<S>` — `scope/solver.rs`

Sealed trait for zero-allocation callback dispatch. Implemented for `()` (no-op) and any `F: for<'a> Fn(SolverProgressRef<'a, S>) + Send + Sync`.

### `SolverScope<'t, S, D, ProgressCb = ()>`

Top-level scope for a retained solve. Holds score director, current score, best solution, best score, RNG, active timing, stats, runtime bridge, terminal reason, and termination state.

Key methods: `new(score_director)`, `new_with_callback(score_director, callback, terminate, runtime)`, `with_progress_callback(F) -> SolverScope<.., F>`, `with_runtime(runtime)`, `start_solving()`, `initialize_working_solution_as_best()`, `replace_working_solution_and_reinitialize(solution)`, `score_director()`, `working_solution()`, `mutate(...)`, `current_score()`, `best_score()`, `calculate_score()`, `update_best_solution()`, `report_progress()`, `report_best_solution()`, `pause_if_requested()`, `pause_timers()`, `resume_timers()`, `mark_cancelled()`, `mark_terminated_by_config()`, `is_terminate_early()`, `set_time_limit()`. The current implementation also tracks a working-solution revision for built-in descriptor-driven construction completion; committed mutation goes through `mutate(...)` (or the equivalent crate-private step boundary), which clears `current_score` and advances that revision exactly once. Speculative phase evaluation uses `Move::do_move`, the returned typed undo value, `Move::undo_move`, and `DirectorScoreState` snapshots to restore both solution values and committed score state after scoring a candidate. Internal prompt-control plumbing also exposes immutable `pending_control()` so built-in phases can abandon partial steps and unwind to runtime-owned boundaries before settling pause/cancel/config termination.

Public fields: `inphase_step_count_limit`, `inphase_move_count_limit`, `inphase_score_calc_count_limit`.

### `PhaseScope<'t, 'a, S, D, BestCb = ()>`

Borrows `&mut SolverScope`. Tracks per-phase state: phase_index, starting_score, step_count, PhaseStats. Public committed mutation delegates to `mutate(...)` on the parent solver scope; speculative candidate evaluation is handled by phase evaluation helpers using typed move undo.

### `StepScope<'t, 'a, 'b, S, D, BestCb = ()>`

Borrows `&mut PhaseScope`. Tracks per-step state: step_index, step_score. `complete()` records step in stats, while public committed mutation delegates to the same `mutate(...)` boundary used by `SolverScope`. Crate-private committed move helpers apply selected moves by ownership after candidate evaluation has used typed undo for rollback.

## Termination Types

| Type | Config | Note |
|------|--------|------|
| `TimeTermination` | `Duration` | `seconds()`, `millis()` helpers |
| `StepCountTermination` | `u64` | Total step limit |
| `BestScoreTermination<Sc>` | `Sc: Score` | Target score |
| `BestScoreFeasibleTermination<S, F>` | Closure | `score_at_least_zero()` convenience |
| `UnimprovedStepCountTermination<S>` | `u64` | Steps without improvement |
| `UnimprovedTimeTermination<S>` | `Duration` | Time without improvement |
| `MoveCountTermination<S>` | `u64` | Total moves evaluated |
| `ScoreCalculationCountTermination<S>` | `u64` | Total score calculations |
| `DiminishedReturnsTermination<S>` | `Duration, f64` | Window + min improvement rate |
| `OrTermination<T, S, D>` | Tuple | Any termination triggers |
| `AndTermination<T, S, D>` | Tuple | All must trigger |

Composite terminations use tuple impls (up to 8 elements) generated via `impl_composite_termination!` macro.

## Manager System

### `SolverFactory<S, D, C, P, T>`

Holds score calculator, phases, termination. Methods: `solve()`, `create_solver()`, `builder()`.

### `SolverFactoryBuilder<S, D, C, P, T>`

Fluent builder: `with_phase()`, `with_phase_factory()`, `with_config()`, `with_time_limit()`, `with_step_limit()`, `with_time_limit_or()`, `build()`.

### `SolverManager<S: Solvable>`

Static lifetime retained-job manager: `solve()` returns `(job_id, receiver)`. Methods: `get_status()`, `pause()`, `resume()`, `cancel()`, `delete()`, `get_snapshot()`, `analyze_snapshot()`. The retained lifecycle contract is expressed in neutral `job`, `snapshot`, and `checkpoint` terminology. `pause()` settles at a runtime-owned safe boundary and `resume()` continues from the exact in-process checkpoint. `delete()` hides a terminal job immediately, but the slot itself is not reusable until the solve worker has definitely exited. `MAX_JOBS = 16`.

### `SolverLifecycleState` / `SolverTerminalReason`

Lifecycle states: `Solving`, `PauseRequested`, `Paused`, `Completed`, `Cancelled`, `Failed`. Terminal reasons: `Completed`, `TerminatedByConfig`, `Cancelled`, `Failed`.

### `SolverStatus<Sc>`

Retained job summary from `get_status()`. Fields: `job_id`, `lifecycle_state`, `terminal_reason`, `checkpoint_available`, `event_sequence`, `latest_snapshot_revision`, `current_score`, `best_score`, `telemetry`. `checkpoint_available` means the runtime currently holds an exact resumable checkpoint for `resume()`. Analysis availability is separate from terminality: a job can expose retained snapshots while still solving or pausing.

### `SolverEvent<S>`

Lifecycle event stream for retained jobs. Variants: `Progress`, `BestSolution`, `PauseRequested`, `Paused`, `Resumed`, `Completed`, `Cancelled`, `Failed`. Each event carries metadata with job id, monotonic event sequence, lifecycle state, terminal reason, telemetry, scores, and optional snapshot revision. Event metadata is authoritative: for example, progress can report `PauseRequested` while a pause is still settling toward a `Paused` checkpoint, and once `pause()` is accepted the stream delivers `PauseRequested` before any later worker-side event already in `PauseRequested` state.

### `SolverSnapshot<S>` / `SolverSnapshotAnalysis<Sc>`

Snapshots are renderable and analyzable job states with monotonic `snapshot_revision`. Snapshot analysis is always bound to the chosen revision, never the live mutable job, and remains available for any retained snapshot while the job is solving, pause-requested, paused, completed, cancelled, terminated-by-config, or failed with a retained snapshot. Snapshot analysis is informational only and must not be interpreted as proof that the job is terminal.

### `SolverManagerError`

Lifecycle error surface for invalid transitions and missing retained state: `NoFreeJobSlots`, `JobNotFound`, `InvalidStateTransition`, `NoSnapshotAvailable`, `SnapshotNotFound`.

### `analyze<S>(solution: &S) -> ScoreAnalysis<S::Score>`

Free function. Requires `S: Analyzable, S::Score: Score`. Delegates to `solution.analyze()`.

### `ScoreAnalysis<Sc>` / `ConstraintAnalysis<Sc>`

Serde-serializable. `ScoreAnalysis { score, constraints: Vec<ConstraintAnalysis> }`. `ConstraintAnalysis { name, weight, score, match_count }`.

### Phase Factories

| Factory | Produces |
|---------|----------|
| `ConstructionPhaseFactory<S, M, P, Fo>` | `ConstructionHeuristicPhase` |
| `LocalSearchPhaseFactory<S, M, MS, A, Fo>` | `LocalSearchPhase` |
| `ListConstructionPhaseBuilder<S, E>` | `ListConstructionPhase` |
| `ListCheapestInsertionPhase<S, E>` | Self (implements Phase directly) |
| `ListRegretInsertionPhase<S, E>` | Self (implements Phase directly) |
| `ListClarkeWrightPhase<S, E>` | Self (implements Phase directly) |
| `KOptPhaseBuilder<S, V>` | `KOptPhase` |

`ListClarkeWrightPhase<S, E>` preserves preassigned routes by filling only empty entities, computes owner-aware savings through `route_depot_fn` and `route_distance_fn`, and assigns constructed routes through deterministic matching against `route_feasible_fn`.
`ListKOptPhase<S, E>` uses the same route hooks for per-route 2-opt polishing: `route_get_fn` reads the route, `route_set_fn` writes an accepted route, `route_depot_fn` supplies the owner depot, `route_distance_fn` scores reversals for that owner, and optional `route_feasible_fn` rejects infeasible improvements.

## Real-Time Planning

**`SolverHandle<S>`** — Client-facing handle. `add_problem_change()`, `terminate_early()`, `is_solving()`.

**`ProblemChangeReceiver<S>`** — Server-side receiver. `try_recv()`, `drain_pending()`, `is_terminate_early_requested()`.

**`ProblemChangeResult`** — `Queued`, `SolverNotRunning`, `QueueFull`.

**`ClosureProblemChange<S, F>`** — Wraps `Fn(&mut dyn Director<S>)`.

**`BoxedProblemChange<S>`** — Type alias: `Box<dyn ProblemChange<S>>`.

## Solver & Convenience Functions

### `Solver<'t, P, T, S, D, ProgressCb = ()>`

Main solver struct. Drives phases and checks termination. `impl_solver!` macro generates `solve(self, score_director: D) -> SolveResult<S>` for phase tuples up to 8.

Builder methods: `new(phases)`, `with_termination(T)`, `with_terminate(&AtomicBool)`, `with_time_limit(Duration)`, `with_config(SolverConfig)`, `with_progress_callback<F>(F) -> Solver<.., F>`. The callback type transitions the `ProgressCb` parameter from `()` to the concrete closure type — no `Box<dyn Fn>` allocation.

### `SolveResult<S>`

`{ solution: S, current_score: Option<S::Score>, best_score: S::Score, terminal_reason: SolverTerminalReason, stats: SolverStats }`. Methods: `solution()`, `into_solution()`, `current_score()`, `best_score()`, `terminal_reason()`, `stats()`, `step_count()`, `moves_evaluated()`, `moves_accepted()`.

### `SolverStats` / `PhaseStats`

Aggregate and per-phase metrics: step count, moves generated, moves evaluated,
moves accepted, moves applied, score calculations, elapsed time, generation
time, evaluation time, acceptance rate, selector-level telemetry, and exact
`Throughput { count, elapsed }` views for generated/evaluated work.
Human-facing `moves/s` is derived only at log/console formatting edges.

`SolverTelemetry` snapshots expose the same counters plus not-doable,
acceptor-rejected, forager-ignored, hard-improving/neutral/worse, conflict
repair provider/filter/exposure counters, `construction_slots_assigned`,
`construction_slots_kept`, `construction_slots_no_doable`, and
`scalar_assignment_required_remaining`, which
distinguish scalar construction slots that received a candidate, legally kept
their current unassigned value, had no doable candidate, or remain uncovered
after scalar assignment construction. Grouped scalar
construction records completion against the exact grouped slot and also marks
every scalar slot covered by the grouped decision, so later construction phases
cannot fill those members one by one. `SelectorTelemetry` exposes
`selector_index`, `selector_label`, generated, evaluated, accepted, applied,
not-doable, acceptor-rejected, forager-ignored, hard-delta, conflict-repair,
generation-time, and evaluation-time counters for local-search and VND
selector diagnosis.

### `runtime.rs`

Runtime helpers:

- `RuntimePhase<C, LS>` — generic runtime phase enum with `Construction` and `LocalSearch`
- `LocalSearchStrategy<S, V, DM, IDM>` — runtime local-search wrapper for `acceptor_forager` and `variable_neighborhood_descent`
- `Construction<S, V, DM, IDM>` — runtime construction phase over one `RuntimeModel`; generic `FirstFit` and `CheapestInsertion` use `phase/construction/engine.rs` when matching list work is present, use the descriptor boundary for scalar-only targets, use grouped scalar construction when `group_name` selects a registered `ScalarGroup`, and delegate specialized scalar-only and list-only heuristics to the existing descriptor/list phase implementations. Grouped construction receives all resolved scalar bindings for legality even when the phase target narrows which members must still need work.
- `ListVariableMetadata<S, DM, IDM>` — list-variable metadata surfaced to macro-generated runtime code
- `ListVariableEntity<S>` — list-variable accessors plus `HAS_LIST_VARIABLE`, `LIST_VARIABLE_NAME`, and `LIST_ELEMENT_SOURCE`
- `build_phases()` — builds the runtime phase sequence from `SolverConfig`, `SolutionDescriptor`, and one `RuntimeModel`
- `PlanningModelSupport` — hidden support trait with no default impl; generated by
  `planning_model!` so solution derives can attach descriptor hooks,
  runtime scalar hooks, scalar groups, validate the manifest-backed model, and delegate
  list-shadow updates without proc-macro registries

Scalar-only, list-only, and mixed planning models now target the same canonical runtime layer through `RuntimeModel`. Generic construction order is the descriptor-backed variable order emitted by the macros, and scalar runtime assembly does not depend on Rust module declaration order. Scalar construction is single-slot by default for non-assignment-owned slots; grouped scalar construction is explicit, named, and atomic. Assignment-owned scalar slots are constructed and searched only through their owning grouped scalar path. Specialized list heuristics remain explicit non-generic phases.

### `AnyTermination` / `build_termination()` — `run.rs`

`AnyTermination` is an enum over all built-in termination types for config-driven dispatch. `build_termination()` constructs an `AnyTermination` from a `SolverConfig`.

`log_solve_start()` in the same module emits shape-specific startup telemetry:
list solves log `element_count`, scalar solves log average
`candidate_count`. Console formatting uses those fields to label startup scale
as `elements` or `candidates`.

`LocalSearchPhase` emits `phase_start` with the current score after calculating
the starting local-search score, and emits `phase_end` with the best score. The
console layer renders phase-start scores when present; construction and
partitioned phase starts currently omit a score field.

### `run_solver()` / `run_solver_with_config()` — `run.rs`

Canonical solve entrypoints used by macro-generated solving. They accept generated descriptor/runtime callbacks plus a retained `SolverRuntime<S>` so the runtime can publish lifecycle events, pause at safe boundaries, and preserve snapshot identity across pause/resume. `ScoreDirector` now calls `PlanningSolution::update_all_shadows()` before initialization and `PlanningSolution::update_entity_shadows()` before reinsertion, so the canonical solver path stays fully monomorphized.

## Architectural Notes

- **Zero-erasure throughout.** All moves, selectors, phases, acceptors, foragers, and terminations are fully monomorphized via generics. No `Box<dyn Trait>` or `Arc` in hot paths.
- **Runtime selectors.** `builder/selector.rs` consumes the monomorphized `RuntimeModel` published by macro/runtime assembly and does not synthesize scalar neighborhoods from descriptor bindings.
- **Grouped scalar is explicit.** Nullable scalar variables that must change together use declared scalar groups and compound scalar moves. The solver does not infer groups from unrelated nullable variables.
- **Compound repair is framework-owned.** Conflict repair providers produce domain edit hints, while the selector layer enforces limits, legality, not-doable filtering, hard-improvement filtering, telemetry, affected-entity reporting, and tabu identity.
- **Cartesian stays sequential.** Cartesian selectors compose exactly two child selectors over a preview state. They are not a general atomic grouped-search facility.
- **Projected scoring rows are never planning entities.** Streams created with `.project(...)` are scoring-only internal cache rows owned by scoring constraints. They are not surfaced in `RuntimeModel`, value ranges, construction heuristics, or move selectors.
- **Explicit descriptor boundary.** Construction and selector assembly for scalar-only targets live under `descriptor/*`; canonical local search stays on the monomorphized `RuntimeModel`, while descriptor selectors are only for callers that intentionally choose that boundary.
- **Function pointer storage.** Moves and selectors store index-aware `fn` pointers (e.g., `fn(&S, usize, usize) -> Option<V>`) instead of trait objects for solution access.
- **PhantomData<fn() -> T>** pattern used in all move types to avoid inheriting Clone/Send/Sync bounds from phantom type parameters.
- **SmallVec<[usize; 8]>** used in RuinMove and ListRuinMove for stack-allocated small ruin counts.
- **Tuple-based composition.** Phases and terminations compose via nested tuples with macro-generated impls, avoiding `Vec<Box<dyn Phase>>`.
- **Intentional `dyn` boundaries.** `DynDistanceMeter` in `nearby.rs` and `DefaultPillarSelector` value extractor closures are intentional type-erasure points to avoid monomorphization bloat.
- **`ProblemChange::apply` uses `&mut dyn Director<S>`** — intentional type erasure at the real-time planning boundary.
- **Arena-based move ownership.** Moves are pushed into `MoveArena`, evaluated by index, and taken (moved out) when selected. Never cloned.
- **Neighborhood support modules stay private.** `list_support.rs`, `nearby_list_support.rs`, and `sublist_support.rs` exist only to share selected-entity snapshots, nearby candidate ordering, and exact finite-selector counting. Public cursor hot loops for list and sublist neighborhoods remain explicit.
- **Canonical neighborhood tests live under subsystem trees.** Multi-file selector behavior for list, nearby-list, and sublist families is documented under `heuristic/selector/tests/`, while move legality stays under `heuristic/move/tests/`.
- **Rayon for parallelism.** Partitioned search uses rayon for CPU-bound parallel solving. `tokio::sync::mpsc` for solution streaming.
