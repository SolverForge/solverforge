# solverforge-scoring WIREFRAME

Zero-erasure incremental constraint scoring infrastructure for SolverForge.

**Location:** `crates/solverforge-scoring/`
**Workspace Release:** `0.23.0`

## Joined grouping fluent surface

- `stream::cross::{Tri, Quad, Penta}` and
  `stream::{TriConstraintStream, QuadConstraintStream, PentaConstraintStream}`
  expose `group_by(key, collector)`. Key closures take three/four/five entity
  references; collector mappers receive the corresponding borrowed tuple.
- `stream::JoinedGroupedStream<S, O, F, C, K, V, R, Acc, Sc>` exposes
  `penalize(weight)` and `reward(weight)` over `(&K, &R)`.
  `stream::JoinedGroupedBuilder<..., W, Sc>::named(name)` produces an
  incremental `OperatorTerminal`. Collector results are borrowed, not cloned;
  `collect_vec` retains move-only payloads and exposes `CollectedVec<T>`.
- All these methods preserve upstream filtering and semantic source indexes.
  Joined tuple grouping uses the existing `GroupNode` and `FilterNode`, not
  arity-specific retention engines.
- File map: `stream/group_by_{tri,quad,penta}.rs`,
  `stream/joined_grouped_stream.rs`, and
  `stream/relational/tuple_collector.rs`; same-source methods are generated in
  `stream/arity_stream_macros/nary_stream/shared.rs`. The doc-hidden
  `stream::relational::TupleCollector<C,V,R,Acc>` is the borrowed-row adapter.

## Typed relational operator surface

These are module-level low-level exports, not facade-root exports or a completed
migration of the fluent stream families:

- `stream::relational::{Leaf<'a, A>, Concat<'a, L, A>}` provides borrowed row
  views. `RowHandle` is a hidden generational identity bridge.
- `stream::relational::operator::Operator<S>` defines `View<'a>: Copy`,
  owned `Evaluation`, `prepare_evaluation`, `visit_evaluation`, `visit_all`,
  `clear`, `initialize`, `handles`, `resolve`, `visit_provenance`,
  `retract`, and `insert`. Views borrow entities; entities need not be Copy.
  Both notification methods append into caller-owned reused buffers
  `(&mut Vec<RowHandle> removed, &mut Vec<RowHandle> inserted)` and clear them
  first: an event costs one memcpy per hop and no heap traffic. A contributor
  retraction can still replace a surviving aggregate row within one event.
- `CollectionNode<S, E>::new(extractor, binding)` adapts `CollectionExtract`.
- `FilterNode<O, F>::new(input, predicate)` preserves accepted input identities
  and semantic source indexes. Its predicate takes `(&S, &O::View<'a>)`;
  membership transitions propagate retract/insert deltas without copying rows.
- `MergeNode<L, R>::new(left, right)` unions compatible borrowed row views with
  branch-specific output identities; equal-valued emissions remain separate.
- `ProjectNode<O, F, T>::new(input, mapper)` owns zero or more emissions per
  input row. The mapper takes `(&S, &O::View<'a>)` and returns `Vec<T>`.
  `ProjectView<'a, V, T>` exposes `input: V`, `value: &'a T`, and `emission:
  usize`; payloads need not implement Clone, Copy, Debug, or PartialEq.
  Explanations and provenance retain the complete upstream row.
- `GroupNode<O, F, C, K, A, V, R>::new(input, key, collector)` groups arbitrary
  input rows with exact accumulator retraction tokens. `GroupView<'a, S, O,
  K, A, V, R>` exposes `key: &'a K` and `with_result(callback)` over the
  accumulator-owned result, including non-Clone `CollectedVec<T>` payloads.
  Either join input can be a grouped producer. Contributor changes replace
  aggregate rows on both callbacks and coalesce within one root notification.
- `ExistenceNode<S, L, R, K, LK, RK>::new(left, right, left_key, right_keys,
  exists)` answers a pure-key semi/anti join from per-key occurrence counts
  (dense arrays for `usize` planning ids via `KeyState` in
  `stream/relational/count_state.rs`, hash tables otherwise). The left key
  adapter implements `ViewKey<L::View<'a>>`; the right adapter implements
  `RightKeySet<R::View<'a>>` (`UnitKeys` for one key per right row,
  `ParentChildKeys` to expand flattened children in place, `GroupRightKeys`
  for grouped rows keyed off their accumulator result). Right-side children
  never become rows and never receive handles; only left rows are published
  for scoring.
- `JoinerExistenceNode<S, L, R, P>::new(left, right, condition, exists)` is
  the per-pair companion for relationships WITH a residual predicate
  (`.and(filtering(...))` over an `EqualJoiner`): it compiles the condition's
  plan, retains passing pair rows and their reverse links, and publishes left
  rows by membership. Use the count-backed `ExistenceNode` when no residual
  predicate exists.
- `FlattenNode<O, F, T>::new(input, extractor)` emits borrowed child slices
  without cloning children or retaining solution references. The extractor takes
  `(&'a S, O::View<'a>)` and returns `&'a [T]`. `FlattenView<'a, V, T>` exposes
  `input: V`, `value: &'a T`, and `child: usize`; parent row provenance and its
  semantic source indexes are preserved, including for owned projected parents.
- `ComplementNode<S, L, R, P, F, T>::new(targets, results, condition, default)`
  produces a typed target-domain outer relation. Real matches retain target and
  result bindings; unmatched targets own their default payloads. `ComplementView`
  is `Real(Pair<L, R>)` or `Default(ProjectView<'a, L, T>)`. Duplicate target
  rows preserve bag multiplicity, and grouped results have one accumulator owner.
- `JoinNode<S, L, R, P>::new(left, right, condition)` compiles a condition and
  produces `Pair<L::View<'a>, R::View<'a>>`; either input can itself be a join.
  `Pair<L, R>` has public `left` and `right` fields. Depth is recursive rather
  than fixed to a named arity.
- `stream::chain::{Chain<S, O, Sc>, ChainBuilder}` is the arity-free fluent continuation:
  `stream::cross::Penta::join` returns one, and `Chain::join` nests further `JoinNode`s with no
  ceiling; `Chain::filter` wraps the tree in an identity-preserving
  `FilterNode`; `Chain::penalize`/`reward` then `ChainBuilder::named` finalize
  into `OperatorTerminal`. The named `Bi`/`Tri`/`Quad`/`Penta` streams are
  ergonomic adapters over the same operator tree, not separate engines.
  The current cross-Penta continuation discards its authored penta filter; apply
  the row filter on the returned `Chain`. Same-source `PentaConstraintStream` is terminal.
- `constraint::relational::OperatorTerminal<S, O, W, Sc>::new(constraint_ref,
  impact, operator, weight, hard)` scores a generic operator; its weight takes
  `(&S, &O::View<'a>)`. It implements incremental scoring, reset, full counts,
  and cold explanations. `ExplainRow` is a hidden analysis bridge.
- `stream::joiner::plan::{CompileCondition, IndexedPlan, ExecutablePlan}`
  separates condition compilation, owned index state, and borrowed-row execution.
  Candidate probes return `Cow<'i, [RowHandle]>`: equality borrows an exact
  bucket; ordered, interval, and explicit scan plans may own candidate lists.
  `candidate_matches` checks residual semantics. Full traversal uses transient
  insertion without retaining equality reverse keys.
- `CompositeEquality<A, B>` and `EqualityWithResidual<E, P, FIRST>` are concrete
  compiled plan types. `ComposePlans`, `EqualityPlan`, `EqualityKeys`,
  `EqualityKind`, and `ResidualKind` supply structural dispatch. All equality
  components form one heterogeneous tuple key, including components separated
  by residual conditions.
- `PlannedCondition`, `Strategy`, and `plan_strategy` report deterministic
  structural strategy. Equality takes precedence over ordered/interval/scan
  conditions; the residual preserves the complete authored relationship.
- `stream::joiner::pair_row(a, a_index, b, b_index)` constructs the
  existing `Concat<Leaf<A>, B>` view using semantic indexes.

New file map: `stream/relational/operator.rs`,
`stream/relational/operator/{analysis,collection,complement,complement_view,
existence,filter,flatten,group,group_view,join,joiner_existence,merge,project,
self_join}.rs`,
`stream/relational/{count_state,keyed_plan,view_plan}.rs`,
`stream/joiner/plan.rs`, `stream/joiner/plan/{executable,hash_scan,conjunction,
composite,mixed_equality,ordered_interval}.rs`, `stream/joiner/row_key.rs`,
`stream/relational/index/{hash,ordered,interval}.rs`,
`stream/relational/hash.rs`, and
`constraint/relational/operator_terminal.rs`. Generational direct side-tables
are internal in `stream/relational/handle_map.rs`; `stream/relational/hash.rs`
holds the deterministic fast hasher the internal index maps build on.

This surface supports collection/filtered/merged/joined, owned projection,
group/complement, existence, and flattened producers, and it is the single
grouping execution path: every fluent `group_by`, grouped complement, and
chained grouped terminal lowers onto these operators. Compiler-owned terminal
sharing lives in `constraint/relational/shared_terminal.rs`, which keeps one
operator owner for chained `penalize`/`reward` terminals and scores, orders,
and explains each terminal independently.
See [Extending typed relational scoring](../../docs/extend-scoring.md) for
execution, ownership, and verification boundaries.

## Dependencies

- `solverforge-core` (path) — Score types, domain traits, descriptors, ConstraintRef, ImpactType
- `thiserror` (workspace) — error derive macros

## File Map

```
src/
├── lib.rs                                          — Crate root; re-exports from all modules
├── api/
│   ├── mod.rs                                      — Re-exports analysis, constraint_set, node_sharing
│   ├── analysis.rs                                 — ScoreExplanation, ConstraintAnalysis, Indictment, IndictmentMap, DetailedConstraintMatch, etc.
│   ├── node_sharing.rs                             — SharedNodeDiagnostics, SharedNodeId, SharedNodeOperation
│   ├── constraint_set/
│   │   ├── mod.rs                                  — Re-exports ConstraintSet, ConstraintSetChain, OrderedConstraintSetChain, ConstraintSetSource, IncrementalConstraint, SingleConstraint, ConstraintMetadata, ConstraintResult
│   │   ├── chain.rs                                — ConstraintSetChain, OrderedConstraintSetChain, and authored-order source spans
│   │   ├── incremental.rs                          — IncrementalConstraint trait, ConstraintSet trait, result and metadata types
│   │   ├── blanket.rs                              — ConstraintSet for every IncrementalConstraint (singleton set)
│   │   ├── tuple.rs                                — ConstraintSet for () and tuples (1..32)
│   │   ├── weighting.rs                            — Crate-private per-constraint contribution scaling and weight-slot splitting
│   │   └── tests/
│   │       ├── mod.rs                              — Test module declarations
│   │       └── constraint_set.rs                   — ConstraintSet tuple tests
│   └── tests/
│       ├── mod.rs                                  — Test module declarations
│       └── analysis.rs                             — Analysis type tests
├── constraint/
│   ├── mod.rs                                      — Module declarations and selected root re-exports
│   ├── macros.rs                                   — impl_get_matches_nary! macro for detailed match generation
│   ├── shared.rs                                   — compute_hash<T>() utility function
│   ├── list_precedence.rs                          — ListPrecedenceMakespanConstraint<S>; stock incremental list-plus-fixed-precedence makespan scoring with optional fixed owner checks
│   ├── list_precedence/{graph,route}.rs            — Precedence-graph refresh and per-owner route bookkeeping for ListPrecedenceMakespanConstraint
│   ├── single_constraint_markers.rs                — SingleConstraint impls for built-in constraint types
│   ├── incremental.rs                              — IncrementalUniConstraint<S,A,E,F,W,Sc>
│   ├── score_sum.rs                                — Crate-private batch score accumulation with one overflow check
│   ├── balance.rs                                  — BalanceConstraint<S,A,K,E,F,KF,Sc>
│   ├── relational/
│   │   ├── mod.rs                                  — Module declarations and selected re-exports
│   │   ├── operator_terminal.rs                    — OperatorTerminal<S,O,W,Sc> over one generic operator
│   │   ├── operator_scorer.rs                      — OperatorScorer, OperatorScorerSet, ScorerChain, and RowWeight/AdaptWeight adapters
│   │   ├── shared_terminal.rs                      — SharedOperatorTerminal/Builder and chained terminal sharing
│   │   └── grouped_weights.rs                      — Group/direct-complement/optional-complement weight adapters
│   ├── flattened_bi.rs                             — FlattenedBiConstraint module root and re-exports
│   ├── flattened_bi/*.rs                           — Retained flattened-bi state, incremental callbacks, and debug accessors
│   ├── exists.rs                                   — IncrementalExistsConstraint<S,A,P,B,K,EA,EP,KA,KB,FA,FP,Flatten,W,Sc>, SelfFlatten
│   ├── exists/
│   │   └── key_state.rs                            — Internal hashed/indexed key bookkeeping for existence constraints
│   ├── projected.rs                                — Projected retained scoring-row constraint module root and re-exports
│   ├── projected/uni.rs                            — projected::Uni terminal constraint for retained projected rows
│   ├── projected/bi.rs                             — projected::Bi retained symmetric projected self-join constraint
│   ├── projected/directed_bi.rs                    — projected::DirectedBi retained directed projected self-join state
│   ├── projected/directed_bi_incremental.rs        — IncrementalConstraint callbacks for directed projected self-joins
│   ├── projected/grouped.rs                        — projected::Grouped fluent builder over the shared terminal
│   ├── projected/complemented_grouped.rs           — projected::ComplementedGrouped fluent builder over ComplementNode
│   └── tests/
│       ├── mod.rs                                  — Test module declarations
│       ├── cross_bi_incr.rs                        — Fluent cross-bi join tests on the generic operator terminal
│       ├── self_join_bi_fluent.rs                  — Fluent same-collection self-join tests
│       ├── relational/self_join_node.rs            — SelfJoinNode unique-combination and delta tests
│       ├── relational/condition_arity.rs           — Comparison/overlap/composed join conditions and None-key complement
│       ├── relational/derived_right_input.rs       — Filtered and complemented producers as right-hand join inputs
│       ├── relational/join_depth.rs                — Six-binding typed join chain (depth beyond Penta)
│       ├── relational/fluent_deep_chain.rs         — Six- and seven-source fluent chains via Penta::join → Chain
│       ├── grouped.rs                              — Grouped stream and shared terminal tests
│       ├── balance.rs                              — BalanceConstraint tests
│       ├── complemented.rs                         — Fluent complemented grouped tests
│       ├── flattened_bi.rs                         — FlattenedBiConstraint tests
│       ├── exists.rs                               — IncrementalExistsConstraint update tests
│       ├── exists_storage.rs                       — Existence storage selection and parity tests
│       ├── projected.rs                            — Projected constraint test module root
│       ├── projected/*.rs                          — Projected support fixtures, localization, update, grouping, merge, and self-join tests
│       ├── repro_unknown.rs                        — Regression fixture coverage for unknown-source behavior
│       └── weights/*.rs                            — Constraint weight multipliers {0,1,3} on every family: incremental vs full vs analysis oracle, shared terminals, ordered chains, duplicate names, reset rebinding
├── director/
│   ├── mod.rs                                      — Re-exports all director types and traits
│   ├── traits.rs                                   — Director<S> trait
│   ├── score_director.rs                           — Re-exports ScoreDirector pieces
│   │   ├── score_director/incremental.rs           — ScoreDirector<S,C,Desc,M> (zero-erasure incremental)
│   │   ├── score_director/constructors.rs          — ScoreDirector constructors, weighted constructors, into_weight_mode
│   │   ├── score_director/descriptor.rs            — DirectorDescriptor, ReplicaDescriptor
│   │   ├── score_director/list_shadows.rs          — Retract/insert of list-shadowed element rows around owner shadow updates
│   │   ├── score_director/weight_mode.rs           — WeightMode (sealed), UnweightedMode, WeightedMode, requires_weighted_mode, static per-mode scoring dispatch
│   │   ├── score_director/analysis.rs              — ScoreDirector constraint_results/constraint_analyses/constraint_match_totals
│   │   └── score_director/adapters.rs              — Debug and Director trait impls for ScoreDirector
│   ├── shadow_aware.rs                             — SolvableSolution trait and shadow lifecycle notes
│   └── tests/
│       ├── mod.rs                                  — Test module declarations
│       ├── bench.rs                                — Benchmark test module declarations
│       ├── benchmarks.rs                           — Performance comparison tests
│       ├── fixtures.rs                             — ScoreDirector fixtures
│       ├── fixtures_tests.rs                       — ScoreDirector fixture tests
│       ├── score_director.rs                       — ScoreDirector tests
│       ├── list_shadows.rs                         — Element streams reading list shadows, against fresh evaluation
│       └── shadow.rs                               — Shadow-aware director tests

├── stream/
│   ├── mod.rs                                      — Module declarations and re-exports for all stream types
│   ├── tests.rs                                    — Stream API tests
│   ├── factory.rs                                  — ConstraintFactory<S,Sc>
│   ├── uni_stream.rs                               — Re-exports
│   ├── uni_stream/base.rs                          — UniConstraintStream
│   ├── uni_stream/weighting.rs                     — UniConstraintBuilder and weighting helpers
│   ├── bi_stream.rs                                — BiConstraintStream, BiConstraintBuilder (via macro)
│   ├── tri_stream.rs                               — TriConstraintStream, TriConstraintBuilder (via macro)
│   ├── quad_stream.rs                              — QuadConstraintStream, QuadConstraintBuilder (via macro)
│   ├── penta_stream.rs                             — PentaConstraintStream, PentaConstraintBuilder (via macro)
│   ├── chain_stream.rs                             — Chain/ChainBuilder: arity-free fluent join continuation
│   ├── cross_penta_stream.rs                       — stream::cross::Penta and its Penta::join → Chain
│   ├── cross_penta_stream/scored.rs                — PentaScored operator (tree plus authored filter)
│   ├── grouped_stream.rs                           — Re-exports
│   ├── grouped_stream/base.rs                      — GroupedConstraintStream
│   ├── grouped_stream/weighting.rs                 — GroupedConstraintBuilder
│   ├── grouped_stream/joined.rs                    — GroupedJoinedStream/Builder: group as a join input
│   ├── balance_stream.rs                           — BalanceConstraintStream, BalanceConstraintBuilder
│   ├── complemented_stream.rs                      — ComplementedConstraintStream, ComplementedConstraintBuilder
│   ├── cross_bi_stream.rs                          — Re-exports
│   ├── cross_bi_stream/base.rs                     — stream::cross::Bi
│   ├── cross_bi_stream/scored.rs                   — BiUnaryPlan, BiPredicatePlan, BiScored (first-join execution)
│   ├── cross_bi_stream/view_comparison.rs          — ViewComparisonPlan, ViewOverlapPlan (entity conditions over leaf views)
│   ├── cross_bi_stream/grouped.rs                  — stream::cross::Grouped and builder
│   ├── cross_bi_stream/complemented_grouped.rs     — stream::cross::ComplementedGrouped and builder
│   ├── cross_bi_stream/weighting.rs                — stream::cross::Builder
│   ├── cross_tri_stream.rs                         — Re-exports
│   ├── cross_tri_stream/base.rs                    — stream::cross::Tri and builder
│   ├── cross_tri_stream/base/scored.rs               — TriScored operator (tree plus authored filter)
│   ├── flattened_bi_stream.rs                      — Re-exports
│   ├── flattened_bi_stream/base.rs                 — FlattenedBiConstraintStream
│   ├── flattened_bi_stream/builder.rs              — FlattenedBiConstraintBuilder
│   ├── flattened_bi_stream/weighting.rs            — Weighting helpers for flattened streams
│   ├── existence_stream.rs                         — ExistsConstraintStream, ExistsConstraintBuilder, ExistenceMode
│   ├── existence_target.rs                         — ExistenceTarget trait for direct and flattened existence targets
│   ├── projected_stream.rs                         — Projected stream module root and re-exports
│   ├── projected_stream/uni.rs                     — stream::projected::Stream and terminal builder
│   ├── projected_stream/bi.rs                      — stream::projected::Bi and terminal builder
│   ├── projected_stream/directed_bi.rs             — stream::projected::DirectedBi and terminal builder for directed projected self-joins
│   ├── projected_stream/join_target.rs             — ProjectedJoinTarget dispatch for symmetric and directed projected self-joins
│   ├── projected_stream/grouped.rs                 — stream::projected::Grouped, reward, complement, and terminal builder
│   ├── projected_stream/complemented_grouped.rs    — stream::projected::ComplementedGrouped and builder
│   ├── projected_stream/source.rs                  — Projection, projected row coordinates, Source trait
│   ├── projected_stream/source/single.rs           — Single-source `.project(...)` source
│   ├── projected_stream/source/filtered.rs         — Row-level filtered projected source
│   ├── projected_stream/source/merged.rs           — Merged projected sources with source-slot offsets
│   ├── projected_stream/source/joined.rs           — Cross-join `.project(...)` projected source
│   ├── collection_extract.rs                       — CollectionExtract trait, hidden source metadata and FilteredExtract membership adapter, VecExtract wrapper, vec() constructor
│   ├── unassigned.rs                               — Hidden UnassignedEntity hook and `.unassigned()` stream method
│   ├── weighting_support.rs                        — ConstraintWeight, FixedWeight, HardWeight, and dynamic closure-weight adapters
│   ├── join_target.rs                              — JoinTarget/ToViewPlan dispatch for self-join, keyed cross-join, comparison, overlap, composed, and predicate cross-join
│   ├── key_extract.rs                              — KeyExtract trait, EntityKeyAdapter struct
│   ├── arity_stream_macros/
│   │   ├── mod.rs                                  — impl_arity_stream! dispatcher macro
│   │   ├── nary_stream.rs                          — Module declarations for arity stream macros
│   │   └── nary_stream/
│   │       ├── shared.rs                           — Shared arity stream macro helpers
│   │       ├── bi.rs                               — impl_bi_arity_stream! macro
│   │       ├── tri.rs                              — impl_tri_arity_stream! macro
│   │       ├── quad.rs                             — impl_quad_arity_stream! macro
│   │       └── penta.rs                            — impl_penta_arity_stream! macro
│   ├── filter/
│   │   ├── mod.rs                                  — Re-exports filter types
│   │   ├── traits.rs                               — UniFilter, BiFilter, TriFilter, QuadFilter, PentaFilter traits
│   │   ├── wrappers.rs                             — TrueFilter, FnUniFilter, FnBiFilter, FnTriFilter, FnQuadFilter, FnPentaFilter
│   │   ├── adapters.rs                             — UniBiFilter, UniLeftBiFilter, UniPairFilter, and hidden PairFilter adapters
│   │   ├── composition.rs                          — AndUniFilter, AndBiFilter, AndTriFilter, AndQuadFilter, AndPentaFilter
│   │   └── tests/
│   │       ├── mod.rs                              — Test module declarations
│   │       └── filter.rs                           — Filter tests
│   ├── joiner/
│   │   ├── mod.rs                                  — Re-exports all joiner types and functions
│   │   ├── equal.rs                                — EqualJoiner, Symmetric/Directed mode markers, equal(), equal_bi() functions
│   │   ├── comparison.rs                           — LessThan/GreaterThan joiners and factory functions
│   │   ├── filtering.rs                            — FilteringJoiner and filtering() function
│   │   ├── overlapping.rs                          — OverlappingJoiner and overlapping() function
│   │   └── match_condition.rs                      — Joiner trait, AndJoiner, FnJoiner
│   └── collector/
│       ├── mod.rs                                  — Re-exports collector types
│       ├── core.rs                                 — Collector<Input> trait, Accumulator trait
│       ├── count.rs                                — CountCollector, CountAccumulator, count()
│       ├── sum.rs                                  — SumCollector, SumAccumulator, sum()
│       ├── load_balance.rs                         — LoadBalanceCollector, LoadBalanceAccumulator, LoadBalance, load_balance()
│       ├── runs.rs                                 — RunsCollector, RunsAccumulator, Run, Runs, consecutive_runs()
│       ├── indexed_presence.rs                     — IndexedPresenceCollector, IndexedPresenceAccumulator, IndexedPresence, indexed_presence()
│       ├── collect_vec.rs                          — CollectVecCollector, CollectVecAccumulator, CollectedVec, collect_vec()
│       └── tests/
│           ├── mod.rs                              — Test module declarations
│           └── collector.rs                        — Collector tests
```

## Public Re-exports (lib.rs)

```rust
// Root constraint exports keep globally distinct names only.
pub use constraint::{IncrementalUniConstraint, ListPrecedenceMakespanConstraint};

// Short family names are intentionally module-scoped:
// constraint::projected::{Uni, Bi, DirectedBi}
// Grouped terminals are constraint::relational::{OperatorTerminal, SharedOperatorTerminal}

// Constraint Set
pub use api::constraint_set::{
    ConstraintMetadata, ConstraintResult, ConstraintSet, IncrementalConstraint,
    SingleConstraint,
};
pub use api::node_sharing::{SharedNodeDiagnostics, SharedNodeId, SharedNodeOperation};

// Score Directors
pub use director::score_director::{DirectorDescriptor, ReplicaDescriptor, ScoreDirector};
pub use director::{Director, DirectorScoreState, SolvableSolution};

// Analysis
pub use api::analysis::{
    ConstraintAnalysis, ConstraintJustification, DetailedConstraintEvaluation,
    DetailedConstraintMatch, EntityRef, Indictment, IndictmentMap, ScoreExplanation,
};

// Fluent Stream API
pub use stream::{
    fixed_weight, hard_weight, BiConstraintBuilder, BiConstraintStream, ConstraintFactory,
    FixedWeight, GroupedConstraintBuilder, GroupedConstraintStream, HardWeight,
    Projection, ProjectionSink, UniConstraintBuilder, UniConstraintStream,
};

// Short cross/projected stream names are intentionally module-scoped:
// stream::cross::{Bi, Builder, Grouped, GroupedBuilder, ComplementedGrouped, ComplementedGroupedBuilder, Tri}
// stream::projected::{Stream, Builder, Bi, BiBuilder, DirectedBi, DirectedBiBuilder, Grouped, GroupedBuilder, ComplementedGrouped, ComplementedGroupedBuilder}
```

## Public Traits

### `Director<S: PlanningSolution>` — `Send`

| Method | Signature | Note |
|--------|-----------|------|
| `working_solution` | `fn working_solution(&self) -> &S` | Current solution reference |
| `working_solution_mut` | `fn working_solution_mut(&mut self) -> &mut S` | Mutable solution reference |
| `calculate_score` | `fn calculate_score(&mut self) -> S::Score` | Full score calculation |
| `fresh_score` | `fn fresh_score(&self) -> Option<S::Score>` | Optional scratch score without mutating committed director state; default `None` |
| `solution_descriptor` | `fn solution_descriptor(&self) -> &SolutionDescriptor` | Runtime metadata |
| `clone_working_solution` | `fn clone_working_solution(&self) -> S` | Deep copy |
| `before_variable_changed` | `fn before_variable_changed(&mut self, descriptor_index: usize, entity_index: usize)` | Pre-change notification |
| `after_variable_changed` | `fn after_variable_changed(&mut self, descriptor_index: usize, entity_index: usize)` | Post-change notification |
| `entity_count` | `fn entity_count(&self, descriptor_index: usize) -> Option<usize>` | Count entities by descriptor |
| `total_entity_count` | `fn total_entity_count(&self) -> Option<usize>` | Total across all descriptors |
| `constraint_metadata` | `fn constraint_metadata(&self) -> Vec<ConstraintMetadata<'_>>` | Borrowed constraint metadata views known to this director |
| `constraint_is_hard` | `fn constraint_is_hard(&self, constraint_ref: &ConstraintRef) -> Option<bool>` | Exact identity helper derived from `constraint_metadata()` |
| `is_incremental` | `fn is_incremental(&self) -> bool` | Default: false |
| `snapshot_score_state` | `fn snapshot_score_state(&self) -> DirectorScoreState<S::Score>` | Snapshot committed score state for speculative evaluation |
| `restore_score_state` | `fn restore_score_state(&mut self, state: DirectorScoreState<S::Score>)` | Restore a previously snapshotted committed score state |
| `reset` | `fn reset(&mut self)` | Discard incremental state after an external working-solution rewrite; default: no-op |
| `fork_replica` | `fn fork_replica(&self) -> Option<Self> where Self: Sized` | Independent evaluator replica with initialized incremental state, sharing the director's borrowed descriptor; default: `None` |

### `DirectorScoreState<Sc>`

Committed score-state snapshot used to roll back speculative evaluation. Fields:
`solution_score`, `committed_score`, `initialized`.

### `SingleConstraint`

Marker that a type is one constraint. Every `IncrementalConstraint` is a one-constraint `ConstraintSet` through a blanket impl; the marker keeps that impl coherent with the tuple and chain sets. Hand-written constraints add `impl SingleConstraint for T {}`.

### `IncrementalConstraint<S, Sc: Score>` — `SingleConstraint + Send + Sync`

| Method | Signature | Note |
|--------|-----------|------|
| `evaluate` | `fn evaluate(&self, solution: &S) -> Sc` | Full recalculation |
| `match_count` | `fn match_count(&self, solution: &S) -> usize` | Number of matches |
| `initialize` | `fn initialize(&mut self, solution: &S) -> Sc` | Initialize state for incremental |
| `on_insert` | `fn on_insert(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc` | Incremental insert delta |
| `on_retract` | `fn on_retract(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc` | Incremental retract delta |
| `reset` | `fn reset(&mut self)` | Clear incremental state |
| `name` | `fn name(&self) -> &str` | Constraint name; default derives from `constraint_ref()` |
| `is_hard` | `fn is_hard(&self) -> bool` | Default: false |
| `constraint_ref` | `fn constraint_ref(&self) -> &ConstraintRef` | Borrowed package-qualified identity owned by the constraint |
| `get_matches` | `fn get_matches<'a>(&'a self, _solution: &S) -> Vec<DetailedConstraintMatch<'a, Sc>>` | Default: empty; unweighted match scores |
| `evaluate_weighted` | `fn evaluate_weighted(&self, solution: &S, multiplier: u32) -> Sc` | Default: `evaluate` scaled by `multiplier` |
| `initialize_weighted` | `fn initialize_weighted(&mut self, solution: &S, multiplier: u32) -> Sc` | Default: `initialize` scaled by `multiplier` |
| `on_insert_weighted` | `fn on_insert_weighted(&mut self, solution: &S, entity_index: usize, descriptor_index: usize, multiplier: u32) -> Sc` | Default: `on_insert` delta scaled by `multiplier` |
| `on_retract_weighted` | `fn on_retract_weighted(&mut self, solution: &S, entity_index: usize, descriptor_index: usize, multiplier: u32) -> Sc` | Default: `on_retract` delta scaled by `multiplier` |
| `get_matches_weighted` | `fn get_matches_weighted<'a>(&'a self, solution: &S, multiplier: u32) -> Vec<DetailedConstraintMatch<'a, Sc>>` | Default: `get_matches` with every match score scaled by `multiplier` |
| `on_insert_overflowing`, `on_retract_overflowing`, `on_insert_weighted_overflowing`, `on_retract_weighted_overflowing` | `fn …(…) -> (Sc, bool)` | `#[doc(hidden)]`; the delta unchecked with an overflow flag. Defaults delegate to the checked methods (flag `false`); built-in constraints implement them and their checked methods panic on the flag |

### `ConstraintSet<S, Sc: Score>` — `Send + Sync`

| Method | Signature | Note |
|--------|-----------|------|
| `evaluate_all` | `fn evaluate_all(&self, solution: &S) -> Sc` | Sum all constraints |
| `constraint_count` | `fn constraint_count(&self) -> usize` | Number of constraints |
| `constraint_metadata` | `fn constraint_metadata(&self) -> Vec<ConstraintMetadata<'_>>` | Deduplicated borrowed ref/name/hardness metadata |
| `constraint_metadata_entries` | `fn constraint_metadata_entries(&self) -> Vec<ConstraintMetadata<'_>>` | Hidden raw metadata entries in authored order |
| `evaluate_each` | `fn evaluate_each<'a>(&'a self, solution: &S) -> Vec<ConstraintResult<'a, Sc>>` | Per-constraint results |
| `evaluate_detailed` | `fn evaluate_detailed<'a>(&'a self, solution: &S) -> Vec<ConstraintAnalysis<'a, Sc>>` | With match details |
| `initialize_all` | `fn initialize_all(&mut self, solution: &S) -> Sc` | Initialize all for incremental |
| `on_insert_all` | `fn on_insert_all(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc` | Incremental insert all |
| `on_retract_all` | `fn on_retract_all(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc` | Incremental retract all |
| `reset_all` | `fn reset_all(&mut self)` | Reset all |
| `constraint_weight_refs` | `fn constraint_weight_refs<'a>(&'a self, refs: &mut Vec<&'a ConstraintRef>)` | Required; pushes one ref per weight slot (`constraint_count()` slots, structural order) |
| `evaluate_all_weighted` | `fn evaluate_all_weighted(&self, solution: &S, multipliers: &[u32]) -> Sc` | Required; one multiplier per weight slot |
| `evaluate_each_weighted` | `fn evaluate_each_weighted<'a>(&'a self, solution: &S, multipliers: &[u32]) -> Vec<ConstraintResult<'a, Sc>>` | Required; weighted scores and each slot's `weight_multiplier` |
| `evaluate_detailed_weighted` | `fn evaluate_detailed_weighted<'a>(&'a self, solution: &S, multipliers: &[u32]) -> Vec<ConstraintAnalysis<'a, Sc>>` | Required; weighted totals and match scores |
| `initialize_all_weighted` | `fn initialize_all_weighted(&mut self, solution: &S, multipliers: &[u32]) -> Sc` | Required |
| `on_insert_all_weighted` | `fn on_insert_all_weighted(&mut self, solution: &S, entity_index: usize, descriptor_index: usize, multipliers: &[u32]) -> Sc` | Required |
| `on_retract_all_weighted` | `fn on_retract_all_weighted(&mut self, solution: &S, entity_index: usize, descriptor_index: usize, multipliers: &[u32]) -> Sc` | Required |
| `on_insert_all_overflowing`, `on_retract_all_overflowing`, `on_insert_all_weighted_overflowing`, `on_retract_all_weighted_overflowing` | `fn …(…) -> (Sc, bool)` | `#[doc(hidden)]`; unchecked delta with an overflow flag. Defaults delegate to the checked methods |

**Overflow:** score deltas never wrap. Built-in constraints, tuple sets and chains sum their rows and members through the unchecked `*_overflowing` methods and the core `Score::overflowing_*` operations, and check the combined flag once at their public boundary, panicking with `a score delta overflows i64`. Retained operator-terminal state stores unsigned row weights; the impact is applied once per delta.

Implemented for each `IncrementalConstraint<S, Sc>` as a singleton set and for tuples `()` through `(C0, C1, ..., C31)` where each `Ci: ConstraintSet<S, Sc>`. `ConstraintSetChain<Left, Right>` composes two existing `ConstraintSet` values without erasing either concrete type. `OrderedConstraintSetChain<Left, Right>` uses `ConstraintSetSource::Left` and `ConstraintSetSource::Right { constraint_count, metadata_entry_count }` spans to preserve user-authored result ordering when macro-generated node sharing combines one shared set with surrounding constraints. Raw metadata entries keep one entry per constraint for ordering; the public `constraint_metadata()` view deduplicates repeated full `ConstraintRef`s when hardness agrees and panics when the same full ref has conflicting hard/non-hard metadata. Package-qualified constraints that share a short name remain distinct.

### Shadow Lifecycle on `PlanningSolution`

`PlanningSolution` itself owns the canonical shadow hooks:
- `update_entity_shadows(&mut self, descriptor_index: usize, entity_index: usize)` — default no-op
- `update_all_shadows(&mut self)` — default no-op

`ScoreDirector` treats an owner's list shadow update as a mutation of the
elements it writes (`list_shadow_elements`). `before_variable_changed`
retracts the rows of the elements the owner holds; `after_variable_changed`
retracts rows of elements that arrived from no list, rewrites the shadows,
inserts the owner and every element it now holds, and clears and re-inserts
elements that left the owner without joining another list. Element rows
therefore always hold the shadows they were inserted with.

### `SolvableSolution` — `: PlanningSolution`

| Method | Signature | Note |
|--------|-----------|------|
| `descriptor` | `fn descriptor() -> SolutionDescriptor` | Static descriptor |
| `entity_count` | `fn entity_count(solution: &Self, descriptor_index: usize) -> usize` | Entity count |

### Runtime Constraint Weights

Weighting is a property of the director's type: `ScoreDirector`'s weight mode
`M` (default `UnweightedMode`) is fixed when the director is built and every
mode-dependent call dispatches statically through it, with no runtime check.
An `UnweightedMode` director compiles to exactly the unweighted
`ConstraintSet` calls: it stores no weights, never binds them, and ignores any
weights the solution carries. A `WeightedMode` director resolves
`PlanningSolution::constraint_weights()` into one multiplier per weight slot
(`constraint_weight_refs` order; absent names and a `None` accessor bind `1`)
on construction and before every (re)initialization, and always takes the
`*_weighted` calls. `requires_weighted_mode(&solution)` —
`HAS_CONSTRAINT_WEIGHTS` and some multiplier other than `1` — is the check a
solve uses to pick the mode; `into_weight_mode::<M2>()` rebuilds a director in
the other mode over the same working solution.

Multipliers scale each constraint's aggregated contribution (its total, its
per-call delta, and its match scores), never the per-match signing inside a
constraint; retained constraint state stays unweighted. Singleton constraints
scale through the `IncrementalConstraint::*_weighted` defaults; shared grouped
terminals scale each chained scorer by its own slot, so terminals sharing one
node keep independent multipliers. `ScoreDirector::constraint_results()`,
`constraint_analyses()`, `constraint_match_totals()`, and `fresh_score()` report
weighted values and each constraint's `weight_multiplier`; the raw
`ConstraintSet` analysis methods report unweighted values with multiplier `1`.
Multiplier `0` zeroes a constraint without changing its `is_hard` metadata.
Changing weights on a live `WeightedMode` director takes effect after
`reset()`.

### Filter Traits

All `Send + Sync`:
- `UniFilter<S, A>` — `fn test(&self, solution: &S, a: &A) -> bool`
- `BiFilter<S, A, B>` — `fn test(&self, solution: &S, a: &A, b: &B, a_idx: usize, b_idx: usize) -> bool`
- `TriFilter<S, A, B, C>` — `fn test(&self, solution: &S, a: &A, b: &B, c: &C, a_idx: usize, b_idx: usize, c_idx: usize) -> bool`
- `QuadFilter<S, A, B, C, D>` — `fn test(&self, solution: &S, a: &A, b: &B, c: &C, d: &D, a_idx: usize, b_idx: usize, c_idx: usize, d_idx: usize) -> bool`
- `PentaFilter<S, A, B, C, D, E>` — `fn test(&self, solution: &S, a: &A, b: &B, c: &C, d: &D, e: &E, a_idx: usize, b_idx: usize, c_idx: usize, d_idx: usize, e_idx: usize) -> bool`

Joined filter indexes are semantic source indexes, not builder-local
placeholders. Same-source joins pass canonical entity indexes; cross-bi passes
left and right source indexes; cross-tri passes the A, B, and C source slice
indexes; flattened-bi passes the A source index and the
owning B source index for the flattened row; projected-bi passes each projected
row's primary owner entity index while `RowCoordinate` still owns row
orientation.

### `Joiner<A, B>` — `Send + Sync`

| Method | Signature | Note |
|--------|-----------|------|
| `matches` | `fn matches(&self, a: &A, b: &B) -> bool` | Test if pair matches |
| `and` | `fn and<J>(self, other: J) -> AndJoiner<Self, J>` | Compose joiners |

### Collector Traits

**`Collector<Input>` — `Send + Sync`**

| Associated Type | Bound | Note |
|-----------------|-------|------|
| `Value` | — | Extracted value type |
| `Result` | `Send + Sync` | Borrowed result view type |
| `Accumulator` | `Accumulator<Self::Value, Self::Result>` | Stateful accumulator |

| Method | Signature | Note |
|--------|-----------|------|
| `extract` | `fn extract(&self, input: Input) -> Self::Value` | Extract value from the borrowed stream match |
| `create_accumulator` | `fn create_accumulator(&self) -> Self::Accumulator` | Create fresh accumulator |

`Input` is the stream match shape: unary and projected grouping pass `&A` or
`&Out`, while direct cross-join grouping passes `(&A, &B)`. Stock collectors are
generic over that input shape, so direct cross joins use the same collector API
as unary streams, for example `sum(|(shift, employee): (&Shift, &Employee)| ...)`.

**`Accumulator<V, R>` — `Send + Sync`**

| Associated Type | Bound | Note |
|-----------------|-------|------|
| `Retraction` | `Send + Sync` | Token returned by accumulation and cached for exact retraction |

| Method | Signature | Note |
|--------|-----------|------|
| `accumulate` | `fn accumulate(&mut self, value: V) -> Self::Retraction` | Add owned value and return retraction token |
| `retract` | `fn retract(&mut self, retraction: Self::Retraction)` | Remove exactly the retained value represented by the token |
| `with_result` | `fn with_result<T>(&self, f: impl FnOnce(&R) -> T) -> T` | Expose current result without forcing an owned clone |
| `finish` | `fn finish(&self) -> R where R: Clone` | Convenience owned snapshot for cloneable results |
| `reset` | `fn reset(&mut self)` | Clear state |

Stock collectors include `count()`, `sum()`, `load_balance()`,
`consecutive_runs()`, `indexed_presence()`, and `collect_vec()`.
`collect_vec()` owns mapped values once and exposes them as `CollectedVec<T>`;
`T` does not need `Copy`, `Clone`, or `PartialEq`.

## Public Structs

### Score Directors

**`ScoreDirector<S, C, Desc = SolutionDescriptor, M = UnweightedMode>`** where `S: PlanningSolution`, `C: ConstraintSet<S, S::Score>`, `Desc: DirectorDescriptor<S, C>`, `M: WeightMode`
- Primary incremental scoring director. Zero-erasure.
- Public methods: `new()`, `with_descriptor()` (unweighted),
  `new_weighted()`, `with_descriptor_weighted()` (`WeightedMode`),
  `with_descriptor_in_mode()` (mode `M` from the type),
  `into_weight_mode::<M2>()` (same working solution, constraint set and
  descriptor in mode `M2`, incremental state reset), `simple()` (convenience for
  `ScoreDirector<S, ()>`), `simple_zero()` (test helper with empty descriptor),
  `working_solution()`, `working_solution_mut()`, `into_working_solution()`,
  `calculate_score()`, `before_variable_changed()`,
  `after_variable_changed()`, `do_change()`, `get_score()`, `reset()`,
  `clone_working_solution()`, `constraints()`, `constraints_mut()`,
  `constraint_metadata()`, `constraint_count()`, `is_initialized()`,
  `constraint_results()` and `constraint_analyses()` (per-constraint results
  and detailed analyses under the director's weight mode),
  `constraint_match_totals()` (`(name, weight_multiplier, weighted score,
  match_count)` per constraint), and `take_solution()`
- Returns borrowed constraint metadata views from the monomorphized `ConstraintSet` on demand.
- `simple(solution, descriptor, entity_counter)` — creates `ScoreDirector<S, ()>` with empty constraint set
- `simple_zero(solution)` — creates `ScoreDirector<S, ()>` with empty descriptor and zero entity counter
- `with_descriptor(solution, constraints, descriptor: Desc, entity_counter)` — `Desc` is an owned `SolutionDescriptor` or a `ReplicaDescriptor`; `new()`, `simple()` and `simple_zero()` build owned-descriptor directors.
- `Director::fork_replica` forks only directors holding a `ReplicaDescriptor`, in the same weight mode: the replica clones the working solution, builds its constraint set from that clone with the recorded constructor, initializes it, and shares the same borrowed descriptor. Owned-descriptor directors return `None`.
- `reset()` discards incremental state after an external rewrite of the working solution. Directors holding a `ReplicaDescriptor` rebuild their constraint set from the current working solution with the recorded constructor, so solution-derived constraint plans follow fact and entity changes; owned-descriptor directors reset their existing set.
- Implements `Director<S>`

**`WeightMode`** (sealed trait) — `const WEIGHTED: bool`; implemented only by `UnweightedMode` (zero-sized, no weight storage) and `WeightedMode` (bound per-slot multipliers).

**`requires_weighted_mode<S: PlanningSolution>(solution: &S) -> bool`** — `S::HAS_CONSTRAINT_WEIGHTS` and some multiplier of `solution.constraint_weights()` differs from `1`.

**`DirectorDescriptor<S, C>`** (trait) — how a `ScoreDirector` holds its descriptor.
- `solution_descriptor(&self) -> &SolutionDescriptor`
- `replica(&self) -> Option<(Self, fn(&S) -> C)>` — the handle and constraint-set constructor for an evaluator replica; `None` for an owned `SolutionDescriptor`.

**`ReplicaDescriptor<'d, S, C>`** — `Copy`; a borrowed `&'d SolutionDescriptor` plus the constraint-set constructor `fn(&S) -> C`.
- `new(descriptor: &'d SolutionDescriptor, constraints: fn(&S) -> C)`

### Constraint Types

All implement `IncrementalConstraint<S, Sc>`.

**`IncrementalUniConstraint<S, A, E, F, W, Sc>`** — Single-collection constraint with filter and weight.

**`OperatorTerminal<S, O, W, Sc>`** — Generic scoring terminal over a concrete operator tree, independent of row arity. Same-collection self-joins finalize here over a `SelfJoinNode`, so bi/tri/quad/penta share one terminal instead of per-arity engines.

**`ListPrecedenceMakespanConstraint<S>`** — Stock incremental
list-plus-fixed-precedence constraint over `HardSoftScore`. `new(...)` binds the
list descriptor, node/owner accessors, durations, and fixed successors;
`with_expected_owner(...)` optionally adds fixed-owner validation.

**`stream::cross::{Bi, Tri}` finalization** — Cross-collection joins finalize onto the generic `constraint::relational::OperatorTerminal`. Each join is a concrete `JoinNode` over a compiled condition plan: the fluent `.join((target, joiner))` carries its own key/condition type, and a row-aware joiner receives the whole borrowed left row. Stateless `evaluate()`, `match_count()`, and `get_matches()` traverse the operator tree directly, so retained analysis works before `initialize()`. Authored bi/tri filters run inside the scored operator, so filters receive the A/B/C source indexes on every path (direct, grouped, and projected) and are applied exactly once across evaluate and retained mutations.

**Grouped terminals** — group-by with collector and weight on `(&K, &R)`, finalized as `SharedOperatorTerminal` over `GroupNode`.

`SharedOperatorTerminal` owns the operator once and refreshes each terminal from
changed group rows.
Additional grouped terminals append through the same fluent
`.penalize(...).named(...)` / `.reward(...).named(...)` finalization chain,
so macro-generated sharing does not use a separate stream construction path.

**Direct grouped cross-join constraints** finalize into `SharedOperatorTerminal` over `JoinNode` +
`GroupNode`. The common operators own join indexes, match rows, group accumulators, exact
retraction tokens, and changed-group reporting; no cross-grouped execution engine exists.
Chained terminals share that one operator and stay independently scored and explained.

**Direct grouped cross-join complements** finalize into `SharedOperatorTerminal` over
`ComplementNode(CollectionNode, GroupNode)`. Complement targets keep their default rows until a
real match appears, duplicate target rows keep bag multiplicity, and each terminal weights key
plus collector result independently.

**`BalanceConstraint<S, A, K, E, F, KF, Sc>`** — Load balancing using sum-of-squared-deviations.

**`ComplementNode<S, L, R, P, F, T>`** (operator layer) — Target-domain left outer join with owned defaults, over a grouped result domain. `group_by().complement()` finalizes into this node inside the shared `OperatorTerminal`.

**`constraint::projected::Uni<S, Out, Src, F, W, Sc>`** — Terminal constraint for scoring retained projected rows one row at a time.

**`constraint::projected::Bi<S, Out, K, Src, F, KF, PF, W, Sc>`** — Symmetric self-join constraint over retained projected rows. Pair ordering is coordinate-stable by `RowCoordinate`; pair-filter indexes are the projected rows' primary owner entity indexes, never retained storage row IDs. It is produced by `stream::projected::Stream::join(equal(...))`.

**`constraint::projected::DirectedBi<S, Out, K, Src, F, KL, KR, PF, W, Sc>`** — Directed self-join constraint over retained projected rows with distinct left and right key extractors. It is produced by `stream::projected::Stream::join(equal_bi(left_key, right_key))`, evaluates oriented pairs where the left key equals the right key, and skips only the same retained row.

**`constraint::projected::Grouped<S, Out, K, Src, F, KF, C, V, R, Acc, W, Sc>`** where `C: Collector<&Out>` — Grouped retained projected rows.

Projected grouped constraints finalize into `SharedOperatorTerminal` over
`GroupNode` + `SourceNode`. The common operators own projected rows, owner
indexes, retraction tokens, and group accumulators; chained terminals share one
group owner and stay independently scored.

**`constraint::projected::ComplementedGrouped<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D, W, Sc>`** where `C: Collector<&Out>` — Projected grouped rows complemented against a second collection, including `join(...).project(...).group_by(...).complement(...)` chains. It finalizes into `SharedOperatorTerminal` over `ComplementNode(CollectionNode, GroupNode)`; complement targets keep default rows until matched and each terminal scores independently.

**`FlattenedBiConstraint<S, A, B, C, K, CK, EA, EB, KA, KB, Flatten, CKeyFn, ALookup, F, W, Sc>`** — Cross-collection with nested collection flattening. Filters receive the A source index and the owning B source index for each flattened C row.

**`IncrementalExistsConstraint<S, A, P, B, K, EA, EP, KA, KB, FA, FP, Flatten, W, Sc>`** — Existence/non-existence check over a source-aware direct or flattened collection source. The constraint owns one scoring algorithm and delegates only key bookkeeping to an internal `ExistsKeyState`: exact `usize` keys use indexed `Vec` storage, while all other key types use hashed storage.

**`ExistenceMode`** — `enum { Exists, NotExists }`

#### Shared grouped-state support

The grouped compiler surface deliberately exposes concrete retained-state and
builder carriers so `#[solverforge_constraints]` can append terminals without
erasing types. Unary sharing returns `GroupedConstraintSetBuilder`; direct
cross and projected sharing return their module-specific `GroupedSetBuilder`;
complemented direct/projected sharing returns
`ComplementedGroupedSetBuilder`. The corresponding immutable analysis views
are module-specific `GroupedEvaluationState` and
`ComplementedGroupedEvaluationState` values.

`GroupedStateView<K, R>` / `ComplementedGroupedStateView<K, R>` expose retained
group results to scorers. `GroupedScorerSet<K, R, Sc>` /
`ComplementedGroupedScorerSet<K, R, Sc>` are the concrete tuple-scorer
dispatch contracts, and `ComplementedGroupedTerminalScorer` is the module-local
alias of the common grouped terminal scorer. These support symbols are
module-level compiler bridges (several are `#[doc(hidden)]`); they are not
crate-root modeling APIs and do not introduce runtime node caches.

Shared grouped node states expose `update_count()`, `changed_key_count()`, and
`evaluation_state()` for compiler diagnostics. `GroupedTerminalScorer` exposes
`refresh_all()`, `refresh_changed()`, and `refresh_count()` for the concrete
shared-set dispatch path.

**`SharedNodeId(pub usize)`** — Stable diagnostic identifier for one shared
compiler node.

**`SharedNodeOperation`** — `Grouped`, `ProjectedGrouped`, `CrossGrouped`,
`CrossComplementedGrouped`, or `ProjectedComplementedGrouped`.

**`SharedNodeDiagnostics`** — Public fields `{ id, fingerprint, operation,
terminal_consumers, update_count, changed_key_count }`; construct with `new(...)`.

### Analysis Types

Constraints own their `ConstraintRef` once. Metadata and analysis types borrow that identity so package-qualified constraint names remain intact without cloning `ConstraintRef` in scoring or reporting paths.

**`ConstraintResult<'a, Sc>`** — `{ name: &'a str, score: Sc, match_count: usize, is_hard: bool, weight_multiplier: u32 }`

**`ConstraintMetadata<'a>`** — `{ constraint_ref: &'a ConstraintRef, is_hard: bool }`; `name()` returns the short constraint name, and `full_name()` returns the package-qualified identity used for exact matching.

**`EntityRef`** — Public fields `{ type_name: String, display: String }` plus a private cloned entity snapshot for typed downcast access.
- Methods: `new()`, `with_display()`, `as_entity::<T>()`, `short_type_name()`
- Implements `Hash + Eq` (by display string)

**`ConstraintJustification`** — `{ entities: Vec<EntityRef>, description: String }`.
Construct with `new(entities)` or `with_description(entities, description)`.

**`DetailedConstraintMatch<'a, Sc: Score>`** — `{ constraint_ref: &'a ConstraintRef, score: Sc, justification: ConstraintJustification }`

**`DetailedConstraintEvaluation<'a, Sc: Score>`** — `{ total_score: Sc, match_count: usize, matches: Vec<DetailedConstraintMatch<'a, Sc>> }`

**`ConstraintAnalysis<'a, Sc: Score>`** — `{ constraint_ref: &'a ConstraintRef, weight_multiplier: u32, score: Sc, matches: Vec<DetailedConstraintMatch<'a, Sc>>, is_hard: bool }`; `new(constraint_ref, weight_multiplier, score, matches, is_hard)`

**`ScoreExplanation<'a, Sc: Score>`** — `{ score: Sc, constraint_analyses: Vec<ConstraintAnalysis<'a, Sc>> }`
- Methods: `total_match_count()`, `non_zero_constraints()`, `all_matches()`

**`Indictment<'a, Sc: Score>`** — `{ entity: EntityRef, score: Sc, constraint_matches: HashMap<&'a ConstraintRef, Vec<DetailedConstraintMatch<'a, Sc>>> }`
- Methods: `add_match()`, `match_count()`, `violated_constraints()`, `constraint_count()`

**`IndictmentMap<'a, Sc: Score>`** — `{ indictments: HashMap<EntityRef, Indictment<'a, Sc>> }`
- Methods: `from_matches()`, `get()`, `entities()`, `worst_entities()`, `len()`, `is_empty()`

**`FixedWeight<Sc>` / `fixed_weight(score)`** — Public zero-erasure wrapper for custom fixed score weights. Use `penalize(fixed_weight(custom_score))` or `reward(fixed_weight(custom_score))` when `Sc` is user-defined.

**`HardWeight<W>` / `hard_weight(weight)`** — Public zero-erasure wrapper that forces hard constraint metadata while delegating scoring to the wrapped fixed or dynamic weight.

Dynamic closure weights are non-hard metadata by default, even when their score type has a hard level. Wrap with `hard_weight(...)` when analysis metadata must report the constraint as hard.

### Stream Builders (Fluent API)

**`ConstraintFactory<S, Sc: Score>`** — Entry point.
- `new()`, `for_each()` → `UniConstraintStream`
- Generated solution source methods pass `for_each()` hidden descriptor/static source metadata.

**`UniConstraintStream<S, A, E, F, Sc>`** — Single collection stream.
- Operations: `filter()`, `unassigned()` when the entity implements hidden `UnassignedEntity<S>`, `join(target)` (single dispatch via `JoinTarget`), `group_by()`, `balance()`, `project(projection)` → `stream::projected::Stream`, `flattened(flatten)` → `FlattenedCollectionTarget`, `if_exists(target)`, `if_not_exists(target)`, `penalize(weight_or_fn)`, `reward(weight_or_fn)`
- `UniConstraintStream` implements `CollectionExtract` by delegating extraction to its source and applying its accumulated filter through `contains(...)`.
- Stream targets preserve their own source filters when passed to keyed or predicate cross-joins. This lets `.join((ConstraintFactory::new().for_each(source).filter(pred), equal_bi(...)))` keep the right-side source predicate inside the joined stream.
- For the first unary keyed equality cross-join, the left unary filter is preserved through hidden source membership before key extraction and pair creation. Original source indexes and change metadata remain intact across incremental membership transitions. Post-join filters remain joined-row filters. This left-filter pushdown does not apply to comparison, overlap, composed-condition, predicate, or same-source self-join dispatch.
- `join()` dispatch: `equal(|a| key)` → self-join `BiConstraintStream`; `(extractor_b, equal_bi(ka, kb))` → keyed `stream::cross::Bi` (`BiUnaryPlan`); `(extractor_b, less_than/greater_than/…_or_equal(a, b))` → comparison `stream::cross::Bi` (`ViewComparisonPlan`); `(extractor_b, overlapping(sa, ea, sb, eb))` → overlap `stream::cross::Bi` (`ViewOverlapPlan`); `(extractor_b, a.and(b))` → composed `stream::cross::Bi` (operands convert via `ToViewPlan` into one concrete conjunction, `EqualityWithResidual` when equality keys the probe); `(other_stream, |a, b| pred)` → predicate `stream::cross::Bi` (`BiPredicatePlan`, an explicit opposite-input scan — never a synthetic constant equality key)
- `into_parts()` → `(E, F)`, `from_parts(extractor, filter)` → `Self`, `extractor()` → `&E`

**`UniConstraintBuilder<S, A, E, F, W, Sc>`** — `named()` → `IncrementalUniConstraint`

**`Projection<A>`** — Retained projection contract for single-source `.project(...)`. Implementations define `type Out`, `const MAX_EMITS: usize`, and `project(&self, input: &A, sink: &mut impl ProjectionSink<Self::Out>)`. Projection implementations emit bounded scoring rows into the sink; Vec-returning closures are not part of the API. `Out` does not need `Clone`.

The fluent projected stream keeps its concrete source type in the return type:
`SingleSource` for one projection, `FilteredSource` after a projected filter,
`MergedSource` after merging compatible projected streams, and `JoinedSource`
for a projected cross join. `JoinedState` is that source's retained keyed
index. Their constructors are crate-private and they are not root exports;
callers obtain these opaque carriers through fluent operations and continue on
`stream::projected::Stream`.

**`ProjectionSink<Out>`** — Emission sink used by `Projection<A>` implementations. `emit(output)` is the only projection output channel.

**`stream::projected::Stream<S, Out, Src, F, Sc>`** — Scoring rows from one or more source streams. Single-source output type is inferred from the named projection type passed to `project(...)`; keyed cross joins use `stream::cross::Bi::project(|left, right| row)` and emit exactly one scoring row per retained joined pair. Retained rows are cached by `RowCoordinate` and indexed by one or two `RowOwner` values. Single-source projected rows update incrementally from their source owner; joined-pair projected rows update incrementally from either joined source when that source is descriptor-localized. Symmetric projected self-join pair order follows `RowCoordinate` ordering; pair-filter indexes use each row's primary owner entity index, and retained storage row IDs are internal and never semantic. Projected rows can be self-joined by `equal(|row| key)` or by directed `equal_bi(left_key, right_key)` without materialized facts, and projected output rows plus projected self-join keys do not need `Clone`. Raw `for_each` extractors with `ChangeSource::Unknown` can evaluate and initialize projected constraints, but localized incremental callbacks panic because their entity indexes cannot be mapped safely.
- Operations: `filter()`, `merge(other)`, `group_by()`, `join(equal(...))`, `join(equal_bi(...))`, `penalize(weight_or_fn)`, `reward(weight_or_fn)`

**`stream::projected::Builder`** — `named()` → `constraint::projected::Uni`

**`stream::projected::Bi<S, Out, K, Src, F, KF, PF, Sc>`** — Self-join stream over projected rows produced by `stream::projected::Stream::join(equal(...))`.
- Operations: `filter()`, `penalize(weight_or_fn)`, `reward(weight_or_fn)`

**`stream::projected::BiBuilder`** — `named()` → `constraint::projected::Bi`

**`stream::projected::DirectedBi<S, Out, K, Src, F, KL, KR, PF, Sc>`** — Directed self-join stream over projected rows produced by `stream::projected::Stream::join(equal_bi(left_key, right_key))`.
- Operations: `filter()`, `penalize(weight_or_fn)`, `reward(weight_or_fn)`

**`stream::projected::DirectedBiBuilder`** — `named()` → `constraint::projected::DirectedBi`

**`RowCoordinate`** — Hidden support coordinate for projected rows:
`{ primary_owner, secondary_owner, emit_index }`. `primary_owner` is always
present; `secondary_owner` is present for joined-pair rows from
`stream::cross::Bi::project(...)`. It is used to keep projected self-join
orientation stable across sparse row-slot reuse and to dedupe callbacks when
both owners localize to the same descriptor/entity update.

Projection syntax:

```rust
struct AssignmentLoadEntries;

impl Projection<Assignment> for AssignmentLoadEntries {
    type Out = LoadEntry;
    const MAX_EMITS: usize = 4;

    fn project<Sink>(&self, assignment: &Assignment, out: &mut Sink)
    where
        Sink: ProjectionSink<Self::Out>,
    {
        assignment.for_each_load_entry(|entry| out.emit(entry));
    }
}

ConstraintFactory::<Plan, HardSoftScore>::new()
    .for_each(Plan::assignments())
    .project(AssignmentLoadEntries)
```

**`stream::projected::Grouped` / `stream::projected::GroupedBuilder`** — Grouped projected rows using stock collectors such as `sum()`, `count()`, `collect_vec()`, `consecutive_runs()`, and `indexed_presence()`. Grouped retained state uses the same `RowOwner` ownership index as ungrouped projected rows. Collector values do not need `Clone`; retained grouped state stores the projected row once by `RowCoordinate` and caches accumulator retraction tokens for exact retracts. Grouped weights use the canonical `penalize(|key, result| ...)` / `reward(|key, result| ...)` shape. `complement()` and `complement_with_key()` continue to `stream::projected::ComplementedGrouped`; `named()` → `constraint::projected::Grouped`.

**`BiConstraintStream<S, A, K, E, KE, F, Sc>`** — Self-join bi stream (macro-generated).
- Operations: `filter()`, `join()` → `TriConstraintStream`, `penalize(weight_or_fn)`, `reward(weight_or_fn)`
- Low-level constructors: `new_self_join()`, `new_self_join_with_filter()`

**`BiConstraintBuilder<S, A, K, E, KE, F, W, Sc>`** — `named()` → `OperatorTerminal` over a `SelfJoinNode` + `FilterNode` tree

**`TriConstraintStream/Builder`** — Same pattern, tri-arity. `join()` →
`QuadConstraintStream`; low-level constructors are `new_self_join()` and
`new_self_join_with_filter()`.

**`QuadConstraintStream/Builder`** — Same pattern, quad-arity. `join()` →
`PentaConstraintStream`; low-level constructors are `new_self_join()` and
`new_self_join_with_filter()`.

**`PentaConstraintStream/Builder`** — Same pattern, penta-arity. Terminal (no
further joins); low-level constructors are `new_self_join()` and
`new_self_join_with_filter()`.

**`stream::cross::Bi<S, A, B, P, EA, EB, F, Sc>`** — Cross-collection bi stream, generic over its compiled first-relationship plan `P` (`BiUnaryPlan` for a keyed join, `BiPredicatePlan` for a predicate join).
- Operations: `filter()`, `group_by(|left, right| key, collector)` → `stream::cross::Grouped`, `project(|left, right| row)` → `stream::projected::Stream`, `penalize(weight_or_fn)`, `reward(weight_or_fn)`, `flatten_last()` → `FlattenedBiConstraintStream`. The group/project/flatten methods are specialized to the keyed plan, which is where a unary key pair exists to rederive.
- Low-level constructors: `new()`, `new_with_filter()` (keyed)

**`stream::cross::Builder`** — `named()` → `constraint::relational::OperatorTerminal` over a `JoinNode` compiled from the stream's unary key pair

**`stream::cross::Grouped/Builder`** — Direct grouped cross-join stream. `penalize(weight_or_fn)`, `reward(weight_or_fn)`, `named()` → `SharedOperatorTerminal` over `JoinNode` + `GroupNode`. `complement(source, key, default)` → `stream::cross::ComplementedGrouped`. Collectors receive the joined pair shape as `(&A, &B)`.

**`stream::cross::ComplementedGrouped/Builder`** — Direct grouped cross-join complement stream. `penalize(weight_or_fn)`, `reward(weight_or_fn)`, `named()` → `SharedOperatorTerminal` over `ComplementNode`. Complement defaults are produced from the complement entity and weighted by key plus collector result. Complement sources use the same `CollectionExtract::contains(...)` membership contract as joined sources.

**`GroupedConstraintStream<S, A, K, E, Fi, KF, C, V, R, Acc, Sc>`** — Grouped stream.
- Operations: `penalize(weight_or_fn)`, `reward(weight_or_fn)`, `join((collection, key))` → `GroupedJoinedStream`, `complement()`, `complement_with_key()` → `ComplementedConstraintStream`
- Dynamic weighted operations use one canonical key-aware closure shape: `Fn(&K, &R) -> Sc`.
- `join(...)` relates a group (key + aggregate) to a second collection by the group key; the weight receives `Fn(&K, &R, &B) -> Sc` (group key, aggregate, matched entity) and finalizes to an `OperatorTerminal` over a `JoinNode(GroupNode(FilterNode(source)))` tree.

**`GroupedJoinedStream/Builder`** — Group-as-join-input. `penalize`/`reward` take `Fn(&K, &R, &B) -> Sc`; `named()` → `OperatorTerminal` over a `JoinNode` + `GroupNode` + `FilterNode` tree.

**`GroupedConstraintBuilder<S, A, K, E, Fi, KF, C, V, R, Acc, W, Sc>`** — `named()` → `SharedOperatorTerminal` over `GroupNode` (group-as-join-input uses `GroupedJoinedStream`)

**`BalanceConstraintStream/Builder`** — Balance stream. `penalize(weight)`, `reward(weight)`, `named()` → `BalanceConstraint`

**`ComplementedConstraintStream/Builder`** — Complemented stream. `penalize(weight_or_fn)`, `reward(weight_or_fn)`, `named()` → `OperatorTerminal` over a `ComplementNode` + `GroupNode` tree. Dynamic weighted operations receive the real or complemented key as `Fn(&K, &R) -> Sc`.

**`stream::projected::ComplementedGrouped/Builder`** — Projected grouped complement stream. `penalize(weight_or_fn)`, `reward(weight_or_fn)`, `named()` → `constraint::projected::ComplementedGrouped`. Complement defaults are produced from the complement entity and weighted by key plus collector result.

**`FlattenedBiConstraintStream/Builder`** — Flattened bi stream. `filter()`, `penalize(weight_or_fn)`, `reward(weight_or_fn)`, `named()` → `FlattenedBiConstraint`. Low-level filters receive the A source index and the B owner index for the flattened C row.

**`ExistsConstraintStream/ExistsConstraintBuilder`** — Existence stream over source-aware direct or flattened collection targets. `penalize(weight_or_fn)`, `reward(weight_or_fn)`, `named()` → `IncrementalExistsConstraint`. There is no separate public indexed existence stream; storage selection is internal to `IncrementalExistsConstraint`.

### Extractor Types

**`CollectionExtract<S>`** — Trait for extracting an entity slice from the solution. All `E`/`EA`/`EB` type params in streams and constraints are bounded by `CollectionExtract<S, Item = A>` rather than raw `Fn(&S) -> &[A]`, allowing both closure forms.
- Associated type: `type Item` — the entity type yielded.
- Method: `fn extract<'s>(&self, s: &'s S) -> &'s [Self::Item]`
- Method: `fn contains(&self, s: &S, item: &Self::Item) -> bool` — source-level membership predicate; plain extractors default to `true`, while `UniConstraintStream` delegates to its accumulated source filter.
- Blanket impl for `Fn(&S) -> &[A] + Send + Sync` — plain slice closures `|s| s.field.as_slice()` work directly.

**`VecExtract<F>`** — Wraps `Fn(&S) -> &Vec<A>` closures so they satisfy `CollectionExtract<S>`. Construct via `vec(f)`.
- Users can write `|s| &s.field` without `.as_slice()`.

**`vec(f)`** — Free function: `fn vec<S, A, F>(f: F) -> VecExtract<F>`. Use when the extractor closure returns `&Vec<A>`:
```rust
factory.for_each(vec(|s: &Schedule| &s.employees))
// or in a join:
.join((vec(|s: &Schedule| &s.employees), equal_bi(...)))
```

**`SourceExtract<E>`** — Hidden descriptor-aware collection extraction wrapper used by macro-generated solution source methods. It satisfies `CollectionExtract<S>` and preserves source metadata for raw keyed joins.

**`ChangeSource`** — Hidden enum describing whether a stream source can localize descriptor-owned incremental callbacks: `Unknown`, `Static`, or `Descriptor(idx)`. `Descriptor(idx)` owns localized events for that descriptor. `Static` never localizes. `Unknown` is non-localized metadata for raw/manual extraction: it is valid for `evaluate()` and `initialize()`, but localized `on_insert(...)` / `on_retract(...)` callbacks panic because the entity index cannot be safely mapped to a source.

**`source(...)`** — Hidden constructor for descriptor-aware collection extraction used by macro-generated solution source methods. Planning entity collections carry `ChangeSource::Descriptor(idx)`; static fact and list-element collections carry `ChangeSource::Static`. This symbol is not part of the facade stream workflow.

**`FlattenExtract<P>`** — Trait for flattening a parent entity into a child slice for existence filtering. Blanket impl for `Fn(&P) -> &[B] + Send + Sync`; `FlattenVecExtract<F>` adapts `Fn(&P) -> &Vec<B>`.

**`ExistenceTarget<S, A, E, F, Sc>`** — Trait for `.if_exists(...)` / `.if_not_exists(...)` dispatch on `UniConstraintStream`.
- Direct target: `(other_stream, equal_bi(left_key, right_key))`
- Flattened target: `(parent_stream, flatten, equal_bi(left_key, flattened_key))`

**`FlattenedCollectionTarget<S, P, B, EP, FP, Flatten, Sc>`** — Intermediate existence target produced by `UniConstraintStream::flattened(flatten)` for nested collection membership checks.

### Join Support Types

**`JoinTarget<S, A, E, F, Sc>`** — Trait for `.join()` dispatch on `UniConstraintStream`.
- Impl groups: `EqualJoiner<KA, KA, K, Symmetric>` (self-join from `equal(...)`), any `CollectionExtract` target with `EqualJoiner<KA, KB, K, Mode>` (keyed cross-join from `equal_bi(...)`, producing `BiUnaryPlan`), comparison joiners (`LessThanJoiner`, `LessThanOrEqualJoiner`, `GreaterThanJoiner`, `GreaterThanOrEqualJoiner`) and `OverlappingJoiner` (each converting an entity-authored condition to a leaf-view plan), `(EB, AndJoiner<J1, J2>)` (composed conditions via `ToViewPlan`), and `(UniConstraintStream<...>, P)` (predicate cross-join with filtered stream target).

**`ToViewPlan<A, B>`** — Converts an entity-authored condition into the leaf-view plan the operator tree executes. Implemented for `EqualJoiner`, the four comparison joiners, `OverlappingJoiner`, and `AndJoiner<J1, J2>` (composing both operands). Used by the conjunction dispatch; top-level equality/comparison/overlap impls build their plans directly so the equality path keeps producing `BiUnaryPlan`.

**`ViewComparisonPlan<K, KA, KB, const LESS: bool, const INCLUSIVE: bool>`** — Ordered comparison over leaf views. `LESS` selects `left < right` (else `>`); `INCLUSIVE` widens to `<=`/`>=`.

**`ViewOverlapPlan<K, SA, EA, SB, EBd>`** — Interval overlap over leaf views.

**`ProjectedJoinTarget<S, Out, Src, F, Sc>`** — Trait for `.join()` dispatch on `stream::projected::Stream`.
- `equal(|row| key)` dispatches to `stream::projected::Bi` and preserves symmetric coordinate-stable pair ordering.
- `equal_bi(left_key, right_key)` dispatches to `stream::projected::DirectedBi` and preserves left/right key orientation for projected rows of the same output type.

**`KeyExtract<S, A, K>`** — Trait for key extraction. Blanket impl for `Fn(&S, &A, usize) -> K + Send + Sync`. Used as the bound on `KE` type params in nary stream/constraint macros.
- Method: `fn extract(&self, s: &S, a: &A, idx: usize) -> K`

**`EntityKeyAdapter<KA>`** — Wraps `KA: Fn(&A) -> K` as a `KeyExtract`. Used in self-join `JoinTarget` impl to adapt entity-only key functions.
- `new(key_fn: KA)` → `EntityKeyAdapter<KA>`

`PhantomKey<T>` is the zero-sized type carrier used in inferred stream return
types when a key parameter must not inherit `Send`, `Sync`, or `Clone` bounds.
It has no public constructor and is not a crate-root modeling symbol.

### Filter Types

**`TrueFilter`** — Always-true filter. Implements all filter traits.

**`FnUniFilter<F>`**, **`FnBiFilter<F>`**, **`FnTriFilter<F>`**, **`FnQuadFilter<F>`**, **`FnPentaFilter<F>`** — Closure wrappers. Joined-arity wrappers are index-aware and receive the same index arguments as their filter traits.

**`AndUniFilter<F1,F2>`**, **`AndBiFilter<F1,F2>`**, **`AndTriFilter<F1,F2>`**, **`AndQuadFilter<F1,F2>`**, **`AndPentaFilter<F1,F2>`** — Conjunctive composition.

**`UniBiFilter<F, A>`** — Adapts UniFilter to BiFilter (tests both args same predicate).

**`UniLeftBiFilter<F, B>`** — Adapts UniFilter to BiFilter (tests left arg only).

**`UniPairFilter<L, R>`** — Internal adapter that composes the left and right stream membership filters for a predicate join, whose relationship predicate is carried by `BiPredicatePlan` (not re-run as a residual).

### Joiner Types

**`EqualJoiner<Fa, Fb, T, Mode>`** — Join by key equality. `Mode` is `Symmetric` for `equal(...)` and `Directed` for `equal_bi(...)`.
- Factory: `equal(key_fn)`, `equal_bi(left, right)`
- Methods: `key_a()`, `key_b()`, `into_keys()`, `key_extractors()`

**`LessThanJoiner<Fa, Fb, T>`**, **`LessThanOrEqualJoiner<Fa, Fb, T>`**, **`GreaterThanJoiner<Fa, Fb, T>`**, **`GreaterThanOrEqualJoiner<Fa, Fb, T>`** — Comparison joiners.
- Factories: `less_than()`, `less_than_or_equal()`, `greater_than()`, `greater_than_or_equal()`

**`FilteringJoiner<F>`** — Arbitrary predicate joiner.
- Factory: `filtering(predicate)`

**`OverlappingJoiner<Fsa, Fea, Fsb, Feb, T>`** — Interval overlap detection.
- Factory: `overlapping(start_a, end_a, start_b, end_b)`

**`AndJoiner<J1, J2>`** — Composed joiner.

**`FnJoiner<F>`** — Raw function joiner.

### Collector Types

**`CountCollector`** / **`CountAccumulator`** — Counts stream matches. Factory: `count()`

**`SumCollector<T, F>`** / **`SumAccumulator<T>`** — Sums mapped values. Factory: `sum(mapper)`

**`LoadBalanceCollector<K, F, M>`** / **`LoadBalanceAccumulator<K>`** / **`LoadBalance<K>`** — Load balance with unfairness metric.
- Factory: `load_balance(key_fn, metric_fn)`
- `LoadBalance<K>` has `loads()` and `unfairness()` methods.

**`RunsCollector<F>`** / **`RunsAccumulator`** / **`Run`** / **`Runs`** — Consecutive unique `i64` point runs with duplicate item accounting.
- Factory: `consecutive_runs(index_fn)`
- `Run` exposes `start()`, `end()`, `point_count()`, and `item_count()`.
- `Runs` exposes `runs()`, `point_count()`, `item_count()`, `len()`, and `is_empty()`.

**`IndexedPresenceCollector<F>`** / **`IndexedPresenceAccumulator`** / **`IndexedPresence`** — Generic ordinal presence with active and complement runs.
- Factory: `indexed_presence(index_fn)`
- `IndexedPresence` exposes `runs()`, `complement_runs(range)`, `contains(index)`, `count()`, `item_count()`, `is_empty()`, `any_in(range)`, and `count_in(range)`.

**`CollectVecCollector<T, F>`** / **`CollectVecAccumulator<T>`** / **`CollectedVec<T>`** — Retains mapped values once and exposes them through an insertion-order iterable view.
- Factory: `collect_vec(mapper)`
- `CollectedVec<T>` exposes `iter() -> CollectedVecIter<'_, T>`, `len()`,
  `is_empty()`, and `to_vec()` when `T: Clone`. `CollectedVecIter` yields
  `&T` in retained insertion order.

## Architectural Notes

### Zero-Erasure Constraint Pipeline

The entire pipeline from stream builder to constraint evaluation is fully monomorphized:
1. `ConstraintFactory::new().for_each(extractor)` — creates `UniConstraintStream`
2. `.filter(predicate)` — composes filter via `AndUniFilter`
3. `.penalize(weight)` — creates `UniConstraintBuilder`
4. `.named("name")` — produces `IncrementalUniConstraint<S, A, E, impl Fn, W, Sc>`

All closures are stored as concrete generic type parameters. No `Box<dyn Fn>`, no `Arc`. The constraint types carry the full closure types through their generics.

### Incremental Scoring Protocol

Score directors use a retract-then-insert protocol:
1. `before_variable_changed()` → constraint `on_retract()` (remove entity's contribution)
2. Modify the solution
3. `after_variable_changed()` → constraint `on_insert()` (add new contribution)

The `ScoreDirector` delegates to `ConstraintSet::on_retract_all()` / `on_insert_all()`.

### ConstraintSet Tuple Implementation

`ConstraintSet` is implemented for every singleton `IncrementalConstraint<S, Sc>` and for tuples of up to 32 nested `ConstraintSet` elements via a macro. Operations iterate over all nested sets, summing scores and flattening per-constraint raw metadata/results without erasure; the public metadata view is deduplicated after ordering. This is the zero-erasure alternative to `Vec<Box<dyn Constraint>>`.

### Self-Join Operator

Same-collection n-ary streams build a `SelfJoinNode<S, A, O, K, KE, N>` over
their collection source. It buckets input rows by key, emits every increasing
handle tuple of arity `N` per bucket (unique ordered combinations, so a triple
is `i < j < k` for one key), and maintains exact insert/retract deltas over a
retained combo set. The view is `[Leaf<'a, A>; N]` in canonical index order.
The authored filter runs in a `FilterNode` on top, and the terminal is the
shared `OperatorTerminal`. No per-arity incremental engine remains.

### Stream Arity Macros

`impl_bi_arity_stream!`, `impl_tri_arity_stream!`, `impl_quad_arity_stream!`, `impl_penta_arity_stream!` generate the stream and builder structs for each arity level. All four macros live under `arity_stream_macros/nary_stream/`. They share the same field layout and method pattern but differ in the number of entity arguments to filter/weight functions. Each passes an explicit `arity` literal and a canonical position table used to unpack the `[Leaf; N]` self-join view.

### PhantomData Pattern

All types use `PhantomData<(fn() -> S, fn() -> A, ...)>` to avoid inheriting bounds from phantom type parameters.

## Cross-Crate Dependencies

- **From `solverforge-core`:** `Score`, `PlanningSolution`, `ConstraintRef`, `ImpactType`, `SolutionDescriptor`, `EntityDescriptor`, `ProblemFactDescriptor`, `VariableDescriptor`, `EntityCollectionExtractor`
