# solverforge-solver WIREFRAME

Solver engine: phases, moves, selectors, acceptors, foragers, termination, and solver management.

**Location:** `crates/solverforge-solver/`
**Workspace Release:** `0.8.10`

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
├── runtime.rs                           — Runtime assembly, explicit descriptor-standard construction dispatch, and list metadata hooks
├── list_solver_tests.rs                 — Tests
├── descriptor_standard.rs               — Re-exports the explicit descriptor-standard bindings, selectors, move types, and construction helpers
├── descriptor_standard/
│   ├── bindings.rs                      — Standard-variable binding discovery, matching, and work checks
│   ├── move_types.rs                    — DescriptorChangeMove<S>, DescriptorSwapMove<S>, DescriptorEitherMove<S>
│   ├── selectors.rs                     — DescriptorChangeMoveSelector<S>, DescriptorSwapMoveSelector<S>, DescriptorLeafSelector<S>, build_descriptor_move_selector()
│   ├── construction.rs                  — DescriptorConstruction<S>, DescriptorEntityPlacer<S>, build_descriptor_construction()
│   └── tests.rs                         — Tests
├── runtime_tests.rs                     — Tests
├── run.rs                               — AnyTermination, build_termination, run_solver()
├── run_tests.rs                         — Tests
├── builder/
│   ├── mod.rs                           — Re-exports from all builder submodules
│   ├── acceptor.rs                      — AnyAcceptor<S> enum, AcceptorBuilder
│   ├── acceptor_tests.rs                — Tests
│   ├── forager.rs                       — AnyForager<S> enum, ForagerBuilder
│   ├── context.rs                       — ModelContext<S, V, DM, IDM>, VariableContext<S, V, DM, IDM>, IntraDistanceAdapter<T>
│   ├── selector.rs                      — Selector<S, V, DM, IDM>, Neighborhood<S, V, DM, IDM>, build_move_selector() over published ModelContext variable contexts
│   ├── list_selector.rs                 — Re-exports list selector leaf and builder modules
│   └── list_selector/
│       ├── builder_impl.rs              — ListMoveSelectorBuilder
│       └── leaf.rs                      — ListLeafSelector<S, V, DM, IDM> enum
├── stats.rs                             — SolverStats, PhaseStats
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
│   │   ├── sublist_change.rs           — SubListChangeMove<S, V>
│   │   ├── sublist_swap.rs             — SubListSwapMove<S, V>
│   │   ├── pillar_change.rs            — PillarChangeMove<S, V>
│   │   ├── pillar_swap.rs              — PillarSwapMove<S, V>
│   │   ├── ruin.rs                      — RuinMove<S, V>
│   │   ├── k_opt.rs                     — KOptMove<S, V>, CutPoint
│   │   ├── k_opt_reconnection.rs       — KOptReconnection patterns
│   │   ├── k_opt_reconnection_tests.rs — Tests
│   │   ├── composite.rs                — CompositeMove<S, M1, M2>
│   │   ├── either.rs                    — EitherMove<S, V> enum
│   │   ├── list_either.rs              — ListMoveImpl<S, V> enum
│   │   ├── list_change_tests.rs        — Tests
│   │   ├── k_opt_tests.rs              — Tests
│   │   ├── sublist_change_tests.rs     — Tests
│   │   ├── sublist_swap_tests.rs       — Tests
│   │   └── tests/                       — Additional test modules
│   │       ├── mod.rs
│   │       ├── arena.rs
│   │       ├── change.rs
│   │       ├── swap.rs
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
│       ├── move_selector.rs             — MoveSelector trait, ChangeMoveSelector, SwapMoveSelector, re-exports
│       ├── move_selector/either.rs      — EitherChangeMoveSelector, EitherSwapMoveSelector
│       ├── move_selector/list_adapters.rs — ListMoveListChangeSelector, ListMoveKOptSelector, ListMoveNearbyKOptSelector, ListMoveListRuinSelector
│       ├── move_selector_tests.rs       — Tests
│       ├── list_change.rs              — ListChangeMoveSelector<S, V, ES>
│       ├── list_swap.rs                — ListSwapMoveSelector<S, V, ES>, ListMoveListSwapSelector
│       ├── list_reverse.rs             — ListReverseMoveSelector<S, V, ES>, ListMoveListReverseSelector
│       ├── list_ruin.rs                — ListRuinMoveSelector<S, V>
│       ├── sublist_change.rs           — SubListChangeMoveSelector<S, V, ES>, ListMoveSubListChangeSelector
│       ├── sublist_swap.rs             — SubListSwapMoveSelector<S, V, ES>, ListMoveSubListSwapSelector
│       ├── pillar.rs                    — PillarSelector trait, DefaultPillarSelector, Pillar, SubPillarConfig
│       ├── pillar_tests.rs             — Tests
│       ├── ruin.rs                      — RuinMoveSelector<S, V>
│       ├── mimic.rs                     — MimicRecorder, MimicRecordingEntitySelector, MimicReplayingEntitySelector
│       ├── mimic_tests.rs               — Tests
│       ├── selection_order.rs          — SelectionOrder enum
│       ├── selection_order_tests.rs    — Tests
│       ├── entity_tests.rs              — Tests
│       ├── value_selector_tests.rs     — Tests
│       ├── nearby.rs                    — NearbyDistanceMeter trait, DynDistanceMeter, NearbyEntitySelector, NearbySelectionConfig
│       ├── nearby_list_change.rs       — CrossEntityDistanceMeter trait, NearbyListChangeMoveSelector, ListMoveNearbyListChangeSelector
│       ├── nearby_list_swap.rs         — NearbyListSwapMoveSelector, ListMoveNearbyListSwapSelector
│       ├── decorator/
│       │   ├── mod.rs                   — Re-exports
│       │   ├── cartesian_product.rs    — CartesianProductArena<S, M1, M2>
│       │   ├── cartesian_product_tests.rs — Tests
│       │   ├── filtering.rs            — FilteringMoveSelector<S, M, Inner>
│       │   ├── filtering_tests.rs      — Tests
│       │   ├── probability.rs          — ProbabilityMoveSelector<S, M, Inner>
│       │   ├── probability_tests.rs    — Tests
│       │   ├── shuffling.rs            — ShufflingMoveSelector<S, M, Inner>
│       │   ├── shuffling_tests.rs      — Tests
│       │   ├── sorting.rs              — SortingMoveSelector<S, M, Inner>
│       │   ├── sorting_tests.rs        — Tests
│       │   ├── union.rs                — UnionMoveSelector<S, M, A, B>
│       │   ├── union_tests.rs          — Tests
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
│           ├── list_change.rs
│           ├── list_ruin.rs
│           ├── mimic.rs
│           ├── nearby.rs
│           ├── pillar.rs
│           └── move_selector.rs
│
├── phase/
│   ├── mod.rs                           — Phase<S, D> trait, tuple impls
│   ├── construction/
│   │   ├── mod.rs                       — ForagerType enum, ConstructionHeuristicConfig, re-exports
│   │   ├── phase.rs                     — ConstructionHeuristicPhase<S, M, P, Fo>
│   │   ├── forager.rs                   — ConstructionForager trait, FirstFit/BestFit/FirstFeasible/WeakestFit/StrongestFit foragers
│   │   ├── placer.rs                    — EntityPlacer trait, Placement, QueuedEntityPlacer, SortedEntityPlacer
│   │   ├── phase_tests.rs              — Tests
│   │   ├── forager_tests.rs            — Tests
│   │   └── placer_tests.rs             — Tests
│   ├── localsearch/
│   │   ├── mod.rs                       — LocalSearchConfig, AcceptorType, re-exports
│   │   ├── phase.rs                     — LocalSearchPhase<S, M, MS, A, Fo>
│   │   ├── forager.rs                   — LocalSearchForager trait, AcceptedCountForager, FirstAcceptedForager, BestScoreForager, re-exports
│   │   ├── forager/improving.rs        — FirstBestScoreImprovingForager, FirstLastStepScoreImprovingForager
│   │   ├── forager_tests.rs            — Tests
│   │   ├── phase_tests.rs              — Tests
│   │   └── acceptor/
│   │       ├── mod.rs                   — Acceptor<S> trait, re-exports
│   │       ├── hill_climbing.rs        — HillClimbingAcceptor
│   │       ├── late_acceptance.rs      — LateAcceptanceAcceptor<S>
│   │       ├── simulated_annealing.rs  — SimulatedAnnealingAcceptor
│   │       ├── simulated_annealing_tests.rs — Tests
│   │       ├── tabu_search.rs          — TabuSearchAcceptor<S>
│   │       ├── entity_tabu.rs          — EntityTabuAcceptor
│   │       ├── value_tabu.rs           — ValueTabuAcceptor
│   │       ├── value_tabu_tests.rs     — Tests
│   │       ├── move_tabu.rs            — MoveTabuAcceptor
│   │       ├── move_tabu_tests.rs      — Tests
│   │       ├── great_deluge.rs         — GreatDelugeAcceptor<S>
│   │       ├── great_deluge_tests.rs   — Tests
│   │       ├── step_counting.rs        — StepCountingHillClimbingAcceptor<S>
│   │       ├── step_counting_tests.rs  — Tests
│   │       ├── diversified_late_acceptance.rs — DiversifiedLateAcceptanceAcceptor<S>
│   │       ├── diversified_late_acceptance_tests.rs — Tests
│   │       └── tests.rs                — Tests
│   ├── exhaustive/
│   │   ├── mod.rs                       — ExhaustiveSearchPhase, ExhaustiveSearchConfig, ExplorationType
│   │   ├── bounder.rs                   — ScoreBounder trait, SoftScoreBounder, FixedOffsetBounder
│   │   ├── bounder_tests.rs             — Tests
│   │   ├── config.rs                    — ExhaustiveSearchConfig
│   │   ├── decider.rs                   — ExhaustiveSearchDecider trait, SimpleDecider
│   │   ├── decider_tests.rs             — Tests
│   │   ├── exploration_type.rs          — ExplorationType
│   │   ├── node.rs                      — ExhaustiveSearchNode, MoveSequence
│   │   ├── node_tests.rs                — Tests
│   │   ├── phase.rs                     — ExhaustiveSearchPhase<Dec>
│   │   ├── phase_tests.rs               — Tests
│   │   └── priority_node.rs             — PriorityNode<S>
│   ├── partitioned/
│   │   ├── mod.rs                       — PartitionedSearchPhase, PartitionedSearchConfig, ChildPhases trait
│   │   ├── partitioner.rs              — SolutionPartitioner trait, FunctionalPartitioner, ThreadCount
│   │   ├── partitioner_tests.rs        — Tests
│   │   ├── phase.rs                    — PartitionedSearchPhase<P, Part>
│   │   └── phase_tests.rs              — Tests
│   ├── sequence.rs                      — PhaseSequence<P>
│   ├── dynamic_vnd.rs                   — DynamicVndPhase<S, M, MS>
│   └── vnd/
│       ├── mod.rs                       — Re-exports
│       ├── phase.rs                     — VndPhase, impl_vnd_phase! macro (up to 8 neighborhoods)
│       └── phase_tests.rs               — Tests
│
├── manager/
│   ├── mod.rs                           — PhaseFactory trait, re-exports
│   ├── config.rs                        — LocalSearchType, ConstructionType, PhaseConfig enums
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
│   │   ├── list_clarke_wright_tests.rs — Tests
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
│   ├── mod_tests_integration/runtime_helpers.rs — Shared telemetry helpers
│   └── phase_factory_tests.rs          — Tests
│
├── scope/
│   ├── mod.rs                           — Re-exports
│   ├── solver.rs                        — SolverScope<'t, S, D, ProgressCb = ()>, ProgressCallback trait, lifecycle-aware SolveResult
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

| Method | Signature |
|--------|-----------|
| `is_doable` | `fn<D: Director<S>>(&self, score_director: &D) -> bool` |
| `do_move` | `fn<D: Director<S>>(&self, score_director: &mut D)` |
| `descriptor_index` | `fn(&self) -> usize` |
| `entity_indices` | `fn(&self) -> &[usize]` |
| `variable_name` | `fn(&self) -> &str` |

Moves are **never cloned**. Ownership transfers via `MoveArena` indices.

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
| `is_accepted` | `fn(&mut self, last_step_score: &S::Score, move_score: &S::Score) -> bool` | — |
| `phase_started` | `fn(&mut self, initial_score: &S::Score)` | no-op |
| `phase_ended` | `fn(&mut self)` | no-op |
| `step_started` | `fn(&mut self)` | no-op |
| `step_ended` | `fn(&mut self, step_score: &S::Score)` | no-op |

### `EntitySelector<S: PlanningSolution>` — `entity.rs`

| Method | Signature |
|--------|-----------|
| `iter` | `fn<'a, D: Director<S>>(&'a self, score_director: &'a D) -> impl Iterator<Item = EntityReference> + 'a` |
| `size` | `fn<D: Director<S>>(&self, score_director: &D) -> usize` |
| `is_never_ending` | `fn(&self) -> bool` |

### `MoveSelector<S: PlanningSolution, M: Move<S>>` — `move_selector.rs`

| Method | Signature |
|--------|-----------|
| `iter_moves` | `fn<'a, D: Director<S>>(&'a self, score_director: &'a D) -> impl Iterator<Item = M> + 'a` |
| `size` | `fn<D: Director<S>>(&self, score_director: &D) -> usize` |
| `is_never_ending` | `fn(&self) -> bool` |

### `ValueSelector<S: PlanningSolution, V>` — `value_selector.rs`

| Method | Signature |
|--------|-----------|
| `iter_typed` | `fn<'a, D: Director<S>>(&'a self, score_director: &'a D, descriptor_index: usize, entity_index: usize) -> impl Iterator<Item = V> + 'a` |
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
| `pick_move_index` | `fn<D: Director<S>>(&self, placement: &Placement<S, M>, score_director: &mut D) -> Option<usize>` |

### `LocalSearchForager<S, M>` — `localsearch/forager.rs`

Requires: `Send + Debug`. Bounds: `S: PlanningSolution, M: Move<S>`.

| Method | Signature |
|--------|-----------|
| `step_started` | `fn(&mut self, best_score: S::Score, last_step_score: S::Score)` |
| `add_move_index` | `fn(&mut self, index: usize, score: S::Score)` |
| `is_quit_early` | `fn(&self) -> bool` |
| `pick_move_index` | `fn(&mut self) -> Option<(usize, S::Score)>` |

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

All moves are generic over `S` (solution) and `V` (value). All use typed `fn` pointers for zero-erasure access.

| Move | Type Params | Key Fields | Clone | Copy |
|------|-------------|------------|-------|------|
| `ChangeMove` | `<S, V>` | entity_index, to_value, getter/setter fn ptrs | Yes (V: Clone) | Yes (V: Copy) |
| `SwapMove` | `<S, V>` | left/right entity indices, getter/setter fn ptrs | Yes | Yes |
| `ListChangeMove` | `<S, V>` | src/dst entity+position, list_len/remove/insert fn ptrs | Yes | Yes |
| `ListSwapMove` | `<S, V>` | first/second entity+position, list_len/get/set fn ptrs | Yes | Yes |
| `ListReverseMove` | `<S, V>` | entity_index, start/end, list_len/reverse fn ptrs | Yes | Yes |
| `ListRuinMove` | `<S, V>` | entity_index, SmallVec element_indices, fn ptrs | Yes (manual) | No |
| `SubListChangeMove` | `<S, V>` | src entity+start/end, dst entity+position, fn ptrs | Yes | Yes |
| `SubListSwapMove` | `<S, V>` | first/second entity+start/end, fn ptrs | Yes | Yes |
| `PillarChangeMove` | `<S, V>` | Vec entity_indices, to_value, getter/setter fn ptrs | Yes (manual) | No |
| `PillarSwapMove` | `<S, V>` | Vec left/right indices, getter/setter fn ptrs | Yes (manual) | No |
| `RuinMove` | `<S, V>` | SmallVec entity_indices, getter/setter fn ptrs | Yes (manual) | No |
| `KOptMove` | `<S, V>` | [CutPoint; 5], KOptReconnection, fn ptrs | Yes (manual) | No |
| `CompositeMove` | `<S, M1, M2>` | index_1, index_2, PhantomData | Yes | Yes |

### Move Union Enums

**`EitherMove<S, V>`** — Standard variable union:
- `Change(ChangeMove<S, V>)`, `Swap(SwapMove<S, V>)`

**`ListMoveImpl<S, V>`** — List variable union:
- `ListChange`, `ListSwap`, `SubListChange`, `SubListSwap`, `ListReverse`, `KOpt`, `ListRuin`

### Supporting Types

**`MoveArena<M>`** — O(1) arena allocator. `push()`, `take(index)`, `reset()`, `shuffle()`, `extend()`. Panics on double-take.

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
| `ChangeMoveSelector<S, V, ES, VS>` | `ChangeMove<S, V>` | Standard variable change |
| `SwapMoveSelector<S, V, LES, RES>` | `SwapMove<S, V>` | Standard variable swap |
| `EitherChangeMoveSelector<S, V, ES, VS>` | `EitherMove<S, V>` | Wraps ChangeMoveSelector |
| `EitherSwapMoveSelector<S, V, LES, RES>` | `EitherMove<S, V>` | Wraps SwapMoveSelector |
| `ListChangeMoveSelector<S, V, ES>` | `ListChangeMove<S, V>` | List element relocation |
| `ListSwapMoveSelector<S, V, ES>` | `ListSwapMove<S, V>` | List element swap |
| `ListReverseMoveSelector<S, V, ES>` | `ListReverseMove<S, V>` | Segment reversal (2-opt) |
| `ListRuinMoveSelector<S, V>` | `ListRuinMove<S, V>` | LNS element removal |
| `SubListChangeMoveSelector<S, V, ES>` | `SubListChangeMove<S, V>` | Segment relocation (Or-opt) |
| `SubListSwapMoveSelector<S, V, ES>` | `SubListSwapMove<S, V>` | Segment swap |
| `KOptMoveSelector<S, V, ES>` | `KOptMove<S, V>` | K-opt tour optimization |
| `NearbyKOptMoveSelector<S, V, D, ES>` | `KOptMove<S, V>` | Distance-pruned k-opt |
| `NearbyListChangeMoveSelector<S, V, D, ES>` | `ListChangeMove<S, V>` | Distance-pruned relocation |
| `NearbyListSwapMoveSelector<S, V, D, ES>` | `ListSwapMove<S, V>` | Distance-pruned swap |
| `RuinMoveSelector<S, V>` | `RuinMove<S, V>` | Standard variable LNS |

**ListMove* wrappers** adapt specific move selectors to produce `ListMoveImpl<S, V>`:
`ListMoveListChangeSelector`, `ListMoveListSwapSelector`, `ListMoveListReverseSelector`, `ListMoveSubListChangeSelector`, `ListMoveSubListSwapSelector`, `ListMoveKOptSelector`, `ListMoveNearbyKOptSelector`, `ListMoveListRuinSelector`, `ListMoveNearbyListChangeSelector`, `ListMoveNearbyListSwapSelector`.

### Selector Decorators

| Decorator | Type Params | Note |
|-----------|-------------|------|
| `UnionMoveSelector<S, M, A, B>` | Two selectors | Sequential combination |
| `CartesianProductArena<S, M1, M2>` | Two move types | Cross-product iteration arena |
| `FilteringMoveSelector<S, M, Inner>` | Predicate `fn(&M) -> bool` | Filters moves |
| `ShufflingMoveSelector<S, M, Inner>` | RNG | Randomizes order |
| `SortingMoveSelector<S, M, Inner>` | Comparator `fn(&M, &M) -> Ordering` | Sorts moves |
| `ProbabilityMoveSelector<S, M, Inner>` | Weight `fn(&M) -> f64` | Probabilistic filtering |

### Supporting Types

**`EntityReference`** — `{ descriptor_index: usize, entity_index: usize }`.

**`Pillar`** — `{ entities: Vec<EntityReference> }`. Methods: `size()`, `is_empty()`, `first()`, `iter()`.

**`SubPillarConfig`** — `{ enabled: bool, minimum_size: usize, maximum_size: usize }`. Methods: `none()`, `all()`, `with_minimum_size()`, `with_maximum_size()`.

**`SelectionOrder`** — Enum: `Inherit`, `Original`, `Random`, `Shuffled`, `Sorted`, `Probabilistic`. Methods: `resolve()`, `is_random()`, `requires_caching()`.

**`NearbySelectionConfig`** — Builder: `with_distribution_type()`, `with_max_nearby_size()`, `with_min_distance()`.

**`KOptConfig`** — `{ k: usize, min_segment_len: usize, limited_patterns: bool }`. Methods: `new(k)`, `with_min_segment_len()`, `with_limited_patterns()`.

**`IntraDistanceAdapter<T>`** — `builder/context.rs`. Newtype wrapping `T: CrossEntityDistanceMeter<S>`. Implements `ListPositionDistanceMeter<S>` by forwarding to `T::distance` with `src_entity_idx == dst_entity_idx`. Used by `ListMoveSelectorBuilder::push_kopt` when `max_nearby > 0`.

**`MimicRecorder`** — Shared state for recording/replaying entity selections. Methods: `new(id)`, `get_has_next()`, `get_recorded_entity()`, `reset()`.

## Phase Types

### Construction Heuristic

**`ConstructionHeuristicPhase<S, M, P, Fo>`** — Bounds: `P: EntityPlacer<S, M>`, `Fo: ConstructionForager<S, M>`.

Construction foragers:

| Forager | Strategy |
|---------|----------|
| `FirstFitForager<S, M>` | First doable move |
| `BestFitForager<S, M>` | Best scoring move |
| `FirstFeasibleForager<S, M>` | First feasible move |
| `WeakestFitForager<S, M>` | Lowest strength move |
| `StrongestFitForager<S, M>` | Highest strength move |

Entity placers:

| Placer | Note |
|--------|------|
| `QueuedEntityPlacer<S, V, ES, VS>` | Iterates entities, generates ChangeMove per value |
| `SortedEntityPlacer<S, M, Inner>` | Wraps placer, sorts entities by comparator |

**`Placement<S, M>`** — `{ entity_ref: EntityReference, moves: Vec<M> }`.

### Local Search

**`LocalSearchPhase<S, M, MS, A, Fo>`** — Bounds: `MS: MoveSelector<S, M>`, `A: Acceptor<S>`, `Fo: LocalSearchForager<S, M>`.

Local search foragers:

| Forager | Strategy |
|---------|----------|
| `AcceptedCountForager<S>` | Best of N accepted moves |
| `FirstAcceptedForager<S>` | First accepted |
| `BestScoreForager<S>` | Best overall score |
| `FirstBestScoreImprovingForager<S>` | First improving best |
| `FirstLastStepScoreImprovingForager<S>` | First improving last step |

### Acceptors

| Acceptor | Type Param | Key Config |
|----------|------------|------------|
| `HillClimbingAcceptor` | — | — |
| `LateAcceptanceAcceptor<S>` | `S: PlanningSolution` | `late_acceptance_size` |
| `SimulatedAnnealingAcceptor` | — | `starting_temperature`, `decay_rate` |
| `TabuSearchAcceptor<S>` | `S: PlanningSolution` | `tabu_size`, `aspiration_enabled` |
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

**`ExhaustiveSearchNode<S>`** — Tree node: depth, score, optimistic_bound, entity/value indices, parent_index.

**`MoveSequence<S, M>`** — Stack of moves for branch reconstruction.

**`SimpleDecider<S, V, B>`** — Generic decider with values and optional bounder.

Score bounders: `SoftScoreBounder`, `FixedOffsetBounder<S>`, `()` (no-op).

### Partitioned Search

**`PartitionedSearchPhase<S, D, PD, Part, SDF, PF, CP>`** — Generic over partitioner, score director factory, phase factory, child phases.

**`FunctionalPartitioner<S, PF, MF>`** — Closure-based partitioner.

**`ThreadCount`** — `Auto`, `Unlimited`, `Specific(usize)`.

### VND (Variable Neighborhood Descent)

**`VndPhase<T, M>`** — Wraps tuple of move selectors. `impl_vnd_phase!` macro generates Phase impls for tuples up to 8 neighborhoods.

## Scope Hierarchy

### `ProgressCallback<S>` — `scope/solver.rs`

Sealed trait for zero-allocation callback dispatch. Implemented for `()` (no-op) and any `F: for<'a> Fn(SolverProgressRef<'a, S>) + Send + Sync`.

### `SolverScope<'t, S, D, ProgressCb = ()>`

Top-level scope for a retained solve. Holds score director, current score, best solution, best score, RNG, active timing, stats, runtime bridge, terminal reason, and termination state.

Key methods: `new(score_director)`, `new_with_callback(score_director, callback, terminate, runtime)`, `with_progress_callback(F) -> SolverScope<.., F>`, `with_runtime(runtime)`, `start_solving()`, `working_solution()`, `current_score()`, `best_score()`, `calculate_score()`, `update_best_solution()`, `report_progress()`, `report_best_solution()`, `pause_if_requested()`, `pause_timers()`, `resume_timers()`, `mark_cancelled()`, `mark_terminated_by_config()`, `is_terminate_early()`, `set_time_limit()`. Internal prompt-control plumbing also exposes immutable `pending_control()` so built-in phases can abandon partial steps and unwind to runtime-owned boundaries before settling pause/cancel/config termination.

Public fields: `inphase_step_count_limit`, `inphase_move_count_limit`, `inphase_score_calc_count_limit`.

### `PhaseScope<'t, 'a, S, D, BestCb = ()>`

Borrows `&mut SolverScope`. Tracks per-phase state: phase_index, starting_score, step_count, PhaseStats.

### `StepScope<'t, 'a, 'b, S, D, BestCb = ()>`

Borrows `&mut PhaseScope`. Tracks per-step state: step_index, step_score. `complete()` records step in stats.

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
| `KOptPhaseBuilder<S, V, DM, ESF>` | `KOptPhase` |

`ListClarkeWrightPhase<S, E>` preserves preassigned routes by pairing its route-construction hooks with an explicit per-entity route-length callback when filling remaining work.

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
moves accepted, score calculations, elapsed time, generation time, evaluation
time, acceptance rate, and exact `Throughput { count, elapsed }` views for
generated/evaluated work. Human-facing `moves/s` is derived only at log/console
formatting edges.

### `runtime.rs`

Runtime helpers:

- `RuntimePhase<C, LS, VND>` — generic runtime phase enum with `Construction`, `LocalSearch`, `Vnd`
- `Construction<S, V>` — construction phase over scalar metadata plus ordered list-owner construction hooks
- `ConstructionArgs<S, V>` — per-list-owner function-pointer bundle for list construction hooks
- `ListVariableMetadata<S, DM, IDM>` — list-variable metadata surfaced to macro-generated runtime code
- `ListVariableEntity<S>` — list-variable accessors plus `HAS_LIST_VARIABLE`, `LIST_VARIABLE_NAME`, and `LIST_ELEMENT_SOURCE`
- `build_phases()` — builds the runtime phase sequence from `SolverConfig`, `SolutionDescriptor`, one `ModelContext`, and ordered list construction hooks for zero or more list owners

Scalar and list-heavy models both target this same runtime layer. Documentation and examples should describe one canonical runtime path rather than separate legacy standard/list builders, and multi-owner list construction should be modeled as repeated `ConstructionArgs` records rather than a special-case runtime split.

### `AnyTermination` / `build_termination()` — `run.rs`

`AnyTermination` is an enum over all built-in termination types for config-driven dispatch. `build_termination()` constructs an `AnyTermination` from a `SolverConfig`.

### `run_solver()` — `run.rs`

Canonical solve entrypoint used by macro-generated solving. Accepts generated descriptor/runtime callbacks plus a retained `SolverRuntime<S>` so the runtime can publish lifecycle events, pause at safe boundaries, and preserve snapshot identity across pause/resume. `ScoreDirector` now calls `PlanningSolution::update_all_shadows()` before initialization and `PlanningSolution::update_entity_shadows()` before reinsertion, so the canonical solver path stays fully monomorphized.

## Architectural Notes

- **Zero-erasure throughout.** All moves, selectors, phases, acceptors, foragers, and terminations are fully monomorphized via generics. No `Box<dyn Trait>` or `Arc` in hot paths.
- **Typed runtime selectors.** `builder/selector.rs` consumes the typed `ModelContext` published by macro/runtime assembly and does not synthesize scalar neighborhoods from descriptor bindings.
- **Explicit descriptor-standard boundary.** Descriptor-driven scalar construction and selector assembly live under `descriptor_standard/*` and are used only by callers that intentionally choose that engine.
- **Function pointer storage.** Moves and selectors store `fn` pointers (e.g., `fn(&S, usize) -> Option<V>`) instead of trait objects for solution access.
- **Neutral selector naming.** Public selector modules and types use `move_selector.rs`, `value_selector.rs`, `MoveSelector`, and `ValueSelector`. The trait method `iter_typed(...)` remains for now even though the public type names are prefix-free.
- **PhantomData<fn() -> T>** pattern used in all move types to avoid inheriting Clone/Send/Sync bounds from phantom type parameters.
- **SmallVec<[usize; 8]>** used in RuinMove and ListRuinMove for stack-allocated small ruin counts.
- **Tuple-based composition.** Phases, terminations, and VND neighborhoods compose via nested tuples with macro-generated impls, avoiding `Vec<Box<dyn Phase>>`.
- **Intentional `dyn` boundaries.** `DynDistanceMeter` in `nearby.rs` and `DefaultPillarSelector` value extractor closures are intentional type-erasure points to avoid monomorphization bloat.
- **`ProblemChange::apply` uses `&mut dyn Director<S>`** — intentional type erasure at the real-time planning boundary.
- **Arena-based move ownership.** Moves are pushed into `MoveArena`, evaluated by index, and taken (moved out) when selected. Never cloned.
- **Rayon for parallelism.** Partitioned search uses rayon for CPU-bound parallel solving. `tokio::sync::mpsc` for solution streaming.
