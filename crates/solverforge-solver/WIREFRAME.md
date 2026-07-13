# solverforge-solver WIREFRAME

Solver engine: phases, moves, selectors, acceptors, foragers, termination, and solver management.

**Location:** `crates/solverforge-solver/`
**Workspace Release:** `0.18.0`

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
├── runtime.rs                           — List-variable metadata plus the sole immutable compiled runtime-graph entrypoint
├── runtime/compiler/                    — Immutable runtime-graph compiler, prepared runner, default policy, and executor kernels; every reached source-backed construction boundary validates stable element keys against the frozen declared stream before deciding whether work remains
├── runtime/provider_cursor.rs           — One lazy compound-provider cursor; static Rust providers retain typed candidates/function pointers, while host callbacks alone use raw named edits and object-safe dispatch
├── model_support.rs                     — Hidden `PlanningModelSupport` bridge implemented by `planning_model!` for model-owned scalar hook attachment, scalar group attachment, model validation, and shadow updates
├── list_placement.rs                    — Private partial fixed-owner restriction helpers for list construction, ruin/recreate, Clarke-Wright, and list selectors; detects all-selected-elements-fixed-to-current so intra-owner reordering still streams while cross-owner moves are filtered
├── descriptor.rs                        — Re-exports descriptor bindings, selectors, move types, and internal construction/runtime helpers
├── descriptor/
│   ├── bindings.rs                      — Scalar-variable binding module root and public/internal re-exports
│   ├── bindings/lookup.rs               — Binding collection, target matching, and frontier-aware scalar work checks
│   ├── bindings/variable.rs             — VariableBinding and ResolvedVariableBinding metadata/value-source helpers
│   ├── move_types.rs                    — DescriptorChangeMove<S>, DescriptorSwapMove<S>, DescriptorPillarChangeMove<S>, DescriptorPillarSwapMove<S>, DescriptorRuinRecreateMove<S>, DescriptorMoveUnion<S>
│   ├── move_types/*.rs                  — Descriptor move implementations split by move family
│   ├── selectors.rs                     — Descriptor selector tree, change/swap/pillar/ruin leaves, and build_descriptor_move_selector(config, descriptor, random_seed)
│   ├── selectors/swap_legality.rs       — Descriptor swap legality index over value-range provider shapes
│   ├── selectors/change_swap.rs         — Descriptor change and swap leaf selectors
│   ├── selectors/pillar_ruin.rs         — Descriptor pillar change/swap and ruin-recreate selectors
│   ├── selectors/dispatch.rs            — Descriptor selector dispatch root
│   ├── selectors/dispatch/*.rs          — Descriptor selector dispatch build/type chunks
│   └── tests/mod.rs                     — Descriptor test root with support, selector, cartesian, pillar, nearby, and ruin-recreate chunks under `tests/mod/`
├── run.rs                               — AnyTermination, ChannelProgressCallback, build_termination(), log_solve_start(), and try_run_solver_with_config_and_search()
├── run_tests.rs                         — Tests
├── runtime_build_error.rs               — Public RuntimeBuildError and RuntimeBuildResult declaration/compiler/preparation/execution boundary
├── builder/
│   ├── mod.rs                           — Re-exports from all builder submodules
│   ├── acceptor.rs                      — AnyAcceptor<S> enum, AcceptorBuilder
│   ├── acceptor/tests.rs                — Tests
│   ├── forager.rs                       — AnyForager<S> enum, ForagerBuilder
│   ├── context.rs                       — Public context module root and re-exports
│   ├── context/model.rs                 — RuntimeModel<S, V, DM, IDM> and VariableSlot<S, V, DM, IDM>
│   ├── context/model_resolution.rs      — Descriptor resolution and immutable runtime-model validation
│   ├── context/candidate_metric.rs      — RuntimeCandidateMetric, binding, and immutable registry for sorted/probabilistic leaves
│   ├── context/list.rs                  — Static list slot metadata and stable element-source keys
│   ├── context/list_access/             — Unified static/dynamic list access capabilities and route adapters
│   ├── context/runtime_list*.rs         — Runtime list slots, binding, source, distance, metadata, route, and policy chunks
│   ├── context/scalar_access.rs         — Unified RuntimeScalarSlot/RuntimeScalarEdit access boundary
│   ├── context/provider.rs              — Public frozen compound-provider registry and host callback contracts
│   ├── context/provider/*.rs            — Concrete static pulls, typed/raw normalization, immutable registry storage, and provider contract types
│   ├── context/scalar/mod.rs            — Scalar slot module root and internal re-exports
│   ├── context/scalar/*.rs              — Scalar value-source, variable, and group binding definitions
│   ├── context/scalar/group/*.rs        — Assignment and member binding chunks
│   ├── search.rs                        — Typed custom-search surface: SearchContext, Search, CustomSearchPhase, local_search(), and typed custom phase registration
│   ├── search/*.rs                      — Recursive typed custom-extension registry
│   ├── selector.rs                      — Internal grouped-scalar leaf and recursive compiled-selector composition root
│   ├── selector/grouped_scalar.rs       — GroupedScalarSelector used as one compiled runtime leaf
│   ├── selector/types/composite.rs      — Limited/union/cartesian compiled-selector composition root
│   └── selector/types/composite/        — Cursor execution and retained stream state over compiled leaves
├── planning/                            — Public scalar targets/candidates/groups/assignment rules and conflict-repair declarations
├── stats.rs                             — Statistics, telemetry, and candidate-trace re-export root
├── stats/                               — Solver/phase stats, telemetry payloads, candidate traces, and qualified provenance
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
│   │   ├── list_permute.rs             — ListPermuteMove<S, V>; contiguous intra-list window permutation with exact undo and tabu identity
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
│   │   ├── conflict_repair.rs          — ConflictRepairMove<S> wrapper over framework-owned compound repair edits
│   │   ├── dynamic_scalar_change.rs    — DynamicScalarChangeMove<S> over descriptor-resolved DynamicScalarVariableSlot<S>
│   │   ├── dynamic_scalar_swap.rs      — DynamicScalarSwapMove<S> over descriptor-resolved DynamicScalarVariableSlot<S>
│   │   ├── dynamic_list_change.rs      — DynamicListChangeMove<S> over descriptor-resolved DynamicListVariableSlot<S>
│   │   ├── runtime_compound.rs         — RuntimeCompoundMove<S> and RuntimeCompoundMoveKind for frozen provider candidates
│   │   ├── composite.rs                — CompositeMove<S, M1, M2>, SequentialCompositeMove<S, M>
│   │   ├── scalar_union.rs             — ScalarMoveUnion<S, V> enum
│   │   ├── list_union.rs               — ListMoveUnion<S, V> enum
│   │   ├── list_multi_swap.rs          — ListMultiSwapMove<S, V> for independent same-step intra-list swaps
│   │   ├── list_kernel/                — Shared typed/dynamic list mutation kernels used by public moves and the compiled executor
│   │   └── tests/                       — Additional test modules
│   │       ├── mod.rs
│   │       ├── arena.rs
│   │       ├── change.rs
│   │       ├── swap.rs
│   │       ├── compound_scalar.rs
│   │       ├── conflict_repair.rs
│   │       ├── list_multi_swap.rs
│   │       ├── list_telemetry.rs
│   │       ├── list_change.rs
│   │       ├── list_swap.rs
│   │       ├── list_permute.rs
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
│       ├── move_selector/*.rs           — borrowed.rs candidate cursor, iter.rs adapter, change.rs selector, and swap.rs selector implementation chunks
│       ├── move_selector/scalar_union.rs — ScalarChangeMoveSelector, ScalarSwapMoveSelector
│       ├── dynamic_scalar_change.rs     — DynamicScalarChangeMoveSelector<S> for explicit dynamic scalar change phases
│       ├── dynamic_scalar_nearby_change.rs — Fallible DynamicScalarNearbyChangeMoveSelector<S> facade over the canonical nearby-value leaf
│       ├── dynamic_scalar_nearby_swap.rs — Fallible DynamicScalarNearbySwapMoveSelector<S> facade over the canonical nearby-entity leaf
│       ├── scalar_neighborhood/          — One frozen RuntimeScalarSlot leaf kernel, family recipes/cursors, explicit ruin/recreate stream state, and thin static/dynamic move facades
│       ├── dynamic_list_change.rs       — DynamicListChangeMoveSelector<S> for explicit unrestricted dynamic list-change phases
│       ├── list_change.rs              — ListChangeMoveSelector<S, V, ES>
│       ├── list_support.rs             — Private selected-entity snapshots and exact list-neighborhood counting
│       ├── list_swap.rs                — ListSwapMoveSelector<S, V, ES>
│       ├── list_permute.rs             — ListPermuteMoveSelector<S, V, ES>; cursor-backed contiguous-window permutation
│       ├── list_precedence.rs          — ListPrecedenceMoveSelector<S, V, ES>; cursor-backed critical-path, singleton critical-node, and critical-sublist moves for list variables with plain precedence hooks
│       ├── precedence_route.rs         — Internal precedence route graph and cycle checks shared by list precedence neighborhoods
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
│       ├── entity_tests.rs              — Tests
│       ├── value_selector_tests.rs     — Tests
│       ├── nearby.rs                    — NearbyDistanceMeter trait, DynDistanceMeter, NearbyEntitySelector, NearbySelectionConfig
│       ├── nearby_list_change.rs       — CrossEntityDistanceMeter trait, NearbyListChangeMoveSelector
│       ├── nearby_list_support.rs      — Private selected-entity snapshots and bounded stable top-k nearby candidate ordering
│       ├── nearby_list_swap.rs         — NearbyListSwapMoveSelector
│       ├── nearby_support.rs           — Shared nearest-candidate ordering and bounded stable top-k helpers for nearby neighborhoods
│       ├── list_kernel/                — Shared list candidate enumeration/emission kernels, including precedence and k-opt
│       ├── decorator/
│       │   ├── mod.rs                   — Re-exports
│       │   ├── cartesian_product.rs    — CartesianProductArena<S, M1, M2>, CartesianProductCursor<S, M>, CartesianProductSelector<S, M, Left, Right>
│       │   ├── cartesian_product/tests.rs — Tests
│       │   ├── filtering.rs            — FilteringMoveSelector<S, M, Inner>
│       │   ├── filtering/tests.rs      — Tests
│       │   ├── limited.rs              — Candidate-limit cursor decorator
│       │   ├── limited/tests.rs        — Tests
│       │   ├── mapped_cursor.rs        — Shared mapped cursor adapter
│       │   ├── vec_union.rs            — VecUnionSelector<S, M, Leaf> (Vec-backed union for config-driven composition)
│       │   ├── vec_union/tests.rs      — Tests
│       │   ├── test_utils.rs           — Test helpers
│       │   ├── test_utils_tests.rs     — Test helper tests
│       │   └── {probability,shuffling,sorting,union}/tests.rs — Test-only coverage for ordering and union semantics
│       ├── k_opt/
│       │   ├── mod.rs                   — Re-exports
│       │   ├── config.rs               — KOptConfig
│       │   ├── iterators.rs            — CutCombinationIterator (pub), binomial(), count_cut_combinations()
│       │   ├── distance_meter.rs       — ListPositionDistanceMeter trait, DefaultDistanceMeter
│       │   ├── distance.rs             — ListPositionDistanceMeter and DefaultDistanceMeter mirror used by the k-opt module split
│       │   ├── nearby.rs               — NearbyKOptMoveSelector<S, V, D, ES>
│       │   ├── selector.rs             — KOptMoveSelector<S, V, ES>
│       │   └── tests.rs                — Tests
│       └── tests/
│           ├── mod.rs
│           ├── k_opt.rs
│           ├── list_neighborhood.rs
│           ├── list_permute.rs
│           ├── list_precedence.rs
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
│   │   ├── placer.rs                    — EntityPlacer trait, Placement, and SortedEntityPlacer
│   │   ├── placer/queued.rs             — QueuedEntityPlacer and its single-path streaming candidate cursor with bounded live-candidate storage
│   │   ├── placer/tests.rs              — Tests
│   │   ├── slot.rs                      — ConstructionSlotId, exact-keyed ConstructionGroupSlotId, ConstructionGroupSlotKey, and ConstructionListElementId for construction frontier tracking
│   │   ├── runtime_slots.rs             — Canonical scalar/list/mixed runtime-slot construction root
│   │   ├── runtime_slots/*.rs           — Global placement, move, and per-slot construction chunks
│   │   ├── grouped_scalar/mod.rs        — Atomic grouped scalar construction module root over declared ScalarGroup candidates and assignment groups bound to runtime scalar slots
│   │   ├── grouped_scalar/assignment_candidate.rs — Assignment move options, required assignment moves, capacity-conflict moves, reassignment moves, and remaining-required telemetry
│   │   ├── grouped_scalar/assignment_block.rs — Required-assignment block planning helpers
│   │   ├── grouped_scalar/assignment_cycle.rs — Bounded augmenting cycle and ejection/reinsert move construction
│   │   ├── grouped_scalar/assignment_edge.rs — Small value objects for assignment-rule sequence-edge checks
│   │   ├── grouped_scalar/assignment_entity.rs — Entity-local required, optional, capacity, and reassignment move construction
│   │   ├── grouped_scalar/assignment_family.rs — Shared assignment candidate-family deduplication
│   │   ├── grouped_scalar/assignment_index.rs — Indexed assignment-state map helpers
│   │   ├── grouped_scalar/assignment_pair.rs — Deterministic bounded pair, rematch, and sequence-window move generation
│   │   ├── grouped_scalar/assignment_path.rs — Bounded augmenting-path move construction for required and optional scalar assignments
│   │   ├── grouped_scalar/assignment_required_batch.rs — Dense hard-first required assignment allocation
│   │   ├── grouped_scalar/assignment_state.rs — Assignment occupancy, capacity, rollback, and conflict bookkeeping
│   │   ├── grouped_scalar/assignment_stream.rs — Cursor-backed assignment move streaming for construction and selectors
│   │   ├── grouped_scalar/assignment_value_cycle.rs — Value-cycle rematch generation
│   │   ├── grouped_scalar/assignment_value_index.rs — Value-index helpers for assignment state
│   │   ├── grouped_scalar/assignment_value_release.rs — Optional occupant release planning
│   │   ├── grouped_scalar/assignment_value_run.rs — Same-sequence value-run rematch generation
│   │   ├── grouped_scalar/move_build.rs — CompoundScalarMove construction from public ScalarCandidate edits
│   │   ├── grouped_scalar/placement.rs  — Grouped scalar construction-target and move-strength helpers
│   │   ├── grouped_scalar/phase.rs      — ScalarGroupConstruction builder that feeds grouped scalar placements into stock ConstructionHeuristicPhase
│   │   ├── grouped_scalar/placer.rs     — ScalarGroupPlacer adapter that opens one cursor-backed placement at a time for provider and assignment groups
│   │   └── grouped_scalar/placer_stream.rs — Concrete candidate and assignment placement stream helpers
│   ├── localsearch/
│   │   ├── mod.rs                       — Acceptor, local-search acceptor/forager, and LocalSearchPhase re-exports
│   │   ├── evaluation.rs                — Shared local-search candidate evaluation and hard-delta classification
│   │   ├── phase.rs                     — LocalSearchPhase<S, M, MS, A, Fo>
│   │   ├── phase/tests.rs               — Tests
│   │   ├── forager.rs                   — LocalSearchForager trait, AcceptedCountForager, FirstAcceptedForager, BestScoreForager, re-exports
│   │   ├── forager/any_tests.rs         — AnyForager dispatch tests
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
│       ├── phase.rs                     — Shared VND solve loop called by the compiled runner
│       ├── telemetry.rs                 — Internal VND selector-label helpers using the shared phase progress pulse
│       └── tests.rs                     — Internal VND tests
│
├── manager/
│   ├── mod.rs                           — PhaseFactory trait, re-exports
│   ├── config.rs                        — LocalSearchAcceptorType, ConstructionType, PhaseConfig enums
│   ├── builder.rs                       — SolverFactoryBuilder, SolverBuildError
│   ├── solver_factory.rs               — SolverFactory, solver_factory_builder() free fn
│   ├── solver_manager.rs               — Re-exports retained lifecycle manager surface
│   ├── solver_manager/types.rs         — SolverLifecycleState, SolverTerminalReason, SolverStatus, SolverTelemetryDetail, SolverEventMetadata, SolverEvent, snapshots, and SolverManagerError
│   ├── solver_manager/runtime.rs       — SolverRuntime retained lifecycle publisher and SolverPanicPayload
│   ├── solver_manager/slot.rs          — Internal retained-job slots and snapshot records
│   ├── solver_manager/manager.rs       — MAX_JOBS, Solvable trait, SolverManager
│   ├── solution_manager.rs             — analyze() free fn, Analyzable trait, ScoreAnalysis, ConstraintAnalysis
│   ├── phase_factory/
│   │   ├── mod.rs                       — Re-exports
│   │   ├── construction.rs             — ConstructionPhaseFactory
│   │   ├── list_construction.rs        — Re-exports
│   │   ├── list_construction/access.rs — Source-indexed scored-list access shared by cheapest and regret kernels
│   │   ├── list_construction/round_robin.rs — Explicit-source-key public facade for mandatory-construction round robin
│   │   ├── list_construction/round_robin/kernel.rs — Canonical round-robin enumeration shared by public/static/dynamic adapters
│   │   ├── list_construction/cheapest.rs — Explicit-source-key public facade for canonical cheapest insertion
│   │   ├── list_construction/cheapest/kernel.rs — Source-indexed cheapest candidate ordering, precedence, and owner filtering
│   │   ├── list_construction/cheapest/live.rs — Live score/trace/commit observer for the one cheapest kernel
│   │   ├── list_construction/regret.rs — Explicit-source-key public facade for canonical regret insertion
│   │   ├── list_construction/regret/kernel/ — Source-indexed regret evaluation, precedence, fallback, and execution kernel
│   │   ├── list_clarke_wright.rs       — ListClarkeWrightPhase
│   │   ├── list_clarke_wright/kernel.rs — Canonical savings construction shared by public/static/dynamic adapters
│   │   ├── list_clarke_wright/tests.rs — Tests
│   │   ├── list_clarke_wright/owner_assignment.rs — Owner-specific route assignment and preservation helpers
│   │   ├── list_clarke_wright/route_state.rs — Route state and merge bookkeeping
│   │   ├── list_clarke_wright/savings.rs — Metric-class savings computation
│   │   ├── list_clarke_wright/tests/metric_class.rs — Shared metric-class and owner-feasibility tests
│   │   ├── list_clarke_wright/tests/owner_binding.rs — Owner-specific hook binding tests
│   │   ├── list_k_opt.rs               — ListKOptPhase
│   │   ├── local_search.rs             — LocalSearchPhaseFactory
│   │   └── k_opt.rs                     — KOptPhaseBuilder, KOptPhase
│   ├── phase_factory_trait.rs          — PhaseFactory<S, D> zero-erasure factory trait
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
│   ├── mod_tests_integration/partitioned_lifecycle_tests.rs — Partitioned retained-lifecycle pause/cancel tests
│   ├── mod_tests_integration/analysis_tests.rs — Snapshot analysis retention tests
│   └── mod_tests_integration/runtime_helpers.rs — Shared telemetry helpers
│
├── scope/
│   ├── mod.rs                           — Re-exports
│   ├── solver.rs                        — SolverScope<'t, S, D, ProgressCb = ()>, ProgressCallback trait, lifecycle-aware SolveResult, and included scope chunks
│   ├── solver/progress.rs               — SolverProgressRef, SolverProgressKind, SolverProgressStatus, and ProgressCallback dispatch
│   ├── solver/scope_core.rs             — Core SolverScope construction, shared phase progress pulse, runtime publication, lifecycle control, mutation, and child-scope helpers
│   ├── solver/scope_progress.rs         — SolverScope score/best-solution/progress/stat reporting helpers
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
| `telemetry_label` | `fn(&self) -> &'static str` (default `"move"`) |
| `requires_hard_improvement` | `fn(&self) -> bool` |
| `requires_score_improvement` | `fn(&self) -> bool` (default `false`) |
| `tabu_signature` | `fn<D: Director<S>>(&self, score_director: &D) -> MoveTabuSignature` |
| `candidate_trace_identity` | `fn(&self) -> Option<CandidateTraceIdentity>` (default `None`) |
| `for_each_affected_entity` | `fn(&self, visitor: &mut dyn FnMut(MoveAffectedEntity<'_>))` |

Speculative candidates are **not cloned** by the solver hot path. A move stays
owned by its cursor while it is evaluated by reference; rejected and replaced
candidates are released immediately, and the selected move transfers by value.
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
| `on_solver_terminal` | `fn(&mut self, solver_scope: &mut SolverScope<'_, S, D, ProgressCb>)` (default no-op) |

All concrete phase types implement `Phase<S, D, ProgressCb>` for all `ProgressCb: ProgressCallback<S>`. The solver invokes `on_solver_terminal` once for every configured top-level phase after the phase loop and before final statistics are taken, including a phase skipped by cancellation or configured termination. `PhaseSequence`, tuple, and runtime wrappers propagate that one terminal notification to their active children. Tuple implementations are via `tuple_impl.rs`.

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

Selectors expose cursor-owned storage plus borrowable candidates. The solver
evaluates candidates by reference and only takes ownership of the chosen move by
stable ID. Intentional public owned-stream helpers remain available,
but cartesian composition is intentionally cursor-native and selected-winner
materialization only. Cartesian remains two-child sequential composition over a
preview state; it is not the grouped atomic scalar-search primitive.

| Method | Signature |
|--------|-----------|
| `Cursor<'a>` | `type Cursor<'a>: MoveCursor<S, M> + 'a where Self: 'a` |
| `open_cursor` | `fn<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a>` |
| `open_cursor_with_context` | `fn<'a, D: Director<S>>(&'a self, score_director: &D, context: MoveStreamContext) -> Self::Cursor<'a>` |
| `validate_cursor` | `fn<D: Director<S>>(&self, score_director: &D)` (default no-op) |
| `iter_moves` | `fn<'a, D: Director<S>>(&'a self, score_director: &D) -> MoveSelectorIter<S, M, Self::Cursor<'a>>` |
| `size` | `fn<D: Director<S>>(&self, score_director: &D) -> usize` |
| `append_moves` | `fn<D: Director<S>>(&self, score_director: &D, arena: &mut MoveArena<M>)` |
| `is_never_ending` | `fn(&self) -> bool` |

`MoveStreamContext` is a small copy context passed by runtime streaming
phases. It carries `step_index`, `step_seed`, the finite
`accepted_count_limit` when the forager has one, and `SelectionOrder`.
`with_selection_order()` overrides the default `Original` order and
`selection_order()` reads it. Canonical `open_cursor()`
uses the default context for deterministic explicit scans; local search and
explicit VND call `open_cursor_with_context()` so typed selectors can rotate
entity/value/child order without boxing cursors or erasing selector types.

`MoveCursor::selector_index()` reports the stable child index for telemetry
through union selectors. Cursor implementations store only discovered
candidates, and broad scalar/list selectors generate the next candidate from
cursor-native loop state instead of prebuilding full move vectors. The cursor
contract also provides `release_candidate()`, `apply_owned_candidate()`,
`next_owned_candidate()`, `next_owned_candidate_matching()`, and
`next_owned_candidate_inspected()` so phases can end candidate residency
promptly, leaf cursors can fuse owned filtering with generation, and Cartesian
composition can transfer one inspected child without first storing it twice.

`MoveCandidateRef<'a, S, M>` is either a borrowed move or a borrowable
two-child sequential composite; `MoveCandidateUndo<U>` mirrors those two
shapes for exact rollback. `CandidateStore<M>` is the public cursor-owned
stable-ID store, while `ArenaMoveCursor<'a, M>` adapts a `MoveArena<M>`.
`MoveCursorSource<S, M>` is the phase-facing GAT contract for opening a
resource-aware cursor from solve-owned state. Ordinary public selectors use
the hidden `SelectorCursorSource<MS>` adapter; the compiled runtime implements
the same contract with persistent selector/provider state.

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
| `select_move_index` | `fn<D, BestCb, C>(&self, placement: &mut Placement<S, M, C>, construction_obligation: ConstructionObligation, step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>) -> Option<ConstructionChoice> where C: MoveCursor<S, M>` |

Stock construction foragers consume the placement cursor directly, poll
termination/control during evaluation, release all losing candidate payloads,
and return either `ConstructionChoice::Select(CandidateId)` or `KeepCurrent`.

### `LocalSearchForager<S, M>` — `localsearch/forager.rs`

Requires: `Send + Debug`. Bounds: `S: PlanningSolution, M: Move<S>`.

| Method | Signature |
|--------|-----------|
| `step_started` | `fn(&mut self, best_score: S::Score, last_step_score: S::Score, step_seed: u64)` |
| `add_move_index` | `fn(&mut self, index: CandidateId, score: S::Score) -> ForagerDecision` |
| `is_quit_early` | `fn(&self) -> bool` |
| `accepted_count_limit` | `fn(&self) -> Option<usize>` |
| `pick_move_index` | `fn(&mut self) -> Option<(CandidateId, S::Score)>` |

`ForagerDecision` is `Keep`, `Release`, or `Replace(CandidateId)` and tells the
phase exactly which live candidate payload remains cursor-owned after online
foraging.

`AcceptedCountForager` is the default finite-horizon forager for broad stock
models. It means "select the best among the first N accepted moves", not
"scan the whole neighborhood and retain N". `BestScoreForager` remains
available for explicit full-neighborhood scans.

### `EntityPlacer<S, M>` — `construction/placer.rs`

Requires: `Send + Debug`. Bounds: `S: PlanningSolution, M: Move<S>`.

| Method | Signature |
|--------|-----------|
| `Cursor<'a>` | `type Cursor<'a>: EntityPlacerCursor<S, M> + 'a where Self: 'a` |
| `open_cursor` | `fn<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a>` |

`EntityPlacerCursor::next_placement()` produces one placement envelope at a
time. Each envelope owns one concrete candidate cursor; construction has no
parallel `Vec<Placement>` or `Vec<M>` fallback path.

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

`DefaultCrossEntityDistanceMeter` is the zero-state default implementation. It
returns absolute position distance within one list entity and infinity across
different entities.

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
| `solve` | `fn(self, runtime: SolverRuntime<Self>, qualified_candidate_trace_provenance: Option<QualifiedCandidateTraceRunProvenance>)` |

### `SolverRuntime<S>` — `manager/solver_manager.rs`

Retained-job runtime context passed into `Solvable::solve()`. This is the public lifecycle emitter surface for manual downstream `Solvable` implementations.

| Method | Signature |
|--------|-----------|
| `detached` | `fn detached() -> Self` |
| `job_id` | `fn(&self) -> usize` |
| `is_cancel_requested` | `fn(&self) -> bool` |
| `emit_progress` | `fn(&self, current_score: Option<S::Score>, best_score: Option<S::Score>, telemetry: SolverTelemetry)` |
| `emit_best_solution` | `fn(&self, solution: S, current_score: Option<S::Score>, best_score: S::Score, telemetry: SolverTelemetry)` |
| `pause_with_snapshot` | `fn(&self, solution: S, current_score: Option<S::Score>, best_score: Option<S::Score>, telemetry: SolverTelemetry) -> bool` |
| `emit_completed` | `fn(&self, solution: S, current_score: Option<S::Score>, best_score: S::Score, telemetry: SolverTelemetry, terminal_reason: SolverTerminalReason)` |
| `emit_cancelled` | `fn(&self, current_score: Option<S::Score>, best_score: Option<S::Score>, telemetry: SolverTelemetry)` |
| `emit_failed` | `fn(&self, error: String)` |

`SolverRuntime::detached()` is for synchronous configured solves that are not
retained by a `SolverManager`; it owns an internal lifecycle slot and no event
receiver. `SolverPanicPayload` is the cold foreign-runtime panic boundary. Its
`new(message, payload)`, `message()`, and `into_parts()` methods preserve both a
displayable message and the original `Box<dyn Any + Send>` payload.

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
| `RuntimeCompoundMove` | `<S>` | frozen provider kind, arena-owned reason ID, and unified static/dynamic runtime scalar edits | Yes (manual) | No |
| `DynamicScalarChangeMove` | `<S>` | descriptor-resolved dynamic scalar slot, entity index, optional usize destination | Yes (manual) | No |
| `DynamicScalarSwapMove` | `<S>` | descriptor-resolved dynamic scalar slot, two entity indices | Yes (manual) | No |
| `DynamicListChangeMove` | `<S>` | descriptor-resolved dynamic list slot, source entity/position, destination entity/pre-removal position | Yes (manual) | No |
| `KOptMove` | `<S, V>` | [CutPoint; 5], KOptReconnection, fn ptrs | Yes (manual) | No |
| `CompositeMove` | `<S, M1, M2>` | index_1, index_2, PhantomData | Yes | Yes |
| `SequentialCompositeMove` | `<S, M>` | owned two-move arena plus cached descriptor/entity/tabu metadata | Yes (M: Clone) | No |
| `ListMultiSwapMove` | `<S, V>` | SmallVec independent `(entity, first, second)` intra-list swaps, fn ptrs | Yes (manual) | No |
| `ListPermuteMove` | `<S, V>` | contiguous intra-list window plus explicit permutation | Yes (manual) | No |

`CompoundScalarEdit<S>` is the crate-root edit payload used by
`CompoundScalarMove<S>` and can be built with `static_edit()` or
`dynamic_edit()`, then optionally gated with `with_value_is_legal()`.
`COMPOUND_SCALAR_VARIABLE` is the stable public variable label
`"compound_scalar"` used when a compound move spans several scalar variables.
`RuntimeCompoundMoveKind` distinguishes grouped, conflict-repair, and compound
conflict-repair provider candidates; runtime compound move construction itself
remains owned by the compiled provider cursor.

### Move Union Enums

**`ScalarMoveUnion<S, V>`** — Scalar variable union:
- `Change(ChangeMove<S, V>)`, `Swap(SwapMove<S, V>)`, `PillarChange(PillarChangeMove<S, V>)`, `PillarSwap(PillarSwapMove<S, V>)`, `RuinRecreate(RuinRecreateMove<S>)`, `ConflictRepair(ConflictRepairMove<S>)`, `CompoundScalar(CompoundScalarMove<S>)`, and `RuntimeCompound(RuntimeCompoundMove<S>)`

**`ListMoveUnion<S, V>`** — List variable union:
- `ListChange`, `ListSwap`, `ListMultiSwap`, `ListPermute`, `SublistChange`, `SublistSwap`, `ListReverse`, `KOpt`, and `ListRuin`

`ScalarMoveUnionUndo<S, V>` and `ListMoveUnionUndo<S, V>` mirror their move
union variants and keep speculative rollback statically typed.

### Move Supporting Types

**`MoveArena<M>`** — Reusable-capacity arena. `new()`, `with_capacity()`, `push()`, `get()`, `iter()`, `iter_mut()`, `take(index)`, `reset()`, `extend()`, `shuffle()`, `len()`, `is_empty()`, and `capacity()`. `take()` transfers exactly one selected slot per reset cycle; `reset()` drops the remaining live slots while retaining allocated capacity and panics are used to reject double-take.

**`MoveCursor<S, M>`** — cursor contract with `next_candidate()`, `candidate(id)`, `take_candidate(id)`, `release_candidate(id)`, `apply_owned_candidate(id)`, `next_owned_candidate()`, `next_owned_candidate_matching()`, `next_owned_candidate_inspected()`, and optional `selector_index(id)`. Consumers may stop after any candidate; dropping a cursor releases retained candidates and unconsumed source state without exhausting the tail. Implementations must not require full enumeration for cleanup or callbacks.

**`MoveCandidateRef<'a, S, M>`** — borrowable move view: either `Borrowed(&M)` or `Sequential(SequentialCompositeMoveRef<'a, S, M>)`.

**`MoveStreamContext`** — `{ step_index, step_seed, accepted_count_limit, selection_order }`. Methods: `new()`, `with_selection_order()`, `selection_order()`, `step_index()`, `step_seed()`, `accepted_count_limit()`, `start_offset()`, `stride()`, and `offset_seed()`.

**`CutPoint`** — `{ entity_index: usize, position: usize }`. Derives: Clone, Copy, Debug, Default, PartialEq, Eq.

**`KOptReconnection`** — `{ segment_order: [u8; 6], reverse_mask: u8, len: u8 }`. Derives: Clone, Copy, Debug, PartialEq, Eq.

## Selector Types

### Entity Selectors

| Selector | Note |
|----------|------|
| `FromSolutionEntitySelector` | Iterates every entity index from one descriptor; constructed with `new(descriptor_index)` |
| `AllEntitiesSelector` | Iterates all entities across all descriptors |
| `NearbyEntitySelector<S, M, ES>` | Distance-pruned entity selection |
| `MimicRecordingEntitySelector<S, ES>` | Records selections for replay |
| `MimicReplayingEntitySelector` | Replays recorded selections |

### Value Selectors

| Selector | Note |
|----------|------|
| `StaticValueSelector<S, V>` | Fixed value list |
| `FromSolutionValueSelector<S, V>` | Extracts values from solution via `fn(&S) -> Vec<V>` |
| `PerEntityValueSelector<S, V>` | Extracts owned per-entity values via `fn(&S, usize) -> Vec<V>` |
| `PerEntitySliceValueSelector<S, V>` | Extracts copyable per-entity values via `for<'a> fn(&'a S, usize) -> &'a [V]` |
| `RangeValueSelector<S>` | Generates 0..count_fn(solution) |

### Move Selectors

| Selector | Produces | Note |
|----------|----------|------|
| `ChangeMoveSelector<S, V, ES, VS>` | `ChangeMove<S, V>` | Scalar variable change; `.with_allows_unassigned(true)` adds exactly one assigned-entity `Some(v) -> None` move |
| `SwapMoveSelector<S, V, LES, RES>` | `SwapMove<S, V>` | Scalar variable swap |
| `ScalarChangeMoveSelector<S, V, ES, VS>` | `ScalarMoveUnion<S, V>` | Wraps ChangeMoveSelector |
| `ScalarSwapMoveSelector<S, V, LES, RES>` | `ScalarMoveUnion<S, V>` | Wraps SwapMoveSelector |
| `DynamicScalarChangeMoveSelector<S>` | `DynamicScalarChangeMove<S>` | Explicit dynamic scalar change selector over descriptor-resolved dynamic slots |
| `DynamicScalarNearbyChangeMoveSelector<S>` | `DynamicScalarChangeMove<S>` | Fallible explicit dynamic nearby change selector. It requires declared nearby-value metadata before any callback pull; source access is lazy per row with bounded stable top-k. An empty declared row yields no replacement values but retains its independent optional unassignment candidate. |
| `DynamicScalarNearbySwapMoveSelector<S>` | `DynamicScalarSwapMove<S>` | Fallible explicit dynamic nearby swap selector. It requires declared nearby-entity metadata before any callback pull; source access is lazy per left row with bounded stable top-k and preserves directional source entries. |
| `DynamicListChangeMoveSelector<S>` | `DynamicListChangeMove<S>` | Explicit unrestricted dynamic list-change selector over descriptor-resolved dynamic slots; includes intra-list tail destinations |
| `ListChangeMoveSelector<S, V, ES>` | `ListChangeMove<S, V>` | List element relocation; canonical order, exact `size()` |
| `ListSwapMoveSelector<S, V, ES>` | `ListSwapMove<S, V>` | List element swap; canonical pair order, exact `size()` |
| `ListPermuteMoveSelector<S, V, ES>` | `ListPermuteMove<S, V>` | Contiguous intra-list window permutation |
| `ListPrecedenceMoveSelector<S, V, ES>` | `ListMoveUnion<S, V>` | Critical-path precedence repair over list variables with precedence hooks |
| `ListReverseMoveSelector<S, V, ES>` | `ListReverseMove<S, V>` | Segment reversal (2-opt) |
| `ListRuinMoveSelector<S, V>` | `ListRuinMove<S, V>` | LNS element removal |
| `SublistChangeMoveSelector<S, V, ES>` | `SublistChangeMove<S, V>` | Segment relocation (Or-opt); canonical order, exact `size()` |
| `SublistSwapMoveSelector<S, V, ES>` | `SublistSwapMove<S, V>` | Segment swap; canonical pair order, exact `size()` |
| `KOptMoveSelector<S, V, ES>` | `KOptMove<S, V>` | K-opt tour optimization with bounded cut-metadata windows and lazy per-pattern move construction |
| `NearbyKOptMoveSelector<S, V, D, ES>` | `KOptMove<S, V>` | Distance-pruned k-opt |
| `NearbyListChangeMoveSelector<S, V, D, ES>` | `ListChangeMove<S, V>` | Distance-pruned relocation with bounded stable top-k tie ordering |
| `NearbyListSwapMoveSelector<S, V, D, ES>` | `ListSwapMove<S, V>` | Distance-pruned swap with bounded stable top-k and canonical pair ordering |
| `RuinMoveSelector<S, V>` | `RuinMove<S, V>` | Scalar variable LNS using `RuinVariableAccess<S, V>` |

Configured conflict-repair leaves do not expose a parallel public selector
type. The runtime compiler binds them to the shared provider cursor described
below and emits `ConflictRepairMove`, `CompoundScalarMove`, or
`RuntimeCompoundMove` payloads according to the registered provider kind.

The public concrete cursor carriers returned by those selector GATs are
`ChangeMoveCursor`, `SwapMoveCursor`, `DynamicScalarChangeMoveCursor`,
`DynamicScalarNearbyChangeMoveCursor`, `DynamicScalarNearbySwapMoveCursor`,
`DynamicListChangeMoveCursor`, `ListChangeMoveCursor`, `ListSwapMoveCursor`,
`ListPermuteMoveCursor`, `ListPrecedenceMoveCursor`, `ListReverseMoveCursor`,
`ListRuinMoveCursor`, `SublistChangeMoveCursor`, `SublistSwapMoveCursor`,
`KOptMoveCursor`, `NearbyKOptMoveCursor`, `NearbyListChangeMoveCursor`,
`NearbyListSwapMoveCursor`, `RuinMoveCursor`, and
`RuntimeScalarFacadeCursor`. They are selector return types, not parallel eager
generation APIs.

List-selector lifting is direct union assembly. The canonical list builder opens
concrete list leaves straight into `ListMoveUnion<S, V>` at leaf-open time, so
cartesian-safe decorators stay same-type and cursor-native instead of relying
on a generic type-lifting map adapter.

### Selector Decorators

| Decorator | Type Params | Note |
|-----------|-------------|------|
| `VecUnionSelector<S, M, Leaf>` | Any number of same-type selectors | Concrete child dispatch with `Sequential`, `RoundRobin`, `RotatingRoundRobin`, `Random`, or `StratifiedRandom`; supports equal, fixed, or candidate-count weighting and stable selector-index telemetry |
| `CartesianProductArena<S, M1, M2>` | Two move types | Cross-product iteration arena |
| `CartesianProductSelector<S, M, Left, Right>` | Two selectors plus a wrapping function | Preview-state sequential composition with borrowable candidates, selected-winner materialization, optional hard-improvement gating, and pure upper-bound `size()` |
| `FilteringMoveSelector<S, M, Inner>` | Predicate `for<'a> fn(MoveCandidateRef<'a, S, M>) -> bool` | Filters moves without reopening cartesian children |

Their concrete public cursor types are `VecUnionMoveCursor`,
`FilteringMoveCursor`, and `MappedMoveCursor`; mapping is a cursor carrier used
by typed composition, not a separately configured selector family.

Configured leaf `SelectionOrder` is implemented by the compiled selector
pipeline rather than exported shuffling/sorting/probability wrapper types.

Cartesian preview state uses `SequentialPreviewDirector`: it owns a cloned working solution for right-child selector generation, updates shadows for previewed left moves, borrows immutable descriptor and constraint metadata from the source director, and intentionally panics on `calculate_score()`.

Conflict repair constraint keys resolve against scoring metadata by identity:
package-qualified constraints must be configured with `ConstraintRef::full_name()`
strings such as `package/name`, package-less constraints use their short name,
and provider registration keys must match the configured key exactly.

### Descriptor Scalar Selectors

`build_descriptor_move_selector(config, descriptor, random_seed)` builds the
public descriptor-backed scalar selector tree. `descriptor_has_bindings()`
reports whether a `SolutionDescriptor` contains scalar bindings before a caller
chooses that explicit boundary. `DescriptorSelector<S>` is the outer
`VecUnionSelector` alias and `DescriptorFlatSelector<S>` is the flat leaf-union
alias.

`DescriptorLeafSelector<S>` variants are `Change`, `Swap`, `NearbyChange`,
`NearbySwap`, `PillarChange`, `PillarSwap`, and `RuinRecreate`;
`DescriptorSelectorNode<S>` adds `Leaf` and two-child `Cartesian` composition.
The corresponding concrete selectors are `DescriptorChangeMoveSelector`,
`DescriptorSwapMoveSelector`, `DescriptorNearbyChangeMoveSelector`,
`DescriptorNearbySwapMoveSelector`, `DescriptorPillarChangeMoveSelector`,
`DescriptorPillarSwapMoveSelector`, and
`DescriptorRuinRecreateMoveSelector`. Their public GAT carriers are
`DescriptorChangeMoveCursor`, `DescriptorSwapMoveCursor`,
`DescriptorPillarChangeMoveCursor`, `DescriptorPillarSwapMoveCursor`, and
`DescriptorRuinRecreateMoveCursor`, composed by `DescriptorLeafCursor` and
`DescriptorSelectorCursor`. Nearby change/swap deliberately reuse the change
and swap cursor shapes after bounded nearby filtering.

`DescriptorMoveUnionUndo<S>` mirrors the five variants of
`DescriptorMoveUnion<S>` and is the typed speculative-rollback payload. These
descriptor selectors cover scalar change/swap, nearby, pillar, ruin/recreate,
union, and cartesian configuration only; list, grouped-scalar, conflict-repair,
and limited-neighborhood compilation remains in the canonical runtime graph.

### Selector Supporting Types

**`EntityReference`** — `{ descriptor_index: usize, entity_index: usize }`.

**`Pillar`** — `{ entities: Vec<EntityReference> }`. Methods: `size()`, `is_empty()`, `first()`, `iter()`. Canonical public pillar semantics exclude unassigned entities and singleton pillars; entity order within a pillar is deterministic by `entity_index`.

**`SubPillarConfig`** — `{ enabled: bool, minimum_size: usize, maximum_size: usize }`. Methods: `none()`, `all()`, `with_minimum_size()`, `with_maximum_size()`.

**`SelectionOrder`** — Re-exported from `solverforge-config`. Enum: `Original`, `Random`, `Shuffled`, `Sorted`, `Probabilistic`. Methods: `is_random()`, `requires_complete_stream()`.

**`NearbySelectionConfig`** — Builder: `new()`, `with_distribution_type()`,
`with_max_nearby_size()`, `with_min_distance()`. Its
`NearbyDistributionType` is `Linear` (default), `Parabolic`, or `Block`.

**`KOptConfig`** — `{ k: usize, min_segment_len: usize, limited_patterns: bool }`. Methods: `new(k)`, `with_min_segment_len()`, `with_limited_patterns()`.

`THREE_OPT_RECONNECTIONS` is the seven-pattern built-in 3-opt table;
`enumerate_reconnections(k)` generates the supported reconnection patterns for
other `k` values. `MAX_LIST_PERMUTE_WINDOW_SIZE` is `8`, the fixed public
window bound used by `ListPermuteMove`.

**`RuinVariableAccess<S, V>`** — `selector/ruin.rs`. Scalar-variable access bundle for `RuinMoveSelector::new(min, max, access)`: entity count, getter, setter, variable index, variable name, and descriptor index.

**Scalar neighborhood facade types** — `ScalarNeighborhoodKind` enumerates
`Change`, `Swap`, `NearbyChange`, `NearbySwap`, `PillarChange`, `PillarSwap`,
and `RuinRecreate`. `ScalarNeighborhoodBindingError` is the shared fallible
leaf-construction error: `ConfigFamilyMismatch`, `MissingCapability`, or
`InvalidRuinBounds`. Direct static/dynamic facade constructors and compiler
lowering use this same validation surface.

**`VariableSlot<S, V, DM, IDM>` / `RuntimeModel<S, V, DM, IDM>`** —
`builder/context.rs`. `VariableSlot` variants are `Scalar`, `List`,
`DynamicScalar`, and `DynamicList`. `RuntimeModel::new(variables)` builds the
model published by macro/runtime assembly or binding code. Public builder
methods: `with_scalar_groups()`, `with_conflict_repairs()`,
`with_runtime_provider_registry()`, `with_candidate_metrics()`, and
`resolve_dynamic_descriptor_indexes(&SolutionDescriptor)`. Dynamic selector
compilation requires descriptor-resolved dynamic slots and returns a declaration
error with the slot diagnostic when a model contains unresolved dynamic slots.
Query methods
include `variables()`, `scalar_groups()`, `conflict_repairs()`,
`runtime_provider_registry()`, `candidate_metrics()`, `is_empty()`,
`has_scalar_variables()`, `has_list_variables()`, `has_dynamic_variables()`,
`has_dynamic_list_variables()`, `is_scalar_only()`, `dynamic_scalar_variables()`,
`dynamic_list_variables()`, and the scalar/list/grouped/repair capability
helpers used by runtime default construction and selector routing.

**`ListVariableSlot<S, V, DM, IDM>`** — typed list-variable access, stable
element-source identity, distance meters, route/savings hooks, owner
restrictions, construction ordering, and precedence metadata. Builder methods
are `with_element_owner_fn()`, `with_construction_element_order_key()`, and
`with_precedence_hooks()`; capability queries include `supports_clarke_wright`,
`supports_k_opt`, `supports_ruin`, and `supports_precedence_moves`.
`usize_element_source_key(&S, &usize) -> usize` is the canonical stable-key
function used by generated `usize` list models.

**Runtime candidate metrics** — `RuntimeCandidateMetric<S>` is the explicit
object-safe host boundary used only by configured `Sorted` and
`Probabilistic` leaf ordering. `measure(&S, &CandidateTraceIdentity) -> f64`
receives the same logical identity recorded by candidate tracing.
`RuntimeCandidateMetricBinding::new(name, Arc<dyn RuntimeCandidateMetric<S>>)`
rejects an empty name and exposes `name()` / `measure()`.
`RuntimeCandidateMetricRegistry::new(bindings)` rejects duplicate names and
`get(name)` resolves an immutable binding. All three types are crate-root
re-exports; the registry is attached with `RuntimeModel::with_candidate_metrics`.

**Runtime scalar access** — `builder::context` publicly exposes
`RuntimeScalarSlotId`, `ScalarAccessCapability`, `RuntimeScalarSlot<S>`, and
`RuntimeScalarEdit<S>`. A slot is `Static(ScalarVariableSlot<S>)` or
`Dynamic(DynamicScalarVariableSlot<S>)`; its public surface is `id()`,
`matches_target()`, `has_capability()`, and `is_dynamic()`. An edit owns the
slot plus public `entity_index` and `to_value` fields and exposes `id()`.

**`ScalarVariableSlot<S>`** — `builder/context.rs`. Canonical scalar-variable metadata used by the monomorphized runtime. The compact scalar `variable_index` is the generated getter/setter dispatch index; hook attachment, descriptor ordering, and user-facing target matching use descriptor index plus variable name, with the canonical entity type name kept for target matching and diagnostics. Getter, setter, and entity-local value sources receive the scalar variable index so selector hot paths do not need descriptor-erased access. In addition to value-source hooks it carries optional nearby hooks and scalar construction order-key hooks via builder-style methods:
- `with_candidate_values(for<'a> fn(&'a S, usize, usize) -> &'a [usize])` for bounded scalar value candidates
- `with_nearby_value_candidates(for<'a> fn(&'a S, usize, usize) -> &'a [usize])` for nearby scalar change
- `with_nearby_entity_candidates(for<'a> fn(&'a S, usize, usize) -> &'a [usize])` for nearby scalar swap
- `with_nearby_value_distance_meter(fn(&S, usize, usize, usize) -> Option<f64>)` to rank/filter nearby value candidates
- `with_nearby_entity_distance_meter(fn(&S, usize, usize, usize) -> Option<f64>)` to rank/filter nearby entity candidates
- `with_construction_entity_order_key(fn(&S, usize, usize) -> Option<i64>)` for decreasing or queue-style entity ordering
- `with_construction_value_order_key(fn(&S, usize, usize, usize) -> Option<i64>)` for weakest-fit, strongest-fit, or queue-style value ordering

The public function-pointer aliases used by this slot are `ScalarGetter<S>`,
`ScalarSetter<S>`, `ScalarCandidateValues<S>`,
`NearbyValueDistanceMeter<S>`, `NearbyEntityDistanceMeter<S>`,
`ConstructionEntityOrderKey<S>`, and `ConstructionValueOrderKey<S>`.

Runtime scalar construction resolves one canonical binding set per variable by
overlaying these runtime hooks onto descriptor-discovered scalar bindings by
descriptor index and variable name. Validation and execution use that
same resolved binding set. Construction order-key hooks are construction-only:
ordinary local-search change, pillar, and ruin/recreate selectors use canonical
bounded candidate order and do not reorder candidates from
`construction_value_order_key`.

**`ScalarGroup<S>` / `ScalarGroupBinding<S>` / `ScalarGroupMemberBinding<S>`** — `planning/scalar/*` and
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

`ScalarGroupBindingKind<S>` is `Candidates { candidate_provider }` or
`Assignment(ScalarAssignmentBinding<S>)`. `bind_scalar_groups(groups, slots)`
is the public typed binding helper used by macro/runtime assembly; it resolves
public `ScalarTarget` declarations against the supplied scalar slots.

`ScalarAssignmentRule<S>` is the public
`fn(&S, left_entity, left_value, right_entity, right_value) -> bool` adjacent
assignment-edge check used by assignment-backed groups.

`RepairLimits` exposes `max_matches_per_step`, `max_repairs_per_match`, and
`max_moves_per_step`. `RepairProvider<S>` is
`fn(&S, RepairLimits) -> Vec<RepairCandidate<S>>`; a
`ConflictRepair<S>` binds that provider to one scoring constraint name.
`ConflictRepairScalarEdit<S>` is the public static edit carrier accepted by
`ConflictRepairMove::new(reason, edits)`.

**Runtime compound-provider boundary** — `builder/context/provider.rs`.
`RuntimeProviderRegistry<S>` freezes schema-order provider declarations for the
compiled provider cursor. Native Rust `ScalarCandidateProvider<S>` and
`RepairProvider<S>` declarations remain function pointers: their original
`ScalarCandidate<S>` / `RepairCandidate<S>` vectors and typed `ScalarEdit<S>`
values pass directly through monomorphized normalization. Host integrations use
the separate object-safe `RuntimeHostCompoundProvider<S>` contract and
`RawProviderCandidate` / `RawProviderEdit` name payloads; structured resolution
failures cross `RuntimeHostProviderErrorBoundary` only on that host path.
Static and host sources share scheduling, legality, deduplication, reason-ID,
candidate ownership, and tabu semantics without adapting static Rust providers
through the host callback representation.

The module-level public contract also includes `RawProviderEdit`,
`RawProviderCandidate`, `ProviderReasonId`, `ProviderReasonArena`,
`RuntimeProviderLimits`, `RuntimeProviderHandle`, `ProviderResolutionError`,
`RuntimeScalarGroupProviderBinding`, `RuntimeConflictRepairProviderBinding`,
`StaticScalarGroupProviderBinding`, `StaticConflictRepairProviderBinding`,
`ResolvedProviderEdit`, `ResolvedProviderCandidate`,
`ProviderNormalizationState`, and `RuntimeProviderSlotResolver`. Only host
bindings contain `Arc<dyn RuntimeHostCompoundProvider<S>>`; static bindings
retain concrete function pointers and typed edits.

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

**Dynamic assignment group boundary** — `builder/context/scalar/group.rs` and
`solverforge-core::domain::DynamicScalarAssignmentMetadata<S>`. Dynamic
assignment registration is declarative and group-bound:
`ScalarGroupBinding::dynamic_assignment(...)` binds one resolved dynamic scalar
slot and one metadata object to one group. The metadata declares required,
capacity, ordering, sequence, and assignment-rule behavior; SolverForge owns
all assignment candidate generation, construction, selected-move ownership,
and grouped local search.

`GroupedScalarSelector` remains an implementation type required by the public
neighborhood graph, but direct construction is crate-private. Configuration is
the only supported selector entrypoint. Assignment stream cursors, move
options, and retained streaming commits are internal implementation details;
there is no dynamic phase, TLS lookup, direct cursor, or selector escape hatch.

**`IntraDistanceAdapter<T>`** — `builder/context.rs`. Newtype wrapping `T: CrossEntityDistanceMeter<S>`. Implements `ListPositionDistanceMeter<S>` by forwarding to `T::distance` with `src_entity_idx == dst_entity_idx`.

**`MimicRecorder`** — Shared state for recording/replaying entity selections. Methods: `new(id)`, `get_has_next()`, `get_recorded_entity()`, `reset()`.

## Phase Types

### Construction Heuristic

**`ConstructionHeuristicPhase<S, M, P, Fo>`** — Bounds: `P: EntityPlacer<S, M>`, `Fo: ConstructionForager<S, M>`. The phase opens one placer cursor and requests one live placement at each step; descriptor and grouped placers own any required live-refresh behavior inside that single cursor path. `with_mandatory_construction_completion()` lets mandatory construction slots finish even after ordinary construction limits have expired. Forager step selection is dispatched through the concrete `Fo` type; stock foragers provide prompt/control-aware selection without `dyn Any` routing.

Long-running construction uses the shared metadata-only one-second phase pulse.
Completed placement boundaries report it automatically, while bounded inner
candidate loops also poll it so one expensive placement cannot create a blind
period. Pulse events attach phase-local telemetry to the cumulative snapshot;
they do not clone or publish a solution and do not alter the shared solve clock.

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
- list-only route heuristics validate their required route-local and savings hook capabilities before phase build
- `element_owner_fn` is interpreted through one crate-private owner-restriction helper: no hook or hook-returned `None` leaves an element unrestricted, valid `Some(owner)` fixes the element to that owner, and invalid owners produce no legal placement
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

**`Placement<S, M, C>`** — one construction target plus a concrete cursor `C: MoveCursor<S, M>`. `entity_ref` remains public; methods expose `candidates()`, `candidates_mut()`, `with_keep_current_legal()`, `keep_current_legal()`, and ownership transfer through `take_move(CandidateId)`. It contains no placement-wide move vector.

`QueuedEntityPlacerCursor`, `QueuedPlacementCandidateCursor`, and
`SortedEntityPlacerCursor` are the public associated cursor carriers returned
by the stock placers. They keep one current placement/candidate stream and do
not expose an eager placement collection.

### Local Search

**`LocalSearchPhase<S, M, MS, A, Fo>`** — Bounds: `MS: MoveSelector<S, M>`, `A: Acceptor<S>`, `Fo: LocalSearchForager<S, M>`.

Local search uses the same shared one-second phase pulse. Completed step
boundaries report it automatically, and bounded inner candidate scans poll it
for prompt progress and interruption; publication is not gated on accumulating
a fixed-size candidate batch.

When runtime phase configuration is omitted, SolverForge always builds
state-aware construction. It appends the model-aware streaming local-search
default only when the top-level solver termination parses to an effective
finite policy (a time limit or a valid configured score/work criterion). No
termination, an empty termination object, or an otherwise invalid score-only
termination therefore remains construction-only; SolverForge does not invent a
binding-specific timeout. When it is appended, omitted local-search
configuration uses one capability table in canonical order. List slots with
explicit precedence metadata receive precedence repair followed by list
permute. Cross-position distance selects nearby change/swap; slots without it
receive the corresponding plain change/swap fallback (swap also requires set
access). Sublist change/swap and reverse are capability-gated. K-opt is nearby
when intra-position distance exists and otherwise uses the unbounded kernel;
both require sublist access. Every list slot receives list ruin through the
shared ruin access contract. Scalar slots with nearby value/entity sources
receive targeted nearby change/swap first, while every non-assignment-owned
scalar slot retains targeted plain change/swap after nearby coverage. Scalar
groups add grouped-scalar neighborhoods and registered repair providers add
compound conflict-repair neighborhoods. Every default leaf uses randomized
candidate order; multiple leaves use `StratifiedRandom` union order and a
single leaf uses `Sequential`. The selected stock acceptor/forager policy keeps
finite accepted-count horizons where applicable so search can improve under
short budgets. VND remains available only through explicit local-search config;
explicit configs own their acceptor, forager, selector, VND neighborhoods, and
union order exactly.

An explicit construction or local-search phase installs an internal
phase-relative termination overlay for its `TerminationConfig`. Its time and
work counters start at that phase boundary; score targets and unimproved limits
observe committed step scores at the same boundary; and the overlay is removed
before the next top-level phase runs. It does not publish extra callbacks or
snapshots. Mandatory omitted construction deliberately has no such overlay so
its required-completion stream remains governed by lifecycle control. The
overlay is local to the top-level runtime phase: `SolverScopeChildConfig` does
not propagate it into partitioned child solvers, which continue to inherit the
documented runtime control, remaining time, and in-phase limits only.

Assignment-owned scalar variables stay on the grouped scalar path. Default
plain scalar neighborhoods and default conflict-repair neighborhoods exclude
scalar slots covered by assignment-backed `ScalarGroup` declarations, and
explicit scalar selector targets that name an assignment-owned variable are
rejected with a diagnostic pointing at the owning grouped scalar selector.

Dynamic scalar variables participate in explicit local-search
`change_move_selector` phases through `DynamicScalarChangeMoveSelector` and
`DynamicScalarChangeMove`, plus `nearby_change_move_selector` and
`nearby_swap_move_selector` phases through the dynamic nearby selectors and
`DynamicScalarSwapMove`. Nearby selectors reject missing declared source
metadata before any callback pull. For declared sources, access remains lazy
per cursor row, respects source-consumption limits, and retains stable bounded
top-k ordering; an empty row supplies no replacement candidate rather than an
ordinary-candidate substitute, while an independently valid optional
unassignment remains available. The dynamic path uses the same score-director
before/after variable-change protocol as typed scalar moves after resolving
logical entity IDs to descriptor indexes. Dynamic list variables participate
through the same compiled list leaves as typed list variables: change, nearby
change, swap, nearby swap, permute, precedence, sublist change/swap, reverse,
k-opt, and ruin. Those leaves share the canonical list kernels, ownership and
precedence filtering, score-director change notifications, and pre-removal
intra-list destination coordinates.

Typed custom search is compiled into the solution, not loaded from a runtime
registry. A solution can declare `#[planning_solution(search = "path::to::search")]`.
The search function receives a `SearchContext<S, V, DM, IDM>`, calls
`ctx.defaults()`, and registers named phases with `.phase("name", |ctx| ...)`.
`SearchBuilder<S, V, DM, IDM, Extensions>` is the returned concrete declaration;
it also exposes `.partitioned_phase(...)` and `into_runtime_parts()`.
`SearchContext` resolves dynamic logical IDs to descriptor indexes before the
runtime compiler receives the model, so configured and custom-extension phases
use the same descriptor-index notifications. The configured entrypoint owns
graph compilation, preparation, and execution; an unresolved dynamic slot
fails declaration before the runner starts.
TOML can then order those compiled-in names with `[[phases]] type = "custom"
name = "..."`. Custom phases implement `CustomSearchPhase<S>` or use the
typed `local_search(selector, acceptor, forager)` helper. Generated code lowers
the result into concrete enums over known phase types; there is no
`Box<dyn Phase>` registry. `CustomSearchPhase::on_solver_terminal(...)` is an
optional matching terminal hook for custom runtime phases and defaults to a
no-op.

`RuntimeExtensionRegistry<S, V, DM, IDM>` is the recursive concrete registry
contract transferred with `Search`; its `RuntimeExtensionPolicy` is `Typed` or
`Dynamic`. `NoTypedExtensions` is the empty typed registry and
`NoDynamicExtensions` is the distinct empty host/dynamic registry that lets
compilation reject unsupported custom declarations. `CustomPhaseNode` and
`PartitionedPhaseNode` prepend typed builders, `CustomPhaseUnion` composes their
concrete phase results, and the uninhabited `NoRuntimeExtensionPhase` is their
zero-extension phase type.

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
| `GreatDelugeAcceptor<S>` | `S: PlanningSolution` | `water_level_increase_ratio` |
| `StepCountingHillClimbingAcceptor<S>` | `S: PlanningSolution` | `step_count_limit` |
| `DiversifiedLateAcceptanceAcceptor<S>` | `S: PlanningSolution` | `late_acceptance_size`, `tolerance` |
| `AnyAcceptor<S>` | `S: PlanningSolution` | Enum over all built-in acceptors; returned by `AcceptorBuilder::build()` |

`HardRegressionPolicy` is `TemperatureControlled` or
`NeverAcceptHardRegression`. `SimulatedAnnealingCalibration` exposes
`sample_size`, `target_acceptance_probability`, and `fallback_temperature`;
its default is used by `SimulatedAnnealingAcceptor::auto_calibrate(decay_rate)`.

### Exhaustive Search

**`ExhaustiveSearchPhase<Dec>`** — Bounds: `Dec: ExhaustiveSearchDecider<S, D>`.

**`ExplorationType`** — `DepthFirst`, `BreadthFirst`, `ScoreFirst`, `OptimisticBoundFirst`.

**`ExhaustiveSearchConfig`** — `{ exploration_type, node_limit, depth_limit, enable_pruning }`.

**`BounderType`** — `None` (default), `Simple`, or `FixedOffset`.

`ExhaustiveSearchPhase` is cooperative with solver lifecycle control: every explored node advances the phase step count and the frontier loop polls pause, cancel, time, and in-phase limits before applying the next partial assignment. Finite `node_limit` or `depth_limit` bounds that leave frontier work unexplored terminate as `TerminatedByConfig`; an exhausted frontier remains `Completed`.

**`ExhaustiveSearchNode<S>`** — Tree node: depth, score, optimistic_bound, descriptor/variable/entity/candidate indices, parent_index. A node can reconstruct its scalar assignment path from stored parent indices.

**`SimpleDecider<S, V, B>`** — Generic decider with values and optional bounder.

Score bounders: `SoftScoreBounder`, `FixedOffsetBounder<S>`, `()` (no-op).

### Partitioned Search

**`PartitionedSearchPhase<S, PD, Part, SDF, PF, CP>`** — Generic over partitioner, score director factory, phase factory, child phases. Child scopes inherit runtime control, environment mode, remaining time limit, in-phase limits, and deterministic child seeds, but retained-job publication stays on the parent scope. The runtime phase-relative termination overlay is not propagated into a child scope. Pause checkpoints are emitted only from the parent full-solution boundary; child pause/cancel/config termination outcomes prevent partition merge.

**`FunctionalPartitioner<S, PF, MF>`** — Closure-based partitioner.

**`ThreadCount`** — `Auto`, `Unlimited`, `Specific(usize)`. `PartitionedSearchPhase` solves child partitions sequentially when the resolved count is `1`, otherwise it installs a dedicated Rayon pool whose worker count matches the resolved value.

### Variable Neighborhood Descent

Variable Neighborhood Descent is an internal `local_search_type`, not a public
standalone phase API. Configured runtime solving reaches it through
the same compiled `RuntimeLocalSearch` runner as acceptor-forager search. It
scans a neighborhood to completion only while no
timeout, pause, or cancel is pending; interruption returns the last committed
incumbent and never applies a partial best move discovered during the abandoned
scan.

## Scope Hierarchy

### `ProgressCallback<S>` — `scope/solver.rs`

Sealed trait for zero-allocation callback dispatch. Implemented for `()` (no-op) and any `F: for<'a> Fn(SolverProgressRef<'a, S>) + Send + Sync`.

### `SolverScope<'t, S, D, ProgressCb = ()>`

Top-level scope for a retained solve. Holds score director, current score, best solution, best score, RNG, active timing, stats, runtime bridge, terminal reason, and termination state.

Key methods: `new(score_director)`, `new_with_callback(score_director, callback, terminate, runtime)`, `with_progress_callback(F) -> SolverScope<.., F>`, `with_runtime(runtime)`, `start_solving()`, `initialize_working_solution_as_best()`, `replace_working_solution_and_reinitialize(solution)`, `score_director()`, `working_solution()`, `mutate(...)`, `current_score()`, `best_score()`, `calculate_score()`, `update_best_solution()`, `report_progress()`, `report_best_solution()`, `pause_if_requested()`, `pause_timers()`, `resume_timers()`, `mark_cancelled()`, `mark_terminated_by_config()`, `is_terminate_early()`, `set_time_limit()`. The current implementation also owns the one-second phase progress pulse and tracks a working-solution revision for built-in descriptor-driven construction completion; committed mutation goes through `mutate(...)` (or the equivalent crate-private step boundary), which clears `current_score` and advances that revision exactly once. Speculative phase evaluation uses `Move::do_move`, the returned typed undo value, `Move::undo_move`, and `DirectorScoreState` snapshots to restore both solution values and committed score state after scoring a candidate. An internal phase-relative termination overlay records the best and last-improving committed scores only while an explicit runtime construction or local-search phase executes; it is neither a public `SolverScope` setting nor child-scope state. Internal prompt-control plumbing also exposes immutable `pending_control()` so built-in phases can abandon partial steps and unwind to runtime-owned boundaries before settling pause/cancel/config termination.

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

Static lifetime retained-job manager: `solve()` returns `(job_id, receiver)`;
`solve_with_qualified_candidate_trace_provenance()` uses the same lifecycle
while installing an externally validated trace attestation. Other methods are
`get_status()`, `get_telemetry_detail()`, `pause()`, `resume()`, `cancel()`,
`delete()`, `get_snapshot()`, `analyze_snapshot()`, and `active_job_count()`.
The retained lifecycle contract is expressed in neutral `job`, `snapshot`, and
`checkpoint` terminology. `pause()` settles at a runtime-owned safe boundary
and `resume()` continues from the exact in-process checkpoint. `delete()` hides
a terminal job immediately, but the slot itself is not reusable until the
solve worker has definitely exited. `MAX_JOBS = 16`.

### `SolverLifecycleState` / `SolverTerminalReason`

Lifecycle states: `Solving`, `PauseRequested`, `Paused`, `Completed`, `Cancelled`, `Failed`. Terminal reasons: `Completed`, `TerminatedByConfig`, `Cancelled`, `Failed`.

### `SolverStatus<Sc>`

Retained job summary from `get_status()`. Fields: `job_id`, `lifecycle_state`, `terminal_reason`, `checkpoint_available`, `event_sequence`, `latest_snapshot_revision`, `current_score`, `best_score`, `telemetry`. `checkpoint_available` means the runtime currently holds an exact resumable checkpoint for `resume()`. Analysis availability is separate from terminality: a job can expose retained snapshots while still solving or pausing.

### `SolverTelemetryDetail<Sc>`

Detached retained-job telemetry returned by `get_telemetry_detail()`. It contains the current `SolverStatus<Sc>` plus optional candidate-trace detail under one retained-publication lock: when detail is present, the status scores and `latest_snapshot_revision` identify the exact paired snapshot a caller may fetch and validate. Ordinary status, snapshot, and lifecycle-event payloads remain compact and never materialize the candidate trace.

### `SolverEvent<S>`

Lifecycle event stream for retained jobs. Variants: `Progress`, `BestSolution`, `PauseRequested`, `Paused`, `Resumed`, `Completed`, `Cancelled`, `Failed`. Each event carries metadata with job id, monotonic event sequence, lifecycle state, terminal reason, telemetry, scores, and optional snapshot revision. Event metadata is authoritative: for example, progress can report `PauseRequested` while a pause is still settling toward a `Paused` checkpoint, and once `pause()` is accepted the stream delivers `PauseRequested` before any later worker-side event already in `PauseRequested` state.

`SolverEventMetadata<Sc>` is that shared payload with public fields `job_id`,
`event_sequence`, `lifecycle_state`, `terminal_reason`, `telemetry`,
`current_score`, `best_score`, and `snapshot_revision`; `SolverEvent::metadata()`
returns it uniformly for every variant.

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

`ListCheapestInsertionPhase<S, E>` and `ListRegretInsertionPhase<S, E>` expose
`with_element_owner_fn(...)`, `with_element_order_key(...)`, and
`with_precedence_hooks(...)`. Owner hooks restrict candidate entities through
the shared owner-restriction relation. Element order hooks provide direct
construction ordering. Precedence hooks supply duration and successor metadata:
cheapest insertion orders unassigned elements by downstream criticality before
greedy insertion, while regret insertion uses the same metadata for
topological element ordering and regret tie-breaking.

All specialized list-construction public constructors require an explicit
`element_source_key`. It is the stable unique identity for a declared element,
its assigned representation, and precedence-successor values. Construction
binds the declared source once per solve and uses source indexes thereafter;
there is no equality/hash identity fallback. Duplicate declarations, unknown
assigned values, and duplicate assigned values fail before candidate work.
When multiple configured phases target a slot, they reuse that frozen
declaration binding but refresh current assignments before every phase, so no
phase can reinsert earlier work or reread declaration callbacks.

`ListClarkeWrightPhase<S, E>` preserves preassigned routes by filling only empty entities, computes savings per optional savings `metric_class` through savings `depot` and `distance`, and first assigns constructed routes through deterministic matching against savings `feasible` plus partial fixed-owner restrictions. The savings feasibility hook is a construction admissibility gate: stock CVRP rejects malformed owners/data/visit ids while leaving scoreable capacity and time-window violations to constraints. When route-to-owner matching is still short, Clarke-Wright completes construction with savings-distance cheapest insertion under the same savings feasibility hook. It uses route `set` only to commit the constructed assignment; it does not consume route-local distance or feasibility.
`ListKOptPhase<S, E>` is route-local polishing: route `get` reads the route, route `set` writes an accepted route, route `depot` supplies the owner depot, route `distance` scores reversals for that owner, and route `feasible` is the route-local commit gate. Stock CVRP route feasibility is strict for capacity and time windows, so k-opt does not commit hard-infeasible route-local improvements. It does not consume Clarke-Wright savings hooks.

## Real-Time Planning

**`SolverHandle<S>`** — Client-facing handle. `add_problem_change()`,
`add_problem_change_boxed()`, `terminate_early()`, `is_solving()`, and
`set_solving()`.

**`ProblemChangeReceiver<S>`** — Server-side receiver. `try_recv()`,
`drain_pending()`, `is_terminate_early_requested()`, `set_solving()`, and
`clear_terminate_early()`.

**`ProblemChangeResult`** — `Queued`, `SolverNotRunning`, `QueueFull`.

**`ClosureProblemChange<S, F>`** — Wraps `Fn(&mut dyn Director<S>)`.

**`BoxedProblemChange<S>`** — Type alias: `Box<dyn ProblemChange<S>>`.

## Solver & Convenience Functions

### `Solver<'t, P, T, S, D, ProgressCb = ()>`

Main solver struct. Drives phases and checks termination. `impl_solver!` macro generates `solve(self, score_director: D) -> SolveResult<S>` for phase tuples up to 8.

Builder methods: `new(phases)`, `with_termination(T)`, `with_terminate(&AtomicBool)`, `with_time_limit(Duration)`, `with_config(SolverConfig)`, `with_progress_callback<F>(F) -> Solver<.., F>`. The callback type transitions the `ProgressCb` parameter from `()` to the concrete closure type — no `Box<dyn Fn>` allocation.

**`NoTermination`** is the marker type used by `Solver::new(...)` before a
termination is configured.

**`MaybeTermination<S, D, ProgressCb = ()>`** is the public marker trait for
termination carriers used by `Solver`. It is implemented for `NoTermination`
and `Option<T>` where `T: Termination<S, D, ProgressCb>`. Methods:
`should_terminate(&SolverScope<...>)` and `install_inphase_limits(&mut SolverScope<...>)`.

### `SolveResult<S>`

`{ solution: S, current_score: Option<S::Score>, best_score: S::Score, terminal_reason: SolverTerminalReason, stats: SolverStats }`. Methods: `solution()`, `into_solution()`, `current_score()`, `best_score()`, `terminal_reason()`, `stats()`, `step_count()`, `moves_evaluated()`, `moves_accepted()`.

### `SolverStats` / `PhaseStats`

Aggregate and per-phase metrics: step count, moves generated, moves evaluated,
moves accepted, moves applied, score calculations, elapsed time, generation
time, evaluation time, acceptance rate, selector-level telemetry, and exact
`Throughput { count, elapsed }` views for generated/evaluated work.
Human-facing `moves/s` is derived only at log/console formatting edges.
`moves_generated` counts candidates actually yielded by a runtime cursor; it
does not count an unrequested logical tail. Selector `size()` and explicit full
cursor exhaustion cover logical neighborhood size and canonical order.

`SolverTelemetry` snapshots expose the same counters plus an optional
`PhaseTelemetry` snapshot identifying the active phase and its local elapsed,
step, move, score-calculation, generation-time, and evaluation-time counters;
not-doable,
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

`MoveTelemetry` aggregates the same lifecycle by `move_label`, including
score-improving/equal/worse, rejected-improving, applied-improving, and total
applied score improvement. `AppliedMoveTelemetry` is the bounded step-level
record: step/candidate indexes, per-step generated/evaluated/accepted/ignored
counts, before/after/delta scores, and hard-feasibility transition. They appear
as `SolverTelemetry::move_telemetry` and `applied_move_trace`.

### Candidate Trace Diagnostics — `stats`

Candidate tracing is opt-in through `SolverConfig::candidate_trace` and is
recorded at the engine candidate-pull boundary. `CANDIDATE_TRACE_FORMAT_VERSION`
is `3`. `CandidateTraceTelemetry` contains the immutable `header`, configured
`max_entries`, total pull count, retained ordered prefix, truncation flag,
`prefix_digest`, and `unencoded_identity_count`. `is_complete()` requires an
untruncated fully terminal prefix with a canonical identity for every retained
pull; `has_complete_execution_provenance()` and `provenance_status()` are
separate provenance checks.

`CandidatePullTelemetry` records global ordinal, `CandidateTraceSource`, phase
and step identity, optional selector index, source-local candidate index,
optional `CandidateTraceConstructionTarget`, optional logical identity, and the
ordered `CandidateTraceDisposition` transitions. Sources cover construction,
local search, VND, generic K-opt, and the specialized list construction/search
trial paths. Dispositions distinguish interruption, evaluation, not-doable,
hard/score-improvement rejection, acceptor rejection, forager loss, selection,
and application.

Logical identities are `CandidateTraceIdentity::Operation` or ordered
`Composite`. `CandidateTraceOperationIdentity` carries descriptor, optional
variable, operation token, and `CandidateTraceCoordinate` values;
`CandidateTraceCompositeIdentity` carries an operation token and child
identities. Coordinates are `Unsigned`, `Absent`, `Text`, or declared `Bytes`.
`CandidateTraceConstructionTarget` carries descriptor and entity indexes.

`CandidateTraceHeader` owns canonical configured input plus digest, the installed
`CandidateTraceExecutionPolicy`, the resolved `CandidateTracePhasePlan`, their
digests/completeness flags, and optional input/qualified provenance. Phase plans
contain sorted unique `CandidateTracePhaseAttribute` values and children;
policies and plans can be explicitly `known(...)` or `opaque(...)`.
`CandidateTraceDigest` is a stable non-cryptographic comparison checksum, while
`CandidateTraceExternalDigest::sha256([u8; 32])` transports caller-computed
cryptographic digests without traversing the model.

`CandidateTraceInputProvenance` carries schema, instance, initial-state,
optional core-tree/build digests, and a `CandidateTraceInputAttestation` naming
the external producer. Its status is `CandidateTraceInputProvenanceStatus::Absent`
or `ExternallyAttested`. `CandidateTraceProvenanceStatus` reports execution
policy, resolved-plan, input, and qualification state independently.

`QualifiedCandidateTraceRunProvenance::externally_attested(...)` requires all
five digests and a non-empty producer; `try_from_input(...)` validates an
existing provenance. `CandidateTraceQualificationStatus` is `NotRequested` or
`Qualified`; `CandidateTraceQualificationError` distinguishes a request that
was absent, an empty producer, and missing core-tree/build digests. A normal
trace with optional provenance is never silently upgraded. Candidate-trace
types are public through `solverforge_solver::stats`; only
`AppliedMoveTelemetry` and `MoveTelemetry` are additionally crate-root
re-exports.

### `runtime.rs`

Runtime helpers:

- `ListVariableMetadata<S, DM, IDM>` — list-variable metadata surfaced to
  macro-generated runtime code. Its public fields are cross/intra distance
  meters; optional route get/set, depot, distance, and feasibility callbacks;
  optional savings depot, metric-class, distance, and feasibility callbacks;
  and an optional partial fixed-owner callback. `new(...)` accepts every field
  except the owner callback, which starts absent and is attached with
  `with_element_owner_fn(...)`.
- `ListVariableEntity<S>` — list-variable accessors plus `HAS_LIST_VARIABLE`, `LIST_VARIABLE_NAME`, and `LIST_ELEMENT_SOURCE`
- `runtime/compiler/` — compiles one value-owned `RuntimeModel` into an immutable graph, prepares solve-owned sources, and runs every configured or default phase through `CompiledRuntimePhaseRunner`
- `PlanningModelSupport` — hidden support trait with no default impl; generated by
  `planning_model!` so solution derives can attach descriptor hooks,
  runtime scalar/list hooks, resolve list element owners, attach scalar groups,
  validate the manifest-backed model, and delegate list-shadow updates without
  proc-macro registries

Scalar-only, list-only, mixed, and zero-variable planning models target the same compiled runtime layer through `RuntimeModel`. Generic construction order is the descriptor-backed variable order emitted by the macros, and scalar runtime assembly does not depend on Rust module declaration order. Scalar construction is single-slot by default for non-assignment-owned slots; grouped scalar construction is explicit, named, and atomic. Assignment-owned scalar slots are constructed and searched only through their owning grouped scalar path. Specialized list algorithms are compiled nodes that call their existing kernels directly.

Reached source-backed list construction binds its declared source once and
validates the current assignment by stable source key before deciding that no
work remains. A valid fully assigned source records `SkippedNoWork`; duplicate
or undeclared assigned keys fail at that reached boundary. Unreached and
already-terminated construction nodes remain lazy and do not bind the source.

### Configured Run Boundary — `run.rs`, `runtime_build_error.rs`

`log_solve_start()` emits shape-specific startup telemetry:
list solves log `element_count`, scalar solves log average
`candidate_count`. Console formatting uses those fields to label startup scale
as `elements` or `candidates`.

`AnyTermination<S, D>`, `build_termination()`, and
`ChannelProgressCallback<S>` are public under `solverforge_solver::run`, not
crate-root re-exports. `AnyTermination` is the concrete config-dispatch enum
over no termination and the supported time/score/work combinations;
`build_termination()` returns it together with the effective time limit.
`ChannelProgressCallback` is the runtime-owned `ProgressCallback` adapter and
has no public constructor.

`LocalSearchPhase` emits `phase_start` with the current score after calculating
the starting local-search score, and emits `phase_end` with the best score. The
console layer renders phase-start scores when present; construction and
partitioned phase starts currently omit a score field.

`try_run_solver_with_config_and_search(...) -> RuntimeBuildResult<S>` is the one
public configured solve entrypoint used by macro-generated solving. It accepts
the solution/constraints/descriptor, entity counting and logging callbacks, a
`SolverRuntime<S>`, `SolverConfig`, fallback time limit, optional qualified
candidate-trace provenance, and a fallible builder for one typed `Search`
declaration. Graph compilation and solve-owned source preparation remain
private; there is no public graph or alternate phase-builder fallback.

`RuntimeBuildError` is `Declaration { message }`,
`Compilation { path, message }`, `Preparation { phase_index, message }`, or
`Execution { phase_index, message }`; `RuntimeBuildResult<T>` is its result
alias. This is the public error surface propagated by generated and host
bindings without exposing private compiler graph types.

The configured path updates all shadows before director initialization and
entity shadows before reinsertion, so the canonical solve remains on the
monomorphized score-director lifecycle.

## Architectural Notes

- **Zero-erasure native path.** Native move and selector carriers, phases,
  acceptors, foragers, terminations, and static Rust compound providers retain
  concrete types. The documented scorer-agnostic `&dyn Director` callbacks and
  dynamic/host boundaries below are the intentional erasure seams; host
  providers do not erase native providers.
- **Runtime selectors.** `runtime/compiler/selector_tree.rs` validates and
  freezes configured selector declarations against the resolved `RuntimeModel`;
  the executor lowers that immutable graph to the shared list/scalar kernels.
  Public concrete selectors live under `heuristic/selector/`, while
  `builder/selector/` owns internal grouped-scalar and compiled-composition
  execution types.
- **Grouped scalar is explicit.** Nullable scalar variables that must change together use declared scalar groups and compound scalar moves. The solver does not infer groups from unrelated nullable variables.
- **Compound repair is framework-owned.** Conflict repair providers produce domain edit hints, while the selector layer enforces limits, legality, not-doable filtering, hard-improvement filtering, telemetry, affected-entity reporting, and tabu identity.
- **Cartesian stays sequential.** Cartesian selectors compose exactly two child selectors over a preview state. They are not a general atomic grouped-search facility.
- **Projected scoring rows are never planning entities.** Streams created with `.project(...)` are scoring-only internal cache rows owned by scoring constraints. They are not surfaced in `RuntimeModel`, value ranges, construction heuristics, or move selectors.
- **Explicit descriptor boundary.** `descriptor/*` is the opt-in public scalar
  selector boundary described above. The immutable runtime compiler is the sole
  construction/configured-search entrypoint for scalar-only, list-only, mixed,
  and dynamic models; descriptor selectors remain standalone selector APIs and
  never create a second construction or configured-search compiler.
- **Function pointer storage.** Moves and selectors store index-aware `fn` pointers (e.g., `fn(&S, usize, usize) -> Option<V>`) instead of trait objects for solution access.
- **PhantomData<fn() -> T>** pattern used in all move types to avoid inheriting Clone/Send/Sync bounds from phantom type parameters.
- **SmallVec<[usize; 8]>** used in RuinMove and ListRuinMove for stack-allocated small ruin counts.
- **Tuple-based composition.** Phases and terminations compose via nested tuples with macro-generated impls, avoiding `Vec<Box<dyn Phase>>`.
- **Intentional `dyn` boundaries.** `DynDistanceMeter` and
  `DefaultPillarSelector` retain concrete meter/closure types but accept
  `&dyn Director<S>` at their scorer-agnostic callback seam. Dynamic scalar/list
  access and metadata, `RuntimeCandidateMetric`,
  `RuntimeHostCompoundProvider`, and the cold
  `RuntimeHostProviderErrorBoundary` are explicit host-language boundaries;
  static Rust providers never enter them.
- **`ProblemChange::apply` uses `&mut dyn Director<S>`** — intentional type erasure at the real-time planning boundary.
- **`SolverPanicPayload` owns `Box<dyn Any + Send>`** — cold foreign-runtime
  panic preservation, outside candidate generation and scoring.
- **Cursor-owned candidate lifetime.** Hot phases evaluate cursor candidates by stable ID, release losers immediately, and move the winner out exactly once. `MoveArena` remains the reusable-capacity owner for APIs and concrete composite storage that require it.
- **Neighborhood support modules stay private.** `list_support.rs`, `nearby_list_support.rs`, and `sublist_support.rs` exist only to share selected-entity snapshots, bounded stable top-k nearby ordering, and exact finite-selector counting. Public cursor hot loops for list and sublist neighborhoods remain explicit.
- **Canonical neighborhood tests live under subsystem trees.** Multi-file selector behavior for list, nearby-list, and sublist families is documented under `heuristic/selector/tests/`, while move legality stays under `heuristic/move/tests/`.
- **Rayon for parallelism.** Partitioned search uses rayon for CPU-bound parallel solving. `tokio::sync::mpsc` for solution streaming.
