# solverforge-scoring WIREFRAME

Zero-erasure incremental constraint scoring infrastructure for SolverForge.

**Location:** `crates/solverforge-scoring/`
**Workspace Release:** `0.17.0`

## Dependencies

- `solverforge-core` (path) — Score types, domain traits, descriptors, ConstraintRef, ImpactType
- `thiserror` (workspace) — error derive macros

## File Map

```
src/
├── lib.rs                                          — Crate root; re-exports from all modules
├── api/
│   ├── mod.rs                                      — Re-exports analysis, constraint_set, node_sharing, weight_overrides
│   ├── analysis.rs                                 — ScoreExplanation, ConstraintAnalysis, Indictment, IndictmentMap, DetailedConstraintMatch, etc.
│   ├── node_sharing.rs                             — SharedNodeDiagnostics, SharedNodeId, SharedNodeOperation
│   ├── constraint_set/
│   │   ├── mod.rs                                  — Re-exports ConstraintSet, ConstraintSetChain, OrderedConstraintSetChain, ConstraintSetSource, IncrementalConstraint, IncrementalConstraintSealed, ConstraintMetadata, ConstraintResult
│   │   ├── chain.rs                                — ConstraintSetChain, OrderedConstraintSetChain, and authored-order source spans
│   │   ├── incremental.rs                          — IncrementalConstraint trait, ConstraintSet trait, singleton and tuple impls (0..32)
│   │   └── tests/
│   │       ├── mod.rs                              — Test module declarations
│   │       └── constraint_set.rs                   — ConstraintSet tuple tests
│   ├── weight_overrides.rs                         — ConstraintWeightOverrides, WeightProvider trait
│   └── tests/
│       ├── mod.rs                                  — Test module declarations
│       ├── analysis.rs                             — Analysis type tests
│       └── weight_overrides.rs                     — Weight override tests
├── constraint/
│   ├── mod.rs                                      — Module declarations and selected root re-exports
│   ├── macros.rs                                   — impl_get_matches_nary! macro for detailed match generation
│   ├── shared.rs                                   — compute_hash<T>() utility function
│   ├── list_precedence.rs                          — ListPrecedenceMakespanConstraint<S>; stock incremental list-plus-fixed-precedence makespan scoring with optional fixed owner checks
│   ├── incremental_markers.rs                      — Hidden IncrementalConstraintSealed impls for built-in constraint types
│   ├── incremental.rs                              — IncrementalUniConstraint<S,A,E,F,W,Sc>
│   ├── grouped.rs                                  — grouped::Uni module root, shared node state, terminal scorers, scorer sets, shared set
│   ├── grouped/*.rs                                — scorer.rs, complemented_scorer.rs, scorer_set.rs, shared_set.rs, state.rs, terminal.rs for GroupedNodeState, terminal scorer sets, SharedGroupedConstraintSet, and the one-terminal wrapper
│   ├── balance.rs                                  — BalanceConstraint<S,A,K,E,F,KF,Sc>
│   ├── complemented.rs                             — constraint::complemented::Grouped module root and re-exports
│   ├── complemented/*.rs                           — Retained complemented state, incremental callbacks, helpers, and debug accessors
│   ├── cross_bi_incremental.rs                     — constraint::cross_bi_incremental::Bi module root and re-exports
│   ├── cross_bi_incremental/*.rs                   — Retained cross-bi state, weights, incremental callbacks, and debug accessors
│   ├── cross_grouped.rs                            — constraint::cross_grouped::Grouped module root and re-exports
│   ├── cross_grouped/*.rs                          — indexes.rs, scorer.rs, shared_set.rs, state.rs, terminal.rs, updates.rs, view.rs for retained direct cross grouped state
│   ├── cross_complemented_grouped.rs               — constraint::cross_complemented_grouped::ComplementedGrouped module root and internal shared engine re-exports
│   ├── cross_complemented_grouped/*.rs             — builder.rs, indexes.rs, scorer.rs, shared_set.rs, state.rs, terminal.rs, updates.rs, view.rs for retained direct cross grouped complements
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
│   ├── projected/grouped.rs                        — projected::Grouped module root and shared grouped re-exports
│   ├── projected/complemented_grouped.rs           — projected::ComplementedGrouped module root and shared complemented-grouped re-exports
│   ├── projected/grouped/*.rs                      — scorer.rs, shared_set.rs, state.rs, terminal.rs for projected grouped retained state
│   ├── projected/complemented_grouped/*.rs         — indexes.rs, scorer.rs, shared_set.rs, state.rs, terminal.rs, view.rs for projected grouped complements
│   ├── nary_incremental/
│   │   ├── mod.rs                                  — Re-exports all nary constraint macros
│   │   ├── bi.rs                                   — impl_incremental_bi_constraint! macro → IncrementalBiConstraint
│   │   ├── higher_arity.rs                         — Re-exports tri/quad/penta incremental constraint macros
│   │   └── higher_arity/
│   │       ├── shared.rs                           — Shared higher-arity detailed-match helpers
│   │       ├── tri.rs                              — impl_incremental_tri_constraint! macro → IncrementalTriConstraint
│   │       ├── quad.rs                             — impl_incremental_quad_constraint! macro → IncrementalQuadConstraint
│   │       └── penta.rs                            — impl_incremental_penta_constraint! macro → IncrementalPentaConstraint
│   └── tests/
│       ├── mod.rs                                  — Test module declarations
│       ├── bi_incr.rs                              — IncrementalBiConstraint tests
│       ├── cross_bi_incr.rs                        — constraint::cross_bi_incremental::Bi tests
│       ├── tri_incr.rs                             — IncrementalTriConstraint tests
│       ├── quad_incr.rs                            — IncrementalQuadConstraint tests
│       ├── penta_incr.rs                           — IncrementalPentaConstraint tests
│       ├── grouped.rs                              — constraint::grouped::Uni and shared grouped node tests
│       ├── cross_grouped.rs                        — Shared direct cross grouped node tests
│       ├── balance.rs                              — BalanceConstraint tests
│       ├── complemented.rs                         — constraint::complemented::Grouped tests
│       ├── cross_complemented_grouped.rs           — Direct cross-join grouped complement tests
│       ├── flattened_bi.rs                         — FlattenedBiConstraint tests
│       ├── exists.rs                               — IncrementalExistsConstraint update tests
│       ├── exists_storage.rs                       — Existence storage selection and parity tests
│       ├── projected.rs                            — Projected constraint test module root
│       ├── projected/*.rs                          — Projected support fixtures, localization, update, grouping, merge, and self-join tests
│       └── repro_unknown.rs                        — Regression fixture coverage for unknown-source behavior
├── director/
│   ├── mod.rs                                      — Re-exports all director types and traits
│   ├── traits.rs                                   — Director<S> trait
│   ├── score_director.rs                           — Re-exports ScoreDirector pieces
│   │   ├── score_director/incremental.rs           — ScoreDirector<S,C> (zero-erasure incremental)
│   │   └── score_director/adapters.rs              — Debug and Director trait impls for ScoreDirector
│   ├── shadow_aware.rs                             — SolvableSolution trait and shadow lifecycle notes
│   └── tests/
│       ├── mod.rs                                  — Test module declarations
│       ├── bench.rs                                — Benchmark test module declarations
│       ├── benchmarks.rs                           — Performance comparison tests
│       ├── fixtures.rs                             — ScoreDirector fixtures
│       ├── fixtures_tests.rs                       — ScoreDirector fixture tests
│       ├── score_director.rs                       — ScoreDirector tests
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
│   ├── grouped_stream.rs                           — Re-exports
│   ├── grouped_stream/base.rs                      — GroupedConstraintStream
│   ├── grouped_stream/weighting.rs                 — GroupedConstraintBuilder
│   ├── balance_stream.rs                           — BalanceConstraintStream, BalanceConstraintBuilder
│   ├── complemented_stream.rs                      — ComplementedConstraintStream, ComplementedConstraintBuilder
│   ├── cross_bi_stream.rs                          — Re-exports
│   ├── cross_bi_stream/base.rs                     — stream::cross::Bi
│   ├── cross_bi_stream/grouped.rs                  — stream::cross::Grouped and builder
│   ├── cross_bi_stream/complemented_grouped.rs     — stream::cross::ComplementedGrouped and builder
│   ├── cross_bi_stream/weighting.rs                — stream::cross::Builder
│   ├── flattened_bi_stream.rs                      — Re-exports
│   ├── flattened_bi_stream/base.rs                 — FlattenedBiConstraintStream
│   ├── flattened_bi_stream/builder.rs              — FlattenedBiConstraintBuilder
│   ├── flattened_bi_stream/weighting.rs            — Weighting helpers for flattened streams
│   ├── existence_stream.rs                         — ExistsConstraintStream, ExistsConstraintBuilder, ExistenceMode, FlattenExtract
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
│   ├── collection_extract.rs                       — CollectionExtract trait, hidden source metadata, VecExtract wrapper, vec() constructor
│   ├── unassigned.rs                               — Hidden UnassignedEntity hook and `.unassigned()` stream method
│   ├── weighting_support.rs                        — ConstraintWeight, FixedWeight, HardWeight, and dynamic closure-weight adapters
│   ├── join_target.rs                              — JoinTarget trait impls for self-join, keyed cross-join, and predicate cross-join
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
│   │   ├── adapters.rs                             — UniBiFilter, UniLeftBiFilter, and hidden PairFilter adapters
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
pub use constraint::{
    IncrementalBiConstraint, IncrementalPentaConstraint, IncrementalQuadConstraint,
    IncrementalTriConstraint, IncrementalUniConstraint, ListPrecedenceMakespanConstraint,
};

// Short family names are intentionally module-scoped:
// constraint::grouped::Uni
// constraint::complemented::Grouped
// constraint::cross_bi_incremental::Bi
// constraint::cross_grouped::Grouped
// constraint::cross_complemented_grouped::ComplementedGrouped
// constraint::projected::{Uni, Bi, DirectedBi, Grouped, ComplementedGrouped}

// Constraint Set
pub use api::constraint_set::{
    ConstraintMetadata, ConstraintResult, ConstraintSet, IncrementalConstraint,
    IncrementalConstraintSealed,
};
pub use api::node_sharing::{SharedNodeDiagnostics, SharedNodeId, SharedNodeOperation};
pub use api::weight_overrides::{ConstraintWeightOverrides, WeightProvider};

// Score Directors
pub use director::score_director::ScoreDirector;
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
// stream::cross::{Bi, Builder, Grouped, GroupedBuilder, ComplementedGrouped, ComplementedGroupedBuilder}
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
| `reset` | `fn reset(&mut self)` | Default: no-op |

### `DirectorScoreState<Sc>`

Committed score-state snapshot used to roll back speculative evaluation. Fields:
`solution_score`, `committed_score`, `initialized`.

### `IncrementalConstraint<S, Sc: Score>` — `IncrementalConstraintSealed + Send + Sync`

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
| `get_matches` | `fn get_matches<'a>(&'a self, _solution: &S) -> Vec<DetailedConstraintMatch<'a, Sc>>` | Default: empty |
| `weight` | `fn weight(&self) -> Sc` | Default: Sc::zero() |

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

Implemented for each `IncrementalConstraint<S, Sc>` as a singleton set and for tuples `()` through `(C0, C1, ..., C31)` where each `Ci: ConstraintSet<S, Sc>`. `ConstraintSetChain<Left, Right>` composes two existing `ConstraintSet` values without erasing either concrete type. `OrderedConstraintSetChain<Left, Right>` uses `ConstraintSetSource::Left` and `ConstraintSetSource::Right { constraint_count, metadata_entry_count }` spans to preserve user-authored result ordering when macro-generated node sharing combines one shared set with surrounding constraints. Raw metadata entries keep one entry per constraint for ordering; the public `constraint_metadata()` view deduplicates repeated full `ConstraintRef`s when hardness agrees and panics when the same full ref has conflicting hard/non-hard metadata. Package-qualified constraints that share a short name remain distinct.

### Shadow Lifecycle on `PlanningSolution`

`PlanningSolution` itself owns the canonical shadow hooks:
- `update_entity_shadows(&mut self, descriptor_index: usize, entity_index: usize)` — default no-op
- `update_all_shadows(&mut self)` — default no-op

### `SolvableSolution` — `: PlanningSolution`

| Method | Signature | Note |
|--------|-----------|------|
| `descriptor` | `fn descriptor() -> SolutionDescriptor` | Static descriptor |
| `entity_count` | `fn entity_count(solution: &Self, descriptor_index: usize) -> usize` | Entity count |

### `WeightProvider<Sc: Score>` — `Send + Sync`

| Method | Signature | Note |
|--------|-----------|------|
| `weight` | `fn weight(&self, name: &str) -> Option<Sc>` | Lookup override weight |
| `weight_or_default` | `fn weight_or_default(&self, name: &str, default: Sc) -> Sc` | Default: uses weight() |

### Filter Traits

All `Send + Sync`:
- `UniFilter<S, A>` — `fn test(&self, solution: &S, a: &A) -> bool`
- `BiFilter<S, A, B>` — `fn test(&self, solution: &S, a: &A, b: &B, a_idx: usize, b_idx: usize) -> bool`
- `TriFilter<S, A, B, C>` — `fn test(&self, solution: &S, a: &A, b: &B, c: &C, a_idx: usize, b_idx: usize, c_idx: usize) -> bool`
- `QuadFilter<S, A, B, C, D>` — `fn test(&self, solution: &S, a: &A, b: &B, c: &C, d: &D, a_idx: usize, b_idx: usize, c_idx: usize, d_idx: usize) -> bool`
- `PentaFilter<S, A, B, C, D, E>` — `fn test(&self, solution: &S, a: &A, b: &B, c: &C, d: &D, e: &E, a_idx: usize, b_idx: usize, c_idx: usize, d_idx: usize, e_idx: usize) -> bool`

Joined filter indexes are semantic source indexes, not builder-local
placeholders. Same-source joins pass canonical entity indexes; cross-bi passes
left and right source indexes; flattened-bi passes the A source index and the
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

**`ScoreDirector<S, C>`** where `S: PlanningSolution`, `C: ConstraintSet<S, S::Score>`
- Primary incremental scoring director. Zero-erasure.
- Key methods: `new()`, `with_descriptor()`, `simple()` (convenience for `ScoreDirector<S, ()>`), `simple_zero()` (test helper with empty descriptor), `calculate_score()`, `before_variable_changed()`, `after_variable_changed()`, `do_change()`, `get_score()`, `constraint_metadata()`, `constraint_match_totals()`, `into_working_solution()`, `take_solution()`
- Returns borrowed constraint metadata views from the monomorphized `ConstraintSet` on demand.
- `simple(solution, descriptor, entity_counter)` — creates `ScoreDirector<S, ()>` with empty constraint set
- `simple_zero(solution)` — creates `ScoreDirector<S, ()>` with empty descriptor and zero entity counter
- Implements `Director<S>`

### Constraint Types

All implement `IncrementalConstraint<S, Sc>`.

**`IncrementalUniConstraint<S, A, E, F, W, Sc>`** — Single-collection constraint with filter and weight.

**`IncrementalBiConstraint<S, A, K, E, KE, F, W, Sc>`** — Self-join bi constraint (pairs from same collection). Joined filters receive the two source entity indexes.

**`IncrementalTriConstraint<S, A, K, E, KE, F, W, Sc>`** — Self-join tri constraint (triples). Joined filters receive the three source entity indexes.

**`IncrementalQuadConstraint<S, A, K, E, KE, F, W, Sc>`** — Self-join quad constraint. Joined filters receive the four source entity indexes.

**`IncrementalPentaConstraint<S, A, K, E, KE, F, W, Sc>`** — Self-join penta constraint. Joined filters receive the five source entity indexes.

**`constraint::cross_bi_incremental::Bi<S, A, B, K, EA, EB, KA, KB, F, W, Sc>`** — Cross-collection bi constraint (two different collections joined by key). Stateless `evaluate()`, `match_count()`, and `get_matches()` rebuild the keyed B-side index directly, so retained analysis works even before `initialize()`. Filters receive the A and B source indexes on every direct, grouped, and projected finalization path. The low-level `new(...)` constructor preserves index-aware weights via `Fn(&S, usize, usize) -> Sc`; fluent stream builders use `PairWeight<W>` internally for `Fn(&A, &B) -> Sc` weights without cloning streams or extractors.

**`CrossBiWeight<S, A, B, Sc>`**, **`IndexWeight<W>`**, **`PairWeight<W>`** — Zero-erasure cross-bi weight strategies. They keep low-level index-aware scoring and fluent pair-aware scoring as separate monomorphized paths.

**`constraint::grouped::Uni<S, A, K, E, Fi, KF, C, V, R, Acc, W, Sc>`** where `C: Collector<&A>` — Group-by with collector and weight on `(&K, &R)`.

The grouped engine is split into `GroupedNodeState` plus
`GroupedTerminalScorer` collections. `SharedGroupedConstraintSet` updates the
node once and refreshes all terminal scorers from changed group keys.
`constraint::grouped::Uni` is the one-terminal wrapper around that shared engine.
Additional grouped terminals append through the same fluent
`.penalize(...).named(...)` / `.reward(...).named(...)` finalization chain,
so macro-generated sharing does not use a separate stream construction path.

**`constraint::cross_grouped::Grouped<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc, W, Sc>`** where `C: Collector<(&A, &B)>` — Direct grouped cross-join constraint. It keeps keyed join indexes and collector retraction tokens without projecting joined pairs first.

Direct cross grouped constraints use `constraint::cross_grouped::GroupedNodeState` for join indexes,
match rows, group accumulators, retraction tokens, and changed-key reporting.
`constraint::cross_grouped::SharedGroupedSet` lets multiple terminal scorers consume that
state while preserving independent terminal metadata.

**`constraint::cross_complemented_grouped::ComplementedGrouped<S, A, B, T, JK, GK, EA, EB, ET, KA, KB, F, GF, KT, C, V, R, Acc, D, W, Sc>`** where `C: Collector<(&A, &B)>` — Direct grouped cross-join constraint complemented against a second collection. `constraint::cross_complemented_grouped::ComplementedGroupedNodeState` keeps join indexes, joined-pair collector retraction tokens, complement target indexes, group accumulators, and changed-key reporting; `constraint::cross_complemented_grouped::SharedComplementedGroupedSet` lets multiple terminal scorers consume each complement row using either the retained grouped result or the provided default result.

**`BalanceConstraint<S, A, K, E, F, KF, Sc>`** — Load balancing using sum-of-squared-deviations.

**`constraint::complemented::Grouped<S, A, B, K, EA, EB, KA, KB, C, V, R, Acc, D, W, Sc>`** where `C: Collector<&A>` — Group-by complemented against a second collection (for supply vs demand).

**`constraint::projected::Uni<S, Out, Src, F, W, Sc>`** — Terminal constraint for scoring retained projected rows one row at a time.

**`constraint::projected::Bi<S, Out, K, Src, F, KF, PF, W, Sc>`** — Symmetric self-join constraint over retained projected rows. Pair ordering is coordinate-stable by `RowCoordinate`; pair-filter indexes are the projected rows' primary owner entity indexes, never retained storage row IDs. It is produced by `stream::projected::Stream::join(equal(...))`.

**`constraint::projected::DirectedBi<S, Out, K, Src, F, KL, KR, PF, W, Sc>`** — Directed self-join constraint over retained projected rows with distinct left and right key extractors. It is produced by `stream::projected::Stream::join(equal_bi(left_key, right_key))`, evaluates oriented pairs where the left key equals the right key, and skips only the same retained row.

**`constraint::projected::Grouped<S, Out, K, Src, F, KF, C, V, R, Acc, W, Sc>`** where `C: Collector<&Out>` — Grouped retained projected rows.

Projected grouped constraints use `constraint::projected::GroupedNodeState` for projected
source state, row outputs, owner indexes, retraction tokens, group accumulators,
and changed-key reporting. `constraint::projected::SharedGroupedSet` shares that
projected grouped state across multiple terminal scorers.

**`constraint::projected::ComplementedGrouped<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D, W, Sc>`** where `C: Collector<&Out>` — Projected grouped rows complemented against a second collection, including `join(...).project(...).group_by(...).complement(...)` chains. `constraint::projected::ComplementedGroupedNodeState` retains projected source state, row ownership indexes, complement target rows, group accumulators, and changed-key reporting for shared terminal scorers.

**`FlattenedBiConstraint<S, A, B, C, K, CK, EA, EB, KA, KB, Flatten, CKeyFn, ALookup, F, W, Sc>`** — Cross-collection with nested collection flattening. Filters receive the A source index and the owning B source index for each flattened C row.

**`IncrementalExistsConstraint<S, A, P, B, K, EA, EP, KA, KB, FA, FP, Flatten, W, Sc>`** — Existence/non-existence check over a source-aware direct or flattened collection source. The constraint owns one scoring algorithm and delegates only key bookkeeping to an internal `ExistsKeyState`: exact `usize` keys use indexed `Vec` storage, while all other key types use hashed storage.

**`ExistenceMode`** — `enum { Exists, NotExists }`

### Analysis Types

Constraints own their `ConstraintRef` once. Metadata and analysis types borrow that identity so package-qualified constraint names remain intact without cloning `ConstraintRef` in scoring or reporting paths.

**`ConstraintResult<'a, Sc>`** — `{ name: &'a str, score: Sc, match_count: usize, is_hard: bool }`

**`ConstraintMetadata<'a>`** — `{ constraint_ref: &'a ConstraintRef, is_hard: bool }`; `name()` returns the short constraint name, and `full_name()` returns the package-qualified identity used for exact matching.

**`EntityRef`** — Public fields `{ type_name: String, display: String }` plus a private cloned entity snapshot for typed downcast access.
- Methods: `new()`, `with_display()`, `as_entity::<T>()`, `short_type_name()`
- Implements `Hash + Eq` (by display string)

**`ConstraintJustification`** — `{ entities: Vec<EntityRef>, description: String }`

**`DetailedConstraintMatch<'a, Sc: Score>`** — `{ constraint_ref: &'a ConstraintRef, score: Sc, justification: ConstraintJustification }`

**`DetailedConstraintEvaluation<'a, Sc: Score>`** — `{ total_score: Sc, match_count: usize, matches: Vec<DetailedConstraintMatch<'a, Sc>> }`

**`ConstraintAnalysis<'a, Sc: Score>`** — `{ constraint_ref: &'a ConstraintRef, weight: Sc, score: Sc, matches: Vec<DetailedConstraintMatch<'a, Sc>>, is_hard: bool }`

**`ScoreExplanation<'a, Sc: Score>`** — `{ score: Sc, constraint_analyses: Vec<ConstraintAnalysis<'a, Sc>> }`
- Methods: `total_match_count()`, `non_zero_constraints()`, `all_matches()`

**`Indictment<'a, Sc: Score>`** — `{ entity: EntityRef, score: Sc, constraint_matches: HashMap<&'a ConstraintRef, Vec<DetailedConstraintMatch<'a, Sc>>> }`
- Methods: `add_match()`, `match_count()`, `violated_constraints()`, `constraint_count()`

**`IndictmentMap<'a, Sc: Score>`** — `{ indictments: HashMap<EntityRef, Indictment<'a, Sc>> }`
- Methods: `from_matches()`, `get()`, `entities()`, `worst_entities()`, `len()`, `is_empty()`

**`ConstraintWeightOverrides<Sc: Score>`** — Runtime map of constraint-name weight overrides.
- Methods: `new()`, `from_pairs()`, `put()`, `remove()`, `get_or_default()`, `get()`, `contains()`, `len()`, `is_empty()`, `clear()`, `into_arc()`

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
- `join()` dispatch: `equal(|a| key)` → self-join `BiConstraintStream`; `(extractor_b, equal_bi(ka, kb))` → keyed `stream::cross::Bi`; `(other_stream, |a, b| pred)` → predicate `stream::cross::Bi`
- `into_parts()` → `(E, F)`, `from_parts(extractor, filter)` → `Self`, `extractor()` → `&E`

**`UniConstraintBuilder<S, A, E, F, W, Sc>`** — `named()` → `IncrementalUniConstraint`

**`Projection<A>`** — Retained projection contract for single-source `.project(...)`. Implementations define `type Out`, `const MAX_EMITS: usize`, and `project(&self, input: &A, sink: &mut impl ProjectionSink<Self::Out>)`. Projection implementations emit bounded scoring rows into the sink; Vec-returning closures are not part of the API. `Out` does not need `Clone`.

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
- Operations: `filter()`, `join()` → TriStream, `penalize(weight_or_fn)`, `reward(weight_or_fn)`

**`BiConstraintBuilder<S, A, K, E, KE, F, W, Sc>`** — `named()` → `IncrementalBiConstraint`

**`TriConstraintStream/Builder`** — Same pattern, tri-arity. `join()` → QuadStream.

**`QuadConstraintStream/Builder`** — Same pattern, quad-arity. `join()` → PentaStream.

**`PentaConstraintStream/Builder`** — Same pattern, penta-arity. Terminal (no further joins).

**`stream::cross::Bi<S, A, B, K, EA, EB, KA, KB, F, Sc>`** — Cross-collection bi stream.
- Operations: `filter()`, `group_by(|left, right| key, collector)` → stream::cross::Grouped, `project(|left, right| row)` → stream::projected::Stream, `penalize(weight_or_fn)`, `reward(weight_or_fn)`, `flatten_last()` → FlattenedBiStream

**`stream::cross::Builder`** — `named()` → `constraint::cross_bi_incremental::Bi`

**`stream::cross::Grouped/Builder`** — Direct grouped cross-join stream. `penalize(weight_or_fn)`, `reward(weight_or_fn)`, `named()` → `constraint::cross_grouped::Grouped`. `complement(source, key, default)` → `stream::cross::ComplementedGrouped`. Collectors receive the joined pair shape as `(&A, &B)`.

**`stream::cross::ComplementedGrouped/Builder`** — Direct grouped cross-join complement stream. `penalize(weight_or_fn)`, `reward(weight_or_fn)`, `named()` → `constraint::cross_complemented_grouped::ComplementedGrouped`. Complement defaults are produced from the complement entity and weighted by key plus collector result. Complement sources use the same `CollectionExtract::contains(...)` membership contract as joined sources.

**`GroupedConstraintStream<S, A, K, E, Fi, KF, C, V, R, Acc, Sc>`** — Grouped stream.
- Operations: `penalize(weight_or_fn)`, `reward(weight_or_fn)`, `complement()`, `complement_with_key()` → ComplementedStream
- Dynamic weighted operations use one canonical key-aware closure shape: `Fn(&K, &R) -> Sc`.

**`GroupedConstraintBuilder<S, A, K, E, Fi, KF, C, V, R, Acc, W, Sc>`** — `named()` → `constraint::grouped::Uni`

**`BalanceConstraintStream/Builder`** — Balance stream. `penalize(weight)`, `reward(weight)`, `named()` → `BalanceConstraint`

**`ComplementedConstraintStream/Builder`** — Complemented stream. `penalize(weight_or_fn)`, `reward(weight_or_fn)`, `named()` → `constraint::complemented::Grouped`. Dynamic weighted operations receive the real or complemented key as `Fn(&K, &R) -> Sc`.

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

**`CollectionExtract<S>`** — Public low-level source contract accepted by `ConstraintFactory::for_each(...)`. Macro-generated solution source functions return the concrete hidden `SourceExtract<fn(&S) -> &[T]>` wrapper, which satisfies `CollectionExtract<S>` and preserves source metadata for raw keyed joins.

**`ChangeSource`** — Hidden enum describing whether a stream source can localize descriptor-owned incremental callbacks: `Unknown`, `Static`, or `Descriptor(idx)`. `Descriptor(idx)` owns localized events for that descriptor. `Static` never localizes. `Unknown` is non-localized metadata for raw/manual extraction: it is valid for `evaluate()` and `initialize()`, but localized `on_insert(...)` / `on_retract(...)` callbacks panic because the entity index cannot be safely mapped to a source.

**`SourceExtract<E>` / `source(...)`** — Hidden descriptor-aware collection extraction used by macro-generated solution source methods. Planning entity collections carry `ChangeSource::Descriptor(idx)`; static fact and list-element collections carry `ChangeSource::Static`. These symbols are not part of the facade stream workflow.

**`FlattenExtract<P>`** — Trait for flattening a parent entity into a child slice for existence filtering. Blanket impl for `Fn(&P) -> &[B] + Send + Sync`; `FlattenVecExtract<F>` adapts `Fn(&P) -> &Vec<B>`.

**`ExistenceTarget<S, A, E, F, Sc>`** — Trait for `.if_exists(...)` / `.if_not_exists(...)` dispatch on `UniConstraintStream`.
- Direct target: `(other_stream, equal_bi(left_key, right_key))`
- Flattened target: `(parent_stream, flatten, equal_bi(left_key, flattened_key))`

**`FlattenedCollectionTarget<S, P, B, EP, FP, Flatten, Sc>`** — Intermediate existence target produced by `UniConstraintStream::flattened(flatten)` for nested collection membership checks.

### Join Support Types

**`JoinTarget<S, A, E, F, Sc>`** — Trait for `.join()` dispatch on `UniConstraintStream`.
- Impl groups: `EqualJoiner<KA, KA, K, Symmetric>` (self-join from `equal(...)`), any `CollectionExtract` target with `EqualJoiner<KA, KB, K, Mode>` (keyed cross-join from `equal_bi(...)`, including filtered `UniConstraintStream` targets), and `(UniConstraintStream<...>, P)` (predicate cross-join with filtered stream target).

**`ProjectedJoinTarget<S, Out, Src, F, Sc>`** — Trait for `.join()` dispatch on `stream::projected::Stream`.
- `equal(|row| key)` dispatches to `stream::projected::Bi` and preserves symmetric coordinate-stable pair ordering.
- `equal_bi(left_key, right_key)` dispatches to `stream::projected::DirectedBi` and preserves left/right key orientation for projected rows of the same output type.

**`KeyExtract<S, A, K>`** — Trait for key extraction. Blanket impl for `Fn(&S, &A, usize) -> K + Send + Sync`. Used as the bound on `KE` type params in nary stream/constraint macros.
- Method: `fn extract(&self, s: &S, a: &A, idx: usize) -> K`

**`EntityKeyAdapter<KA>`** — Wraps `KA: Fn(&A) -> K` as a `KeyExtract`. Used in self-join `JoinTarget` impl to adapt entity-only key functions.
- `new(key_fn: KA)` → `EntityKeyAdapter<KA>`

### Filter Types

**`TrueFilter`** — Always-true filter. Implements all filter traits.

**`FnUniFilter<F>`**, **`FnBiFilter<F>`**, **`FnTriFilter<F>`**, **`FnQuadFilter<F>`**, **`FnPentaFilter<F>`** — Closure wrappers. Joined-arity wrappers are index-aware and receive the same index arguments as their filter traits.

**`AndUniFilter<F1,F2>`**, **`AndBiFilter<F1,F2>`**, **`AndTriFilter<F1,F2>`**, **`AndQuadFilter<F1,F2>`**, **`AndPentaFilter<F1,F2>`** — Conjunctive composition.

**`UniBiFilter<F, A>`** — Adapts UniFilter to BiFilter (tests both args same predicate).

**`UniLeftBiFilter<F, B>`** — Adapts UniFilter to BiFilter (tests left arg only).

**`PairFilter<L, R, P>`** — Hidden internal adapter that composes the left stream filter, right stream filter, and user pair predicate for predicate joins.

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
- `CollectedVec<T>` exposes `iter()`, `len()`, `is_empty()`, and `to_vec()` when `T: Clone`.

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

### N-ary Constraint Macros

`IncrementalBiConstraint`, `IncrementalTriConstraint`, `IncrementalQuadConstraint`, `IncrementalPentaConstraint` are all generated by declarative macros (`impl_incremental_bi_constraint!`, etc.). They share the same structure:
- `entity_to_matches: HashMap<usize, HashSet<(usize, ...)>>` — per-entity match tracking
- `matches: HashSet<(usize, ...)>` — all current matches
- `key_to_indices: HashMap<K, HashSet<usize>>` — key-based index for join
- `index_to_key: HashMap<usize, K>` — reverse key lookup

### Stream Arity Macros

`impl_bi_arity_stream!`, `impl_tri_arity_stream!`, `impl_quad_arity_stream!`, `impl_penta_arity_stream!` generate the stream and builder structs for each arity level. All four macros live under `arity_stream_macros/nary_stream/`. They share the same field layout and method pattern but differ in the number of entity arguments to filter/weight functions.

### PhantomData Pattern

All types use `PhantomData<(fn() -> S, fn() -> A, ...)>` to avoid inheriting bounds from phantom type parameters.

## Cross-Crate Dependencies

- **From `solverforge-core`:** `Score`, `PlanningSolution`, `ConstraintRef`, `ImpactType`, `SolutionDescriptor`, `EntityDescriptor`, `ProblemFactDescriptor`, `VariableDescriptor`, `EntityCollectionExtractor`
