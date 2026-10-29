# solverforge-solver WIREFRAME

Solver engine: phases, moves, selectors, acceptors, foragers, termination, and solver management.

**Location:** `crates/solverforge-solver/`
**Workspace Release:** `0.23.0`

## Dependencies

- `solverforge-core` (path) — Score types, domain traits, descriptors
- `solverforge-scoring` (path) — Director trait, constraint scoring
- `solverforge-config` (path) — SolverConfig, PhaseConfig, AcceptorConfig
- `tracing` (workspace) — Logging
- `rand` (workspace) — RNG (`SmallRng`, `StdRng`)
- `rayon` (workspace) — Parallel computation
- `smallvec` (workspace) — Stack-allocated small vectors
- `serde` (workspace) — Serialization
- `tokio` (sync feature) — `mpsc` channels for solution streaming

**Features:** `default` is an empty feature set.

## File Map

```
src/
├── lib.rs                               — Crate root; module declarations, re-exports
├── solver.rs                            — Solver struct, SolveResult
├── solver/session.rs                    — Crate-private SolveSession: Solver::solve as start/run/finish over one phase, able to switch directors and resume the phase
├── runtime.rs                           — List-variable metadata plus the sole immutable compiled runtime-graph entrypoint
├── runtime/compiler/                    — Immutable runtime-graph compiler, prepared runner, default policy, and executor kernels; every reached source-backed construction boundary validates stable element keys against the frozen declared stream before deciding whether work remains
├── runtime/compiler/executor/completion.rs — Structural mandatory-work gate and deferred best-solution publication
├── runtime/compiler/executor/runner/failure.rs — Cold propagation of configured-runtime execution failures to the public run boundary
├── runtime/compiler/executor/runner/problem_change.rs — Problem-change pass loop and per-pass runner state reset
├── runtime/provider_cursor.rs           — One lazy compound-provider cursor; static Rust providers retain typed candidates/function pointers, while host callbacks alone use raw named edits and object-safe dispatch
├── model_support.rs                     — Hidden `PlanningModelSupport` bridge implemented by `planning_model!` for model-owned scalar hook attachment, scalar group attachment, model/solution validation, and shadow updates
├── pinning.rs                           — Internal descriptor-backed pin checks for solver-generated moves and entity mutations
├── list_placement.rs                    — Private partial fixed-owner restriction helpers for list construction, ruin/recreate, Clarke-Wright, and list selectors; detects all-selected-elements-fixed-to-current so intra-owner reordering still streams while cross-owner moves are filtered
├── descriptor.rs                        — Re-exports descriptor bindings, selectors, move types, and internal construction/runtime helpers
├── descriptor/
│   ├── bindings.rs                      — Scalar-variable binding module root and public/internal re-exports
│   ├── bindings/lookup.rs               — Binding collection, target matching, and frontier-aware scalar work checks
│   ├── bindings/variable.rs             — VariableBinding and ResolvedVariableBinding metadata/value-source helpers
│   ├── move_types/*.rs                  — Descriptor move implementations split by move family
│   ├── selectors/dispatch/*.rs          — Descriptor selector dispatch build/type chunks
├── run.rs                               — log_solve_start(), try_run_solver_with_config_and_search(), and hidden try_run_weighted_solver_with_config_and_search(); crate-private configured termination and progress adapter
├── run/termination.rs                   — Configured termination parsing, build_director_termination(), and the execution-policy description
├── run/weight_modes.rs                  — Per-solve weight-mode choice and the director-switching solve loop
├── run_tests.rs                         — Tests
├── runtime_build_error.rs               — Public RuntimeBuildError and RuntimeBuildResult declaration/compiler/constraint-weight/preparation/execution boundary
├── constraint_weights.rs                — Crate-private validation of a solution's constraint weights at solve start and after problem changes
├── builder/
│   ├── mod.rs                           — Re-exports from all builder submodules
│   ├── acceptor.rs                      — AnyAcceptor<S> enum built from validated acceptor config
│   ├── acceptor/tests.rs                — Tests
│   ├── forager.rs                       — AnyForager<S> enum built from validated forager config
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
│   ├── selector.rs                      — Internal grouped-scalar leaf and indexed compiled-selector composition root
│   ├── selector/grouped_scalar.rs       — GroupedScalarSelector used as one compiled runtime leaf
│   ├── selector/types/composite.rs      — Indexed Limited/union/cartesian declaration arena with stable node IDs
│   └── selector/types/composite/        — Indexed cursor execution and retained leaf-state arena over concrete generic leaves
├── planning/                            — Public scalar targets/candidates/groups/assignment rules and conflict-repair declarations
├── stats.rs                             — Statistics, telemetry, and candidate-trace re-export root
├── stats/                               — Solver/phase stats, telemetry payloads, candidate traces, and qualified provenance
├── test_utils.rs                        — TestSolution, TestDirector, NQueens helpers
├── test_utils/alloc_counter.rs          — Per-thread counting allocator for allocation pins (tests only)
├── test_utils_tests.rs                  — Tests
│
├── heuristic/
│   ├── mod.rs                           — Re-exports from move/ and selector/
│   │
│   ├── move/
│   │   ├── mod.rs                       — Module declarations, re-exports
│   │   ├── traits.rs                    — Move<S> trait definition
│   │   ├── change.rs                    — ChangeMove<S, V>
│   │   ├── list_change.rs              — ListChangeMove<S, V>
│   │   ├── list_swap.rs                — ListSwapMove<S, V>
│   │   ├── list_permute.rs             — ListPermuteMove<S, V>; contiguous intra-list window permutation; undo carries the inverse permutation indexes and restores through the same ownership-transfer gather
│   │   ├── list_reverse.rs             — ListReverseMove<S, V>
│   │   ├── list_ruin.rs                — ListRuinMove<S, V>
│   │   ├── metadata.rs                 — MoveTabuSignature, scoped entity/value tabu tokens, exact move identities
│   │   ├── sublist_change.rs           — SublistChangeMove<S, V>
│   │   ├── sublist_swap.rs             — SublistSwapMove<S, V>
│   │   ├── segment_layout.rs           — Post-move segment coordinate derivation and reverse-identity helpers for sublist moves
│   │   ├── ruin.rs                      — RuinMove<S, V>
│   │   ├── k_opt.rs                     — KOptMove<S, V>, CutPoint
│   │   ├── k_opt_reconnection.rs       — KOptReconnection patterns
│   │   ├── k_opt_reconnection_tests.rs — Tests
│   │   ├── compound_scalar.rs          — CompoundScalarMove<S> for atomic multi-scalar edits with exact undo, tabu identity, and affected-entity reporting
│   │   ├── dynamic_scalar_change.rs    — DynamicScalarChangeMove<S> with `Option<usize>` undo over a descriptor-resolved DynamicScalarVariableSlot<S>
│   │   ├── dynamic_scalar_swap.rs      — DynamicScalarSwapMove<S> over descriptor-resolved DynamicScalarVariableSlot<S>
│   │   ├── dynamic_list_change.rs      — DynamicListChangeMove<S> over descriptor-resolved DynamicListVariableSlot<S>
│   │   ├── runtime_compound.rs         — RuntimeCompoundMove<S> and RuntimeCompoundMoveKind for frozen provider candidates
│   │   ├── composite.rs                — SequentialCompositeMove<S, M> and the public borrowed SequentialCompositeMoveRef
│   │   ├── composite/preview.rs         — Internal owned preview director for read-only sequential checks and ordinary Cartesian selectors
│   │   ├── list_union.rs               — ListMoveUnion<S, V> enum
│   │   ├── list_multi_swap.rs          — ListMultiSwapMove<S, V> for independent same-step intra-list swaps
│   │   ├── list_kernel/                — Shared typed/dynamic list mutation kernels used by public moves and the compiled executor
│   │   └── tests/                       — Additional test modules
│   │       ├── mod.rs
│   │       ├── change.rs
│   │       ├── compound_scalar.rs
│   │       ├── list_multi_swap.rs
│   │       ├── list_telemetry.rs
│   │       ├── list_change.rs
│   │       ├── list_swap.rs
│   │       ├── list_permute.rs
│   │       ├── list_reverse.rs
│   │       ├── list_ruin.rs (+ list_ruin/transfer.rs zero-clone ownership-transfer contract)
│   │       ├── ruin.rs
│   │       ├── sublist_change.rs
│   │       ├── sublist_swap.rs
│   │       └── k_opt.rs
│   │
│   └── selector/
│       ├── mod.rs                       — Re-exports
│       ├── entity.rs                    — EntitySelector trait, EntityReference, FromSolutionEntitySelector
│       ├── value_selector.rs              — ValueSelector trait, StaticValueSelector
│       ├── move_selector.rs             — Re-exports for the move_selector/ files
│       ├── move_selector/*.rs           — borrowed.rs candidate cursor, arena.rs candidate store and arena cursor, unit_resource.rs unit resource cursor, iter.rs adapter, and change.rs selector implementation chunks
│       ├── dynamic_scalar_nearby_change.rs — Fallible DynamicScalarNearbyChangeMoveSelector<S> facade over the canonical nearby-value leaf
│       ├── dynamic_scalar_nearby_swap.rs — Fallible DynamicScalarNearbySwapMoveSelector<S> facade over the canonical nearby-entity leaf
│       ├── scalar_neighborhood/          — One frozen RuntimeScalarSlot leaf kernel, direct change/swap semantics, explicit ruin/recreate stream state, and thin static/dynamic move facades
│       ├── dynamic_list_change.rs       — DynamicListChangeMoveSelector<S> for explicit unrestricted dynamic list-change phases
│       ├── list_change.rs              — ListChangeMoveSelector<S, V, ES>
│       ├── list_support.rs             — Private selected-entity snapshots and exact list-neighborhood counting
│       ├── list_swap.rs                — ListSwapMoveSelector<S, V, ES>
│       ├── list_permute.rs             — ListPermuteMoveSelector<S, V, ES>; cursor-backed contiguous-window permutation
│       ├── list_precedence.rs          — ListPrecedenceMoveSelector<S, V, ES>; cursor-backed critical-path, singleton critical-node, and critical-sublist moves for list variables with plain precedence hooks
│       ├── precedence_route.rs         — Internal precedence route graph and cycle checks shared by list precedence neighborhoods
│       ├── precedence_route/delta.rs   — Route-after-move builders, edge diffs, and route-delta cycle search
│       ├── list_ruin.rs                — ListRuinMoveSelector<S, V>
│       ├── sublist_change.rs           — SublistChangeMoveSelector<S, V, ES>
│       ├── sublist_support.rs          — Private sublist segment enumeration and exact counting helpers
│       ├── sublist_swap.rs             — SublistSwapMoveSelector<S, V, ES>
│       ├── pillar.rs                    — Pillar and SubPillarConfig: pillar groups the runtime pillar cursors read through collect_pillar_groups
│       ├── pillar_support.rs            — Deterministic pillar grouping, legal-domain intersection, and mutual swap-compatibility helpers
│       ├── ruin.rs                      — RuinMoveSelector<S, V>, RuinVariableAccess<S, V>
│       ├── seed.rs                      — Scoped deterministic selector seed derivation from SolverConfig random_seed
│       ├── entity_tests.rs              — Tests
│       ├── value_selector_tests.rs     — Tests
│       ├── nearby_list_change.rs       — CrossEntityDistanceMeter trait, NearbyListChangeMoveSelector
│       ├── nearby_list_support.rs      — Private selected-entity snapshots and bounded stable top-k nearby candidate ordering
│       ├── nearby_list_swap.rs         — NearbyListSwapMoveSelector
│       ├── nearby_support.rs           — Shared nearest-candidate ordering and bounded stable top-k helpers for nearby neighborhoods
│       ├── list_kernel/                — Shared list candidate enumeration/emission kernels, including precedence and k-opt
│       ├── decorator/
│       │   ├── mod.rs                   — Re-exports
│       │   ├── cartesian_product.rs    — CartesianProductCursor<S, M>, CartesianProductSelector<S, M, Left, Right>
│       │   ├── cartesian_product/cursor.rs — CartesianProductCursor rows, pair scratch, and MoveCursor impl
│       │   ├── cartesian_product/tests.rs — Tests
│       │   ├── filtering.rs            — FilteringMoveSelector<S, M, Inner>
│       │   ├── filtering/tests.rs      — Tests
│       │   ├── limited.rs              — Shared candidate-limit budget with test-only ordinary cursor adapter
│       │   ├── limited/tests.rs        — Tests
│       │   ├── vec_union.rs            — Union child context, union weights, and the VecUnionMoveCursor / ResourceVecUnionMoveCursor the compiled union drives
│       │   ├── vec_union/scheduler.rs   — Canonical union scheduler shared by ordinary and indexed composition cursors
│       │   ├── vec_union/tests.rs      — Tests
│       │   ├── test_utils.rs           — Test helpers
│       │   ├── test_utils_tests.rs     — Test helper tests
│       │   └── {probability,shuffling,sorting,union}/tests.rs — Test-only coverage for ordering and union semantics
│       ├── k_opt/
│       │   ├── mod.rs                   — Re-exports
│       │   ├── config.rs               — KOptConfig
│       │   ├── iterators.rs            — count_cut_combinations(), cut_combination_at(), binomial()
│       │   ├── distance_meter.rs       — ListPositionDistanceMeter trait, DefaultDistanceMeter
│       │   ├── distance.rs             — ListPositionDistanceMeter and DefaultDistanceMeter mirror used by the k-opt module split
│       │   ├── selector.rs             — KOptMoveSelector<S, V, ES>
│       │   └── tests.rs                — Tests
│       └── tests/
│           ├── mod.rs
│           ├── k_opt.rs
│           ├── list_neighborhood.rs
│           ├── list_permute.rs
│           ├── list_precedence.rs
│           ├── list_ruin.rs
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
│   │   ├── decision.rs                  — Shared baseline/tie-breaking helpers for construction choice resolution
│   │   ├── evaluation.rs                — Trial-move evaluation via typed move undo and score-state snapshots
│   │   ├── frontier.rs                  — Revision-scoped ConstructionFrontier shared by generic scalar and list work
│   │   ├── phase.rs                     — ConstructionHeuristicPhase<S, M, P, Fo>
│   │   ├── phase/*.rs                   — Construction phase type and selection helpers
│   │   ├── phase/tests.rs               — Construction phase test root with support, selection, and lifecycle chunks
│   │   ├── forager.rs                   — ConstructionChoice enum, ConstructionForager trait, FirstFitForager, and crate-private BestFitForager/WeakestFitForager/StrongestFitForager
│   │   ├── forager_impl.rs              — Stock construction forager strategy implementations
│   │   ├── forager_step.rs              — Step-aware stock construction selection with telemetry and prompt/control polling
│   │   ├── forager_step/strength.rs     — Weakest-fit and strongest-fit selection
│   │   ├── forager/tests.rs             — Tests
│   │   ├── placer.rs                    — EntityPlacer and EntityPlacerCursor traits, Placement, ConstructionTarget
│   │   ├── placer/queued.rs             — QueuedEntityPlacer and its single-path streaming candidate cursor with bounded live-candidate storage
│   │   ├── placer/tests.rs              — Tests
│   │   ├── slot.rs                      — ConstructionSlotId, exact-keyed ConstructionGroupSlotId, ConstructionGroupSlotKey, and ConstructionListElementId for construction frontier tracking
│   │   ├── runtime_slots.rs             — Canonical scalar/list/mixed runtime-slot construction root
│   │   ├── runtime_slots/*.rs           — Global placement, move, and per-slot construction chunks
│   │   ├── telemetry.rs                 — Shared construction lifecycle events, candidate counters, and bounded progress polling
│   │   ├── grouped_scalar/mod.rs        — Atomic grouped scalar construction module root over declared ScalarGroup candidates and assignment groups bound to runtime scalar slots
│   │   ├── grouped_scalar/assignment_candidate.rs — Assignment move options, required assignment moves, capacity-conflict moves, reassignment moves, and remaining-required telemetry
│   │   ├── grouped_scalar/assignment_candidate/key.rs — Internal order-independent exact edit identity with inline short keys; no per-yield key sorting
│   │   ├── grouped_scalar/assignment_block.rs — Required-assignment block planning helpers
│   │   ├── grouped_scalar/assignment_cycle.rs — Bounded augmenting cycle and ejection/reinsert move construction
│   │   ├── grouped_scalar/assignment_entity.rs — Entity-local required, optional, capacity, and reassignment move construction
│   │   ├── grouped_scalar/assignment_family.rs — Shared assignment candidate-family deduplication
│   │   ├── grouped_scalar/assignment_pair.rs — Deterministic bounded pair, rematch, and sequence-window move generation
│   │   ├── grouped_scalar/assignment_path.rs — Bounded augmenting-path move construction for required and optional scalar assignments
│   │   ├── grouped_scalar/assignment_path_overlay.rs — Sparse reversible augmenting-path and required-batch discovery state
│   │   ├── grouped_scalar/assignment_required_batch.rs — Dense hard-first required assignment allocation
│   │   ├── grouped_scalar/assignment_state.rs — Committed assignment occupancy, capacity, and conflict bookkeeping
│   │   ├── grouped_scalar/assignment_overlay.rs — Sparse final-edit capacity and sequence-rule validation without mutating assignment indexes
│   │   ├── grouped_scalar/assignment_pressure.rs — Current-value and sequence-pressure candidate ordering
│   │   ├── grouped_scalar/assignment_stream.rs — Cursor-backed assignment move streaming for construction and selectors
│   │   ├── grouped_scalar/assignment_value_coordinates.rs — Lazy rotated pair and oriented-triple traversal without eager Cartesian products
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
│   │   ├── evaluation.rs                — Trial kernel (run_trial), candidate accounting (account_trial), and hard-delta classification
│   │   ├── phase.rs                     — LocalSearchPhase<S, M, MS, A, Fo>
│   │   ├── phase/candidates.rs          — Step candidate state and the parallel window loop
│   │   ├── phase/decision.rs            — Generated-candidate records and in-order candidate consumption for windows
│   │   ├── phase/evaluator.rs           — Step evaluation strategy, selected once per phase: sequential loop or evaluator pool
│   │   ├── phase/sequential.rs          — One-evaluator candidate loop
│   │   ├── phase/step.rs                — One step: candidate selection and commit
│   │   ├── phase/tests.rs               — Tests
│   │   ├── forager.rs                   — LocalSearchForager trait, AcceptedCountForager, FirstAcceptedForager, BestScoreForager, re-exports
│   │   ├── forager/any_tests.rs         — AnyForager dispatch tests
│   │   ├── forager/improving.rs        — FirstBestScoreImprovingForager, FirstLastStepScoreImprovingForager
│   │   ├── forager/tests.rs             — Tests
│   │   ├── parallel/mod.rs              — Parallel evaluation module declarations
│   │   ├── parallel/budget.rs           — Remaining in-phase evaluation budget for window sizing
│   │   ├── parallel/config.rs           — move_thread_count resolution to an evaluator count
│   │   ├── parallel/pool.rs             — EvaluatorPool: evaluator replicas, sync, and post-commit advance
│   │   ├── parallel/window.rs           — Window evaluation across the main director and replicas
│   │   ├── parallel/tests/              — Pool ceiling, window sizing, trajectory parity, and allocation-free window tests
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
│   │   ├── bounder.rs                   — ScoreBounder trait
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
│   │   ├── phase_tests.rs              — Tests
│   │   └── tests/problem_change.rs     — Children yield to a queued problem change without merging
│   └── localsearch/vnd/
│       ├── mod.rs                       — Internal VND module declarations
│       ├── phase.rs                     — Shared VND solve loop called by the compiled runner
│       ├── telemetry.rs                 — Internal VND selector-label helpers using the shared phase progress pulse
│       └── tests.rs                     — Internal VND tests
│
├── manager/
│   ├── mod.rs                           — Re-exports
│   ├── solver_manager.rs               — Re-exports retained lifecycle manager surface
│   ├── solver_manager/types.rs         — SolverLifecycleState, SolverTerminalReason, SolverStatus, SolverTelemetryDetail, SolverEventMetadata, SolverEvent, snapshots, and SolverManagerError
│   ├── solver_manager/runtime.rs       — SolverRuntime retained lifecycle publisher and SolverPanicPayload
│   ├── solver_manager/runtime/pause.rs — Pause settlement with or without a public snapshot according to structural completion
│   ├── solver_manager/runtime/problem_change.rs — Runtime-side problem-change bit decode and FIFO drain/close
│   ├── solver_manager/problem_change.rs — ProblemChangeId, SolverManager::add_problem_change, per-job change queue
│   ├── solver_manager/slot.rs          — Internal retained-job slots and snapshot records
│   ├── solver_manager/manager.rs       — MAX_JOBS, Solvable trait, SolverManager
│   ├── solution_manager.rs             — analyze() free fn, Analyzable trait, ScoreAnalysis, ConstraintAnalysis
│   ├── phase_factory/
│   │   ├── mod.rs                       — Re-exports
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
│   │   ├── list_clarke_wright/kernel.rs — Canonical savings merge and route assignment (savings.rs generates and sorts savings) shared by public/static/dynamic adapters
│   │   ├── list_clarke_wright/tests.rs — Tests
│   │   ├── list_clarke_wright/owner_assignment.rs — Owner-specific route assignment, buffered-merge publication on interruption, and preservation helpers
│   │   ├── list_clarke_wright/route_state.rs — Route state and merge bookkeeping
│   │   ├── list_clarke_wright/savings.rs — Metric-class savings computation
│   │   ├── list_clarke_wright/tests/metric_class.rs — Shared metric-class and owner-feasibility tests
│   │   ├── list_clarke_wright/tests/owner_binding.rs — Owner-specific hook binding tests
│   │   ├── list_k_opt.rs               — ListKOptPhase
│   ├── mod_tests_integration.rs        — Integration test module declarations
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
│   ├── solver/progress.rs               — SolverProgressRef, SolverProgressKind, SolverLifecycleState status, ProgressCallback dispatch, and the shared phase progress pulse
│   ├── solver/scope_core.rs             — Core SolverScope construction, runtime publication, lifecycle control, mutation, and child-scope helpers
│   ├── solver/phase_budget.rs           — PhaseBudget and scoped phase termination for child phases
│   ├── solver/scope_progress.rs         — SolverScope score/best-solution/progress/stat reporting helpers
│   ├── solver/scope_trace.rs            — SolverScope candidate-trace recording
│   ├── solver/problem_change.rs         — Pending problem-change control, queue drain/close, and batch application
│   ├── solver/weight_mode.rs            — Crate-private weight-mode control: switch detection after a change and moving the scope onto a rebuilt director
│   ├── phase.rs                         — PhaseScope<'t, 'a, S, D, BestCb = ()>
│   ├── step.rs                          — StepScope<'t, 'a, 'b, S, D, BestCb = ()>
│   ├── tests.rs                         — Tests
│   ├── tests/problem_change.rs          — Problem-change control priority, child yield, and application tests
│   └── tests/weight_mode.rs             — Weight-mode switch requests, deferred rescore, and unweighted-only failure
│
├── termination/
│   ├── mod.rs                           — Termination<S, D, BestCb = ()> trait, re-exports
│   ├── time.rs                          — TimeTermination
│   ├── step_count.rs                    — StepCountTermination
│   ├── best_score.rs                    — BestScoreTermination<Sc>
│   ├── unimproved.rs                    — UnimprovedStepCountTermination<S>, UnimprovedTimeTermination<S>
│   ├── composite.rs                     — OrTermination<T, S, D> (tuple impls up to 8)
│   └── tests.rs                         — Tests
│
└── realtime/
    ├── mod.rs                           — Re-exports
    ├── problem_change.rs               — ProblemChange trait, ClosureProblemChange
    └── problem_change_tests.rs         — Tests
```

## Public Surface

Every supported item is exported once, at the crate root. Implementation
modules are private, so an item has exactly one path:
`solverforge_solver::Name`. Applications reach these items through the
`solverforge` facade.

**Model and search composition:** `ListVariableSlot`, `RuntimeModel`, `VariableSlot`, `bind_scalar_groups`, `ScalarGroupBinding`, `ValueSource`, `ScalarVariableSlot`, `CustomSearchPhase`, `local_search`, `Search`, `SearchContext`, `PlanningModelSupport`, `ConflictRepair`, `RepairCandidate`, `RepairLimits`, `RepairProvider`, `ScalarAssignmentRule`, `ScalarCandidate`, `ScalarCandidateProvider`, `ScalarEdit`, `ScalarGroup`, `ScalarGroupLimits`, `ScalarTarget`, `ListVariableEntity`, `ListVariableMetadata`

**Moves and move streams:** `SequentialCompositeMoveRef`, `MoveTabuScope`, `MoveTabuSignature`, `MoveUndoIdentity`, `ScopedEntityTabuToken`, `ScopedValueTabuToken`, `Move`, `MoveAffectedEntity`, `DefaultDistanceMeter`, `CandidateId`, `MoveCandidateRef`, `MoveCandidateUndo`, `MoveCursor`, `MoveSelector`, `MoveSelectorIter`, `MoveStreamContext`, `CrossEntityDistanceMeter`, `DefaultCrossEntityDistanceMeter`

**Phases:** `ScoreBounder`, `ExhaustiveSearchConfig`, `ExhaustiveSearchDecider`, `SimpleDecider`, `ExplorationType`, `ExhaustiveSearchNode`, `ExhaustiveSearchPhase`, `Acceptor`, `ForagerDecision`, `LocalSearchForager`, `ChildPhases`, `PartitionedSearchConfig`, `FunctionalPartitioner`, `SolutionPartitioner`, `ThreadCount`, `PartitionedSearchPhase`, `Phase`

**Solver scope and progress:** `PhaseScope`, `ProgressCallback`, `SolverProgressKind`, `SolverProgressRef`, `SolverScope`, `StepScope`

**Solver management and analysis:** `analyze`, `Analyzable`, `ConstraintAnalysis`, `ScoreAnalysis`, `Solvable`, `SolverManager`, `ProblemChangeId`, `SolverPanicPayload`, `SolverRuntime`, `SolverEvent`, `SolverEventMetadata`, `SolverLifecycleState`, `SolverManagerError`, `SolverSnapshot`, `SolverSnapshotAnalysis`, `SolverStatus`, `SolverTelemetryDetail`, `SolverTerminalReason`, `ClosureProblemChange`, `ProblemChange`

**Configured run boundary:** `log_solve_start`, `try_run_solver_with_config_and_search`, `try_run_weighted_solver_with_config_and_search`, `RuntimeBuildError`, `RuntimeBuildResult`

**Statistics and candidate traces:** `CandidateTraceCompositeIdentity`, `CandidateTraceConstructionTarget`, `CandidateTraceCoordinate`, `CandidateTraceIdentity`, `CandidateTraceOperationIdentity`, `CandidateTraceExecutionPolicy`, `CandidateTracePhaseAttribute`, `CandidateTracePhasePlan`, `CandidateTraceExternalDigest`, `CandidateTraceInputAttestation`, `CandidateTraceInputProvenance`, `CandidateTraceInputProvenanceStatus`, `CandidateTraceProvenanceStatus`, `CandidatePullTelemetry`, `CandidateTraceDisposition`, `CandidateTraceSource`, `CandidateTraceDigest`, `CandidateTraceHeader`, `CandidateTraceTelemetry`, `CANDIDATE_TRACE_FORMAT_VERSION`, `CandidateTraceQualificationError`, `CandidateTraceQualificationStatus`, `QualifiedCandidateTraceRunProvenance`, `PhaseStats`, `SolverStats`, `AppliedMoveTelemetry`, `MoveTelemetry`, `PhaseTelemetry`, `SelectorTelemetry`, `SolverTelemetry`, `Throughput`

### Hidden modules

- `__bench` (`#[doc(hidden)]`): the concrete selectors, moves, placers and the construction phase that `benches/selector_cursor_gate.rs` drives directly. No compatibility promise covers them: `ChangeMove`, `ListChangeMove`, `CartesianProductSelector`, `FilteringMoveSelector`, `FromSolutionEntitySelector`, `KOptConfig`, `KOptMoveSelector`, `ListChangeMoveSelector`, `ListSwapMoveSelector`, `ChangeMoveSelector`, `NearbyListChangeMoveSelector`, `NearbyListSwapMoveSelector`, `RuinMoveSelector`, `RuinVariableAccess`, `SublistChangeMoveSelector`, `SublistSwapMoveSelector`, `StaticValueSelector`, `FirstFitForager`, `ConstructionHeuristicPhase`, `EntityPlacer`, `EntityPlacerCursor`, `QueuedEntityPlacer`.
- `__private` (`#[doc(hidden)]`): the host-provider contracts that `solverforge-bridge` re-exports for host-language bindings: `RuntimeCandidateMetric`, `RuntimeCandidateMetricBinding`, `RuntimeCandidateMetricRegistry`, `RuntimeProviderRegistry`, `ProviderResolutionError`, `RawProviderCandidate`, `RawProviderEdit`, `RuntimeConflictRepairProviderBinding`, `RuntimeHostCompoundProvider`, `RuntimeHostProviderErrorBoundary`, `RuntimeProviderLimits`, `RuntimeScalarGroupProviderBinding`, `NoDynamicExtensions`.

Types that appear only inside public signatures, such as the concrete
builder types `SearchContext::defaults` and `local_search` return, stay
unnameable: callers hold them through `impl Trait` or inference.

## Core Traits

### `Move<S: PlanningSolution>` — `traits.rs`

Requires: `Send + Sync + Debug`.

| Item | Signature |
|------|-----------|
| `type Undo` | `Send` |
| `is_doable` | `fn<D: Director<S>>(&self, score_director: &D) -> bool` |
| `is_doable_with_working_preview` | `fn<D: Director<S>>(&self, score_director: &mut D) -> bool` (default `is_doable`) — doability that may run an explicit working preview on the live director (apply earlier legs, check later legs, undo, restore score state before returning); sequential composites override it and the phase trial path calls it |
| `do_move` | `fn<D: Director<S>>(&self, score_director: &mut D) -> Self::Undo` |
| `undo_move` | `fn<D: Director<S>>(&self, score_director: &mut D, undo: Self::Undo)` |
| `descriptor_index` | `fn(&self) -> usize` |
| `entity_indices` | `fn(&self) -> &[usize]` |
| `variable_name` | `fn(&self) -> &str` |
| `telemetry_label` | `fn(&self) -> &'static str` (default `"move"`) |
| `requires_hard_improvement` | `fn(&self) -> bool` |
| `requires_score_improvement` | `fn(&self) -> bool` (default `false`) |
| `tabu_signature` | `fn<D: Director<S>>(&self, score_director: &D) -> Cow<'_, MoveTabuSignature>` — moves that own a prebuilt signature lend it (`Cow::Borrowed`); wrappers forward it unchanged |
| `candidate_trace_identity` | `fn(&self) -> Option<CandidateTraceIdentity>` (default `None`) |
| `for_each_affected_entity` | `fn<V: FnMut(MoveAffectedEntity<'_>)>(&self, visitor: &mut V)` — concrete visitor, monomorphized; implementations must report every mutated entity, including both pillar halves for pillar swaps |

Descriptor pillar swaps cache the deduplicated affected rows at construction,
preserving left-then-right declaration order. Both typed and descriptor swap
visitors report both pillars even though `entity_indices()` retains the primary
left slice; descriptor move unions forward the complete visitor to pin checks.

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
| `defers_initial_best_solution_publication` | `fn(&self) -> bool` (default `false`) |
| `on_solver_terminal` | `fn(&mut self, solver_scope: &mut SolverScope<'_, S, D, ProgressCb>)` (default no-op) |

All concrete phase types implement `Phase<S, D, ProgressCb>` for all `ProgressCb: ProgressCallback<S>`. Compiled runtime phases report whether their graph contains mandatory planning work through `defers_initial_best_solution_publication`; the solver then withholds public best-solution events until the compiled runtime proves that work complete. The solver invokes `on_solver_terminal` once for every configured top-level phase after the phase loop and before final statistics are taken, including a phase skipped by cancellation or configured termination. The solver drives exactly one phase; the compiled runtime phase composes configured phases internally and propagates both capabilities to its active children.

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
| `iter_moves` | `fn<'a, 'd, D: Director<S>>(&'a self, score_director: &'d D) -> MoveSelectorIter<'d, S, M, Self::Cursor<'a>>` |
| `size` | `fn<D: Director<S>>(&self, score_director: &D) -> usize` |
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
cursor-native loop state instead of prebuilding full move vectors.
`next_candidate_with_control()` lets expensive cursors poll solve control during
generation; because `None` can mean either interruption or exhaustion, callers
recheck the same control signal before resolving an exhausted scan. The cursor
contract also provides `release_candidate()`, `apply_owned_candidate()`,
`next_owned_candidate()`, `next_owned_candidate_matching()`, and
`next_owned_candidate_inspected()` so phases can end candidate residency
promptly, leaf cursors can fuse owned filtering with generation, and Cartesian
composition can transfer one inspected child without first storing it twice.

Ordinary cursor pulls lend `&S` through `next_candidate(solution)`,
`next_candidate_with_control(solution, should_stop)`, `next_owned_candidate(solution)`
and `next_owned_candidate_inspected(solution, inspect)`. Inside the crate,
compiled cursors pull through a resource boundary that takes the mutable
director instead of `&S`. Compiled phases
lend that director through unions and limits so Cartesian discovery can apply and
undo preview moves without retaining a solution borrow between pulls. Open-time
derived indexes and metric orders retain their own activation contracts.

`MoveCandidateRef<'a, S, M>` is either a borrowed move or a borrowable
two-child sequential composite; `MoveCandidateUndo<U>` mirrors those two
shapes for exact rollback. A `MoveSelector` implementation owns the store
behind its stable IDs; candidates leave it only by `take_candidate`, so there
is no eager move-collection path.

Compiled Limited/Union/Cartesian composition owns one immutable indexed declaration
arena per execution and transfers its retained leaf-state arena into a cursor and
back on drop. Structural edges use stable node IDs, not recursively owned trees;
union nodes use the canonical scheduler, and Cartesian right nodes open only after
left preview application on the live director. Each pull snapshots score state,
applies its row's left move, generates/checks the right candidate, then undoes and
restores before returning. Nested rows compose the same transaction; no compiled
Cartesian cursor owns a cloned preview solution. Flat extensions remain concrete
generic types, including non-Clone declaration/state types. This internal
representation does not change the public `MoveSelector` extension contract or
configured selector syntax.

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

The stock foragers are chosen with `solverforge_config::ForagerConfig`. The
default, `AcceptedCount`, means "select the best among the first N accepted
moves", not "scan the whole neighborhood and retain N". `BestScore` scans the
whole neighborhood.

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

### `CrossEntityDistanceMeter<S>` — `nearby_list_change.rs`

| Method | Signature |
|--------|-----------|
| `distance` | `fn(&self, solution: &S, src_entity: usize, src_pos: usize, dst_entity: usize, dst_pos: usize) -> f64` |

`DefaultCrossEntityDistanceMeter` is the zero-state default implementation. It
returns absolute position distance within one list entity and infinity across
different entities.

### `Solvable` — `manager/solver_manager.rs`

Requires: `PlanningSolution + Send + 'static`.

| Method | Signature |
|--------|-----------|
| `solve` | `fn(self, runtime: SolverRuntime<Self>, qualified_candidate_trace_provenance: Option<QualifiedCandidateTraceRunProvenance>) -> RuntimeBuildResult<()>` |

Return `Err` when the solve could not be declared, compiled, prepared, or
executed. The manager emits `SolverEvent::Failed` with the error's message
unless the solve already emitted a terminal event; a panic is treated as a
bug and also ends the job with `Failed`.

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
receiver. The slot is cached and reused per solution type per thread; each
detached solve starts from a pristine slot state. `SolverPanicPayload` is the cold foreign-runtime panic boundary. Its
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
| `apply` | `fn(self: Box<Self>, score_director: &mut dyn Director<S>)` |

A change is consumed when applied, so it can move its payload into the
working solution without cloning.

### Exhaustive Search

**`ExhaustiveSearchPhase<Dec>`** — Bounds: `Dec: ExhaustiveSearchDecider<S, D>`.

**`ExplorationType`** — `DepthFirst`, `BreadthFirst`, `ScoreFirst`, `OptimisticBoundFirst`.

**`ExhaustiveSearchConfig`** — `{ exploration_type, node_limit, depth_limit, enable_pruning }`.


`ExhaustiveSearchPhase` is cooperative with solver lifecycle control: every explored node advances the phase step count and the frontier loop polls pause, cancel, time, and in-phase limits before applying the next partial assignment. Finite `node_limit` or `depth_limit` bounds that leave frontier work unexplored terminate as `TerminatedByConfig`; an exhausted frontier remains `Completed`.

**`ExhaustiveSearchNode<S>`** — Tree node: depth, score, optimistic_bound, descriptor/variable/entity/candidate indices, parent_index. A node can reconstruct its scalar assignment path from stored parent indices.

**`SimpleDecider<S, V, B>`** — Generic decider with values and optional bounder.

`SimpleDecider` takes any `ScoreBounder`; `()` is the no-op bounder.

### Partitioned Search

**`PartitionedSearchPhase<S, PD, Part, SDF, PF, CP>`** — Generic over partitioner, score director factory, phase factory, child phases. Child scopes inherit runtime control, environment mode, remaining time limit, in-phase limits, and deterministic child seeds, but retained-job publication stays on the parent scope. The runtime phase-relative termination overlay is not propagated into a child scope. Pause checkpoints are emitted only from the parent full-solution boundary; child pause/cancel/config termination outcomes prevent partition merge.

**`FunctionalPartitioner<S, PF, MF>`** — Closure-based partitioner.

**`ThreadCount`** — `Auto`, `Unlimited`, `Specific(usize)`. `PartitionedSearchPhase` solves child partitions sequentially when the resolved count is `1`, otherwise it installs a dedicated Rayon pool whose worker count matches the resolved value.

## Scope Hierarchy

### `ProgressCallback<S>` — `scope/solver.rs`

Public zero-allocation callback dispatch trait. Implemented for `()` (no-op) and
any `F: for<'a> Fn(SolverProgressRef<'a, S>) + Send + Sync`. Its public
`invoke()` method receives one borrowed progress payload; the hidden
`PUBLISHES_PROGRESS` constant lets the runtime compile out no-op publication.

**`SolverProgressKind`** — `Progress` or `BestSolution`.

**`SolverProgressRef<'a, S>`** — Borrowed callback payload with public `kind`,
`status: SolverLifecycleState`, optional `solution`, `current_score`, and
`best_score` references, plus owned aggregate `telemetry`.

### `SolverScope<'t, S, D, ProgressCb = ()>`

Top-level scope for a retained solve. Holds score director, current score, best solution, best score, RNG, active timing, stats, runtime bridge, terminal reason, termination state, and the internal configured-runtime publication gate. Configured execution defers best-solution publication until the compiled graph proves mandatory structural completion; partial construction scores remain internal.

Custom phases draw randomness through `random_below(upper: usize) -> usize` (panics for `upper == 0`) and `random_u64() -> u64`, both from the solve's seeded generator; the generator itself is crate-private, so a `rand` upgrade is not a public API change.

Key methods: `new(score_director)`, `new_with_callback(score_director, callback, terminate, runtime)`, `with_progress_callback(F) -> SolverScope<.., F>`, `with_runtime(runtime)`, `start_solving()`, `initialize_working_solution_as_best()`, `replace_working_solution_and_reinitialize(solution)`, `score_director()`, `working_solution()`, `mutate(...)`, `current_score()`, `best_score()`, `calculate_score()`, `update_best_solution()`, `report_progress()`, `report_best_solution()`, `pause_if_requested()`, `pause_timers()`, `resume_timers()`, `mark_cancelled()`, `mark_terminated_by_config()`, `is_terminate_early()`, `set_time_limit()`. The current implementation also owns the one-second phase progress pulse and tracks a working-solution revision for built-in descriptor-driven construction completion; committed mutation goes through `mutate(...)` (or the equivalent crate-private step boundary), which clears `current_score` and advances that revision exactly once. The committed-writer contract behind that sentence: every committed change (accepted construction/local-search/VND moves, configuration-list commits, partition merges, apply-closures) funnels through one committed-mutation path, speculation never advances the revision, and model callbacks (candidate values, order keys, value legality, ownership policies) may read the whole solution — placement and enumeration therefore re-read committed global state rather than trusting a cached snapshot. `replace_working_solution_and_reinitialize(solution)` is the wholesale writer: fresh director reset, revision back to 1, frontier completion markers invalidated. Speculative phase evaluation uses `Move::do_move`, the returned typed undo value, `Move::undo_move`, and `DirectorScoreState` snapshots to restore both solution values and committed score state after scoring a candidate, and sequential composite doability checks run `Move::is_doable_with_working_preview` on the caller's live director instead of an immutable preview clone. An internal phase-relative termination overlay records the best and last-improving committed scores only while an explicit runtime construction or local-search phase executes; it is neither a public `SolverScope` setting nor child-scope state. Internal prompt-control plumbing also exposes immutable `pending_control()` so built-in phases can abandon partial steps and unwind to runtime-owned boundaries before settling pause/cancel/config termination or a queued real-time problem change.

Public fields: `inphase_step_count_limit`, `inphase_move_count_limit`, `inphase_score_calc_count_limit`.

### `PhaseScope<'t, 'a, S, D, BestCb = ()>`

Borrows `&mut SolverScope`. Tracks per-phase state: phase_index, starting_score, step_count; its `PhaseStats` record lives in the solver's `SolverStats` for the scope's lifetime. Public committed mutation delegates to `mutate(...)` on the parent solver scope; speculative candidate evaluation is handled by phase evaluation helpers using typed move undo.

### `StepScope<'t, 'a, 'b, S, D, BestCb = ()>`

Borrows `&mut PhaseScope`. Tracks per-step state: step_index, step_score. `complete()` records step in stats, while public committed mutation delegates to the same `mutate(...)` boundary used by `SolverScope`. Crate-private committed move helpers apply selected moves by ownership after candidate evaluation has used typed undo for rollback.

The compiled runtime checks mandatory completion from the frozen graph bindings:
all declared list elements must be assigned exactly once, assignment groups may
leave only non-required rows unassigned, and scalar slots outside assignment
groups may remain unassigned only when their slot declares that capability.
Local search cannot start before this gate passes. Config or phase termination
with unresolved mandatory work travels through `RuntimeBuildError::Execution`
and the existing `Failed` manager lifecycle, without a `BestSolution`, completed
snapshot, or partial paused snapshot.

## Manager System

### `SolverManager<S: Solvable>`

Static lifetime retained-job manager: `solve()` returns `(job_id, receiver)`;
`solve_with_qualified_candidate_trace_provenance()` uses the same lifecycle
while installing an externally validated trace attestation. Other methods are
`get_status()`, `get_telemetry_detail()`, `pause()`, `resume()`, `cancel()`,
`delete()`, `get_snapshot()`, `analyze_snapshot()`, `add_problem_change()`, and
`active_job_count()`.
The retained lifecycle contract is expressed in neutral `job`, `snapshot`, and
`checkpoint` terminology. `pause()` settles at a runtime-owned safe boundary
and `resume()` continues from the exact in-process checkpoint. `delete()` hides
a terminal job immediately, but the slot itself is not reusable until the
solve worker has definitely exited. `MAX_JOBS = 16`.

`add_problem_change<P: ProblemChange<S> + 'static>(job_id, change) ->
Result<ProblemChangeId, SolverManagerError>` queues a real-time change on a
`Solving`, `PauseRequested`, or `Paused` job; see Real-Time Planning. It returns
`JobNotFound` for a hidden or free slot and `InvalidStateTransition { action:
"add_problem_change", .. }` once cancellation was requested, the job is
terminal, or the runtime finished its final phase pass.

### `ProblemChangeId`

`Copy` acknowledgement of one queued change; `get() -> u64` is the 1-based
submission ordinal within its job. The change has been applied once a later
event or status reports `telemetry.problem_changes_applied >= id.get()`.

### `SolverLifecycleState` / `SolverTerminalReason`

Lifecycle states: `Solving`, `PauseRequested`, `Paused`, `Completed`, `Cancelled`, `Failed`. Terminal reasons: `Completed`, `TerminatedByConfig`, `Cancelled`, `Failed`.

### `SolverStatus<Sc>`

Retained job summary from `get_status()`. Fields: `job_id`, `lifecycle_state`, `terminal_reason`, `checkpoint_available`, `event_sequence`, `latest_snapshot_revision`, `current_score`, `best_score`, `telemetry`. `checkpoint_available` means the runtime currently holds an exact resumable checkpoint for `resume()`. Analysis availability is separate from terminality: a job can expose retained snapshots while still solving or pausing.

### `SolverTelemetryDetail<Sc>`

Detached retained-job telemetry returned by `get_telemetry_detail()`. It contains the current `SolverStatus<Sc>` plus optional candidate-trace detail under one retained-publication lock: when detail is present, the status scores and `latest_snapshot_revision` identify the exact paired snapshot a caller may fetch and validate. Ordinary status, snapshot, and lifecycle-event payloads remain compact and never materialize the candidate trace.

### `SolverEvent<S>`

Lifecycle event stream for retained jobs. Variants: `Progress`, `BestSolution`, `PauseRequested`, `Paused`, `Resumed`, `Completed`, `Cancelled`, `Failed`. Each event carries metadata with job id, monotonic event sequence, lifecycle state, terminal reason, telemetry, scores, and optional snapshot revision. Event metadata is authoritative: for example, progress can report `PauseRequested` while a pause is still settling toward a `Paused` checkpoint, and once `pause()` is accepted the stream delivers `PauseRequested` before any later worker-side event already in `PauseRequested` state.

Best-solution scores are monotonic only between problem-change boundaries:
after applied problem changes the stream reports a `Progress` event whose
`telemetry.problem_changes_applied` increased and whose `best_score` is `None`,
then a `BestSolution` for the changed problem once structural completion holds.
That `BestSolution` may score below earlier ones.

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

Serde-serializable. `ScoreAnalysis { score, constraints: Vec<ConstraintAnalysis> }`. `ConstraintAnalysis { name, weight_multiplier, score, match_count }` (serde `weightMultiplier`); `score` is the weighted contribution and `weight_multiplier` is the bound runtime multiplier (`0` = disabled).

## Real-Time Planning

Changes are submitted with `SolverManager::add_problem_change`, which boxes
them into the job's FIFO queue and raises a problem-change bit in the job's
control word, which also carries the pause request. Every built-in phase family
reads that word at its existing pause poll points, so a pending change costs no
extra load, and stops at the next safe point without recording a terminal
reason; pause, cancellation, and configured limits rank above it. Partitioned-search children
yield to their parent, which returns without merging; only the configured
runtime drains the queue. It then:

1. starts from the published best solution (or the working solution while
   publication is still deferred during construction),
2. applies the whole batch in submission order as one committed mutation,
3. resets the score director, which rebuilds the constraint set from the
   changed solution, and rescores from scratch,
4. drops the old best, defers publication, bumps
   `telemetry.problem_changes_applied`, and reports `Progress`,
5. restarts its phase sequence: construction assigns new or unassigned work and
   the structural completion gate publishes the changed solution as
   `BestSolution` before local search resumes.

A paused job applies its queue after `resume()`; a paused job is never mutated.
The queue is drained or closed under the publication lock after every phase
pass, so a late submission is either applied by another pass or rejected.
Changes still queued when cancellation or a configured limit stops the solve
are discarded. Configured time and work limits stay binding across passes, so a
long-running real-time job should use no solver-level time limit and end with
`cancel()`; a limit reached during re-construction fails the job like any other
incomplete mandatory construction.

**`ClosureProblemChange<S, F>`** — `new(name, change_fn)`; wraps a one-shot
`FnOnce(&mut dyn Director<S>) + Send` and implements `ProblemChange<S>`.

### `SolverStats` / `PhaseStats`

Aggregate and per-phase metrics: step count, moves generated, moves evaluated,
moves accepted, moves applied, score calculations, elapsed time, generation
time, evaluation time, acceptance rate, selector-level telemetry, and exact
`Throughput { count, elapsed }` views for generated/evaluated work.

`SolverStats` records every counter exactly once, into the running phase's
`PhaseStats` (or, between phases, into solve-level work such as the initial
score calculation). `begin_phase(phase_type)` opens a phase record and
`end_phase()` freezes it; `PhaseScope` opens its record on creation and closes
it on drop, and `PhaseScope::stats()` / `stats_mut()` borrow that record. Run
counters are read through methods (`step_count()`, `moves_evaluated()`,
`moves_speculative()`, `score_calculations()`, ...) that sum the finished
phases, solve-level work, and the running phase. `phase()` / `phase_mut()` /
`phase_elapsed()` expose the running phase. `SolverStats` also owns the
pause-aware solve clock: `start()`, `pause()`, `resume()`, `elapsed()`,
`active_elapsed()`, `started_at()`, and `is_paused()`; `SolverScope::elapsed()`
and `PhaseScope::elapsed()` read it. `problem_changes_applied()` and
`record_problem_changes_applied(count)` keep the cumulative real-time change
counter.
Human-facing `moves/s` is derived only at log/console formatting edges.
`moves_generated` counts candidates actually yielded by a runtime cursor; it
does not count an unrequested logical tail. `moves_speculative` counts
candidates that parallel local-search evaluation evaluated ahead of the
sequential decision point and then discarded; it is zero for sequential
evaluation and never feeds `moves_evaluated` or `score_calculations`. Selector `size()` and explicit full
cursor exhaustion cover logical neighborhood size and canonical order.

`SolverTelemetry` snapshots expose the same counters plus `phases`, one
`PhaseTelemetry` per phase started so far in execution order (the last entry of
a progress snapshot is the running phase), each with its `phase_index`,
`phase_type`, solver-active elapsed, and step, move (including
`moves_speculative`), score-calculation, generation-time, and evaluation-time
counters. Phase counters sum to the run counters, apart from score
calculations made outside any phase. Run snapshots also carry
not-doable,
acceptor-rejected, forager-ignored, hard-improving/neutral/worse, conflict
repair provider/filter/exposure counters, `construction_slots_assigned`,
`construction_slots_kept`, `construction_slots_no_doable`,
`problem_changes_applied` (cumulative real-time changes applied to the solve),
and `scalar_assignment_required_remaining`, which
distinguish scalar construction slots that received a candidate, legally kept
their current unassigned value, had no doable candidate, or remain uncovered
after scalar assignment construction. Candidate-backed grouped scalar
construction records completion against the exact grouped slot and also marks
every scalar slot covered by the grouped decision, so later construction phases
cannot fill those members one by one. Assignment-backed decisions complete only
the anchor group slot; assigned occupants remain available to required augmenting
rematches. `SelectorTelemetry` exposes
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
application, and `DiscardedSpeculative` (pulled into a parallel evaluation
window, then discarded when the step ended before it was consumed).

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
and aggregate telemetry types are crate-root re-exports.

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
- `PlanningModelSupport` — `#[doc(hidden)]` support trait (and `#[doc(hidden)]`
  `model_support` module) with no default impl; generated by
  `planning_model!` so solution derives can attach descriptor hooks,
  runtime scalar/list hooks, resolve list element owners, attach scalar groups,
  validate the manifest-backed model, and delegate configured list-shadow
  updates without proc-macro registries.

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

The configured termination and the runtime's `ProgressCallback` adapter are
internal to the run boundary.

The local-search phase emits `phase_start` with the current score after calculating
the starting local-search score, and emits `phase_end` with the best score. The
console layer renders phase-start scores when present; construction and
partitioned phase starts currently omit a score field.

`try_run_solver_with_config_and_search(...) -> RuntimeBuildResult<S>` is the one
public configured solve entrypoint used by macro-generated solving. It accepts
the solution, a constraint-set constructor `fn(&S) -> C`, the descriptor,
entity counting and logging callbacks, a
`SolverRuntime<S>`, `SolverConfig`, fallback time limit, optional qualified
candidate-trace provenance, and a fallible builder for one typed `Search`
declaration. The constructor builds the working solution's constraint set and
every evaluator replica's. Before building the director it validates the
solution's `PlanningSolution::constraint_weights()` against the constraint
set's metadata; an unknown constraint name or a multiplier above
`MAX_CONSTRAINT_WEIGHT_MULTIPLIER` emits the `Failed` lifecycle event and
returns `RuntimeBuildError::ConstraintWeights` before any phase runs. Weights
installed by an applied problem change are validated the same way before the
rescore, and an invalid set fails the job with the same error.

It scores on an `UnweightedMode` director and fails with
`RuntimeBuildError::ConstraintWeights` when the solution's weights require
weighting (`requires_weighted_mode`). The doc-hidden
`try_run_weighted_solver_with_config_and_search(...)`, with the same
signature, is the entrypoint `#[planning_solution]` emits for types with a
`#[constraint_weights]` field. It chooses the director's weight mode once per
solve: `WeightedMode` when some multiplier differs from `1`, otherwise
`UnweightedMode`, which runs exactly the unweighted scoring code. When an
applied problem change needs the other mode, the solve stops before
rescoring, rebuilds the director in that mode from the changed working
solution, moves the whole solve state onto it (statistics and clock,
configured limits and termination state, RNG, candidate trace, runtime and
publication state, runner state), rescores, and resumes the pass; lifecycle
events are emitted exactly as without a switch. Only models that name it
monomorphize the weighted solver. Graph
compilation and solve-owned source preparation remain private; there is no
public graph or alternate phase-builder fallback.

`RuntimeBuildError` is `Configuration { message }` (an embedded `solver.toml`
that does not parse; displayed as `solver configuration invalid: ...`),
`Declaration { message }`,
`Compilation { path, message }`, `ConstraintWeights { message }`,
`Preparation { phase_index, message }`, or
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
- **Tuple-based composition.** Terminations compose via nested tuples with macro-generated impls, and configured phases compose inside one compiled runtime phase, avoiding `Vec<Box<dyn Phase>>`.
- **Intentional `dyn` boundaries.** `ProblemChange::apply` takes
  `&mut dyn Director<S>` at the real-time integration seam. Dynamic scalar/list
  access and metadata, `RuntimeCandidateMetric`,
  `RuntimeHostCompoundProvider`, and the cold
  `RuntimeHostProviderErrorBoundary` are explicit host-language boundaries;
  static Rust providers never enter them.
- **`ProblemChange::apply` uses `&mut dyn Director<S>`, and queued changes are `Box<dyn ProblemChange<S>>`** — intentional, cold, per-change type erasure at the real-time planning boundary.
- **`SolverPanicPayload` owns `Box<dyn Any + Send>`** — cold foreign-runtime
  panic preservation, outside candidate generation and scoring.
- **Cursor-owned candidate lifetime.** Hot phases evaluate cursor candidates by stable ID, release losers immediately, and move the winner out exactly once. A sequential composite owns its two child moves in one two-slot allocation.
- **Neighborhood support modules stay private.** `list_support.rs`, `nearby_list_support.rs`, and `sublist_support.rs` exist only to share selected-entity snapshots, bounded stable top-k nearby ordering, and exact finite-selector counting. Public cursor hot loops for list and sublist neighborhoods remain explicit.
- **Canonical neighborhood tests live under subsystem trees.** Multi-file selector behavior for list, nearby-list, and sublist families is documented under `heuristic/selector/tests/`, while move legality stays under `heuristic/move/tests/`.
- **Rayon for parallelism.** Partitioned search uses rayon for CPU-bound parallel solving. `tokio::sync::mpsc` for solution streaming.
