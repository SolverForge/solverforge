# solverforge-scoring WIREFRAME

Zero-erasure incremental constraint scoring infrastructure for SolverForge.

**Location:** `crates/solverforge-scoring/`
**Workspace Release:** `0.11.0`

## Dependencies

- `solverforge-core` (path) — Score types, domain traits, descriptors, ConstraintRef, ImpactType
- `thiserror` (workspace) — error derive macros
- `solverforge-test` (dev, workspace) — test utilities

## File Map

```
src/
├── lib.rs                                          — Crate root; re-exports from all modules
├── api/
│   ├── mod.rs                                      — Re-exports analysis, constraint_set, weight_overrides
│   ├── analysis.rs                                 — ScoreExplanation, ConstraintAnalysis, Indictment, IndictmentMap, DetailedConstraintMatch, etc.
│   ├── constraint_set/
│   │   ├── mod.rs                                  — Re-exports ConstraintSet, IncrementalConstraint, ConstraintMetadata, ConstraintResult
│   │   ├── incremental.rs                          — IncrementalConstraint trait, ConstraintSet trait, tuple impls (0..32)
│   │   └── tests/
│   │       ├── mod.rs                              — Test module declarations
│   │       └── constraint_set.rs                   — ConstraintSet tuple tests
│   ├── weight_overrides.rs                         — ConstraintWeightOverrides, WeightProvider trait
│   └── tests/
│       ├── mod.rs                                  — Test module declarations
│       ├── analysis.rs                             — Analysis type tests
│       └── weight_overrides.rs                     — Weight override tests
├── constraint/
│   ├── mod.rs                                      — Re-exports all constraint types
│   ├── macros.rs                                   — impl_get_matches_nary! macro for detailed match generation
│   ├── shared.rs                                   — compute_hash<T>() utility function
│   ├── incremental.rs                              — IncrementalUniConstraint<S,A,E,F,W,Sc>
│   ├── grouped.rs                                  — GroupedUniConstraint<S,A,K,E,Fi,KF,C,W,Sc>
│   ├── balance.rs                                  — BalanceConstraint<S,A,K,E,F,KF,Sc>
│   ├── complemented.rs                             — ComplementedGroupConstraint module root and re-exports
│   ├── complemented/*.rs                           — Retained complemented state, incremental callbacks, helpers, and debug accessors
│   ├── cross_bi_incremental.rs                     — IncrementalCrossBiConstraint module root and re-exports
│   ├── cross_bi_incremental/*.rs                   — Retained cross-bi state, weights, incremental callbacks, and debug accessors
│   ├── flattened_bi.rs                             — FlattenedBiConstraint module root and re-exports
│   ├── flattened_bi/*.rs                           — Retained flattened-bi state, incremental callbacks, and debug accessors
│   ├── exists.rs                                   — IncrementalExistsConstraint<S,A,P,B,K,EA,EP,KA,KB,FA,FP,Flatten,W,Sc>, SelfFlatten
│   ├── exists/
│   │   └── key_state.rs                            — Internal hashed/indexed key bookkeeping for existence constraints
│   ├── projected.rs                                — Projected retained scoring-row constraint module root and re-exports
│   ├── projected/*.rs                              — Projected uni, bi, grouped constraints and shared source state
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
│       ├── tri_incr.rs                             — IncrementalTriConstraint tests
│       ├── quad_incr.rs                            — IncrementalQuadConstraint tests
│       ├── penta_incr.rs                           — IncrementalPentaConstraint tests
│       ├── grouped.rs                              — GroupedUniConstraint tests
│       ├── balance.rs                              — BalanceConstraint tests
│       ├── complemented.rs                         — ComplementedGroupConstraint tests
│       ├── flattened_bi.rs                         — FlattenedBiConstraint tests
│       ├── exists.rs                               — IncrementalExistsConstraint update tests
│       ├── exists_storage.rs                       — Existence storage selection and parity tests
│       ├── projected.rs                            — Projected constraint test module root
│       ├── projected/*.rs                          — Projected support fixtures, localization, update, grouping, merge, and self-join tests
│       └── repro_unknown.rs                        — Regression fixture coverage for unknown-source behavior
├── director/
│   ├── mod.rs                                      — Re-exports all director types and traits
│   ├── traits.rs                                   — Director<S> trait
│   ├── score_director.rs                           — Re-exports typed ScoreDirector pieces
│   │   ├── score_director/typed.rs                 — ScoreDirector<S,C> (zero-erasure incremental)
│   │   └── score_director/adapters.rs              — Debug and Director trait impls for ScoreDirector
│   ├── recording.rs                                — RecordingDirector<'a,S,D> (automatic undo tracking)
│   ├── shadow_aware.rs                             — SolvableSolution trait and shadow lifecycle notes
│   └── tests/
│       ├── mod.rs                                  — Test module declarations
│       ├── bench.rs                                — Benchmark test module declarations
│       ├── benchmarks.rs                           — Performance comparison tests
│       ├── score_director.rs                       — ScoreDirector tests
│       ├── recording.rs                            — RecordingDirector tests
│       └── shadow.rs                               — Shadow-aware director tests

├── stream/
│   ├── mod.rs                                      — Module declarations and re-exports for all stream types
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
│   ├── cross_bi_stream/base.rs                     — CrossBiConstraintStream
│   ├── cross_bi_stream/weighting.rs                — CrossBiConstraintBuilder
│   ├── flattened_bi_stream.rs                      — Re-exports
│   ├── flattened_bi_stream/base.rs                 — FlattenedBiConstraintStream
│   ├── flattened_bi_stream/builder.rs              — FlattenedBiConstraintBuilder
│   ├── flattened_bi_stream/weighting.rs            — Weighting helpers for flattened streams
│   ├── existence_stream.rs                         — ExistsConstraintStream, ExistsConstraintBuilder, ExistenceMode, FlattenExtract
│   ├── existence_target.rs                         — ExistenceTarget trait for direct and flattened existence targets
│   ├── projected_stream.rs                         — Projected stream module root and re-exports
│   ├── projected_stream/uni.rs                     — ProjectedConstraintStream and terminal builder
│   ├── projected_stream/bi.rs                      — ProjectedBiConstraintStream and terminal builder
│   ├── projected_stream/grouped.rs                 — ProjectedGroupedConstraintStream and terminal builder
│   ├── projected_stream/source.rs                  — Projection, projected row coordinates, ProjectedSource trait
│   ├── projected_stream/source/single.rs           — Single-source `.project(...)` source
│   ├── projected_stream/source/filtered.rs         — Row-level filtered projected source
│   ├── projected_stream/source/merged.rs           — Merged projected sources with source-slot offsets
│   ├── projected_stream/source/joined.rs           — Cross-join `.project(...)` projected source
│   ├── collection_extract.rs                       — CollectionExtract trait, source-aware extractors, VecExtract wrapper, vec() constructor
│   ├── join_target.rs                              — JoinTarget trait + 3 impls (self-join, keyed cross-join, predicate cross-join)
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
│   │   ├── adapters.rs                             — UniBiFilter, UniLeftBiFilter, UniLeftPredBiFilter adapters
│   │   ├── composition.rs                          — AndUniFilter, AndBiFilter, AndTriFilter, AndQuadFilter, AndPentaFilter
│   │   └── tests/
│   │       ├── mod.rs                              — Test module declarations
│   │       └── filter.rs                           — Filter tests
│   ├── joiner/
│   │   ├── mod.rs                                  — Re-exports all joiner types and functions
│   │   ├── equal.rs                                — EqualJoiner, equal(), equal_bi() functions
│   │   ├── comparison.rs                           — LessThan/GreaterThan joiners and factory functions
│   │   ├── filtering.rs                            — FilteringJoiner and filtering() function
│   │   ├── overlapping.rs                          — OverlappingJoiner and overlapping() function
│   │   └── match_condition.rs                      — Joiner trait, AndJoiner, FnJoiner
│   └── collector/
│       ├── mod.rs                                  — Re-exports collector types
│       ├── uni.rs                                  — UniCollector trait, Accumulator trait
│       ├── count.rs                                — CountCollector, CountAccumulator, count()
│       ├── sum.rs                                  — SumCollector, SumAccumulator, sum()
│       ├── load_balance.rs                         — LoadBalanceCollector, LoadBalanceAccumulator, LoadBalance, load_balance()
│       └── tests/
│           ├── mod.rs                              — Test module declarations
│           └── collector.rs                        — Collector tests
```

## Public Re-exports (lib.rs)

```rust
// Constraints
pub use constraint::{
    GroupedUniConstraint, IncrementalBiConstraint, IncrementalCrossBiConstraint,
    IncrementalPentaConstraint, IncrementalQuadConstraint, IncrementalTriConstraint,
    IncrementalUniConstraint, ProjectedGroupedConstraint, ProjectedUniConstraint,
};

// Constraint Set
pub use api::constraint_set::{
    ConstraintMetadata, ConstraintResult, ConstraintSet, IncrementalConstraint,
};
pub use api::weight_overrides::{ConstraintWeightOverrides, WeightProvider};

// Score Directors
pub use director::score_director::ScoreDirector;
pub use director::{
    RecordingDirector, Director, DirectorScoreState, SolvableSolution,
};

// Analysis
pub use api::analysis::{
    ConstraintAnalysis, ConstraintJustification, DetailedConstraintEvaluation,
    DetailedConstraintMatch, EntityRef, Indictment, IndictmentMap, ScoreExplanation,
};

// Fluent Stream API
pub use stream::{
    BiConstraintBuilder, BiConstraintStream, ConstraintFactory, GroupedConstraintBuilder,
    GroupedConstraintStream, ProjectedBiConstraintBuilder, ProjectedBiConstraintStream,
    ProjectedConstraintBuilder, ProjectedConstraintStream, ProjectedGroupedConstraintBuilder,
    ProjectedGroupedConstraintStream, Projection, ProjectionSink, UniConstraintBuilder,
    UniConstraintStream,
};
```

## Public Traits

### `Director<S: PlanningSolution>` — `Send`

| Method | Signature | Note |
|--------|-----------|------|
| `working_solution` | `fn working_solution(&self) -> &S` | Current solution reference |
| `working_solution_mut` | `fn working_solution_mut(&mut self) -> &mut S` | Mutable solution reference |
| `calculate_score` | `fn calculate_score(&mut self) -> S::Score` | Full score calculation |
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
| `register_undo` | `fn register_undo(&mut self, _undo: Box<dyn FnOnce(&mut S) + Send>)` | Default: no-op |

### `DirectorScoreState<Sc>`

Committed score-state snapshot used to roll back speculative evaluation. Fields:
`solution_score`, `committed_score`, `initialized`.

### `IncrementalConstraint<S, Sc: Score>` — `Send + Sync`

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
| `constraint_metadata` | `fn constraint_metadata(&self) -> Vec<ConstraintMetadata<'_>>` | Borrowed ref/name/hardness metadata |
| `evaluate_each` | `fn evaluate_each<'a>(&'a self, solution: &S) -> Vec<ConstraintResult<'a, Sc>>` | Per-constraint results |
| `evaluate_detailed` | `fn evaluate_detailed<'a>(&'a self, solution: &S) -> Vec<ConstraintAnalysis<'a, Sc>>` | With match details |
| `initialize_all` | `fn initialize_all(&mut self, solution: &S) -> Sc` | Initialize all for incremental |
| `on_insert_all` | `fn on_insert_all(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc` | Incremental insert all |
| `on_retract_all` | `fn on_retract_all(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc` | Incremental retract all |
| `reset_all` | `fn reset_all(&mut self)` | Reset all |

Implemented for tuples `()` through `(C0, C1, ..., C31)` where each `Ci: IncrementalConstraint<S, Sc>`. Tuple metadata deduplicates repeated full `ConstraintRef`s when hardness agrees and panics when the same full ref has conflicting hard/non-hard metadata. Package-qualified constraints that share a short name remain distinct.

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
- `TriFilter<S, A, B, C>` — `fn test(&self, solution: &S, a: &A, b: &B, c: &C) -> bool`
- `QuadFilter<S, A, B, C, D>` — `fn test(&self, solution: &S, a: &A, b: &B, c: &C, d: &D) -> bool`
- `PentaFilter<S, A, B, C, D, E>` — `fn test(&self, solution: &S, a: &A, b: &B, c: &C, d: &D, e: &E) -> bool`

### `Joiner<A, B>` — `Send + Sync`

| Method | Signature | Note |
|--------|-----------|------|
| `matches` | `fn matches(&self, a: &A, b: &B) -> bool` | Test if pair matches |
| `and` | `fn and<J>(self, other: J) -> AndJoiner<Self, J>` | Compose joiners |

### Collector Traits

**`UniCollector<A>` — `Send + Sync`**

| Associated Type | Bound | Note |
|-----------------|-------|------|
| `Value` | — | Extracted value type |
| `Result` | `Clone + Send + Sync` | Finalized result type |
| `Accumulator` | `Accumulator<Self::Value, Self::Result>` | Stateful accumulator |

| Method | Signature | Note |
|--------|-----------|------|
| `extract` | `fn extract(&self, entity: &A) -> Self::Value` | Extract value from entity |
| `create_accumulator` | `fn create_accumulator(&self) -> Self::Accumulator` | Create fresh accumulator |

**`Accumulator<V, R>` — `Send + Sync`**

| Method | Signature | Note |
|--------|-----------|------|
| `accumulate` | `fn accumulate(&mut self, value: &V)` | Add value |
| `retract` | `fn retract(&mut self, value: &V)` | Remove value |
| `finish` | `fn finish(&self) -> R` | Produce result |
| `reset` | `fn reset(&mut self)` | Clear state |

## Public Structs

### Score Directors

**`ScoreDirector<S, C>`** where `S: PlanningSolution`, `C: ConstraintSet<S, S::Score>`
- Primary incremental scoring director. Zero-erasure.
- Key methods: `new()`, `with_descriptor()`, `simple()` (convenience for `ScoreDirector<S, ()>`), `simple_zero()` (test helper with empty descriptor), `calculate_score()`, `before_variable_changed()`, `after_variable_changed()`, `do_change()`, `get_score()`, `constraint_metadata()`, `constraint_match_totals()`, `into_working_solution()`, `take_solution()`
- Builds immutable constraint metadata once from the typed `ConstraintSet`.
- `simple(solution, descriptor, entity_counter)` — creates `ScoreDirector<S, ()>` with empty constraint set
- `simple_zero(solution)` — creates `ScoreDirector<S, ()>` with empty descriptor and zero entity counter
- Implements `Director<S>`

**`RecordingDirector<'a, S, D>`** where `S: PlanningSolution`, `D: Director<S>`
- Wraps any director with automatic undo tracking.
- Restores the wrapped director's committed score state after speculative
  evaluation and undo.
- Key methods: `new()`, `undo_changes()`, `reset()`, `change_count()`, `is_empty()`
- Implements `Director<S>`

### Constraint Types

All implement `IncrementalConstraint<S, Sc>`.

**`IncrementalUniConstraint<S, A, E, F, W, Sc>`** — Single-collection constraint with filter and weight.

**`IncrementalBiConstraint<S, A, K, E, KE, F, W, Sc>`** — Self-join bi constraint (pairs from same collection).

**`IncrementalTriConstraint<S, A, K, E, KE, F, W, Sc>`** — Self-join tri constraint (triples).

**`IncrementalQuadConstraint<S, A, K, E, KE, F, W, Sc>`** — Self-join quad constraint.

**`IncrementalPentaConstraint<S, A, K, E, KE, F, W, Sc>`** — Self-join penta constraint.

**`IncrementalCrossBiConstraint<S, A, B, K, EA, EB, KA, KB, F, W, Sc>`** — Cross-collection bi constraint (two different collections joined by key). Stateless `evaluate()`, `match_count()`, and `get_matches()` rebuild the keyed B-side index directly, so retained analysis works even before `initialize()`. The low-level `new(...)` constructor preserves index-aware weights via `Fn(&S, usize, usize) -> Sc`; fluent stream builders use `PairWeight<W>` internally for `Fn(&A, &B) -> Sc` weights without cloning streams or extractors.

**`CrossBiWeight<S, A, B, Sc>`**, **`IndexWeight<W>`**, **`PairWeight<W>`** — Zero-erasure cross-bi weight strategies. They keep low-level index-aware scoring and fluent pair-aware scoring as separate monomorphized paths.

**`GroupedUniConstraint<S, A, K, E, Fi, KF, C, W, Sc>`** where `C: UniCollector<A>` — Group-by with collector and weight on result.

**`BalanceConstraint<S, A, K, E, F, KF, Sc>`** — Load balancing using sum-of-squared-deviations.

**`ComplementedGroupConstraint<S, A, B, K, EA, EB, KA, KB, C, D, W, Sc>`** where `C: UniCollector<A>` — Group-by complemented against a second collection (for supply vs demand).

**`FlattenedBiConstraint<S, A, B, C, K, CK, EA, EB, KA, KB, Flatten, CKeyFn, ALookup, F, W, Sc>`** — Cross-collection with nested collection flattening.

**`IncrementalExistsConstraint<S, A, P, B, K, EA, EP, KA, KB, FA, FP, Flatten, W, Sc>`** — Existence/non-existence check over a source-aware direct or flattened collection source. The constraint owns one scoring algorithm and delegates only key bookkeeping to an internal `ExistsKeyState`: exact `usize` keys use indexed `Vec` storage, while all other key types use hashed storage.

**`ExistenceMode`** — `enum { Exists, NotExists }`

### Analysis Types

Constraints own their `ConstraintRef` once. Metadata and analysis types borrow that identity so package-qualified constraint names remain intact without cloning `ConstraintRef` in scoring or reporting paths.

**`ConstraintResult<'a, Sc>`** — `{ name: &'a str, score: Sc, match_count: usize, is_hard: bool }`

**`ConstraintMetadata<'a>`** — `{ constraint_ref: &'a ConstraintRef, is_hard: bool }`; `name()` returns the short constraint name, and `full_name()` returns the package-qualified identity used for exact matching.

**`EntityRef`** — `{ type_name: String, display: String, entity: Arc<dyn Any + Send + Sync> }`
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

**`ConstraintWeightOverrides<Sc: Score>`** — `{ weights: HashMap<String, Sc> }`
- Methods: `new()`, `from_pairs()`, `put()`, `remove()`, `get_or_default()`, `get()`, `contains()`, `len()`, `is_empty()`, `clear()`, `into_arc()`

### Stream Builders (Fluent API)

**`ConstraintFactory<S, Sc: Score>`** — Entry point.
- `new()`, `for_each()` → `UniConstraintStream`
- Generated domain accessors call the same `for_each()` with hidden descriptor/static source metadata.

**`UniConstraintStream<S, A, E, F, Sc>`** — Single collection stream.
- Operations: `filter()`, `join(target)` (single dispatch via `JoinTarget`), `group_by()`, `balance()`, `project(projection)` → `ProjectedConstraintStream`, `flattened(flatten)` → `FlattenedCollectionTarget`, `if_exists(target)`, `if_not_exists(target)`, `penalize()`, `penalize_with()`, `penalize_hard_with()`, `penalize_hard()`, `penalize_soft()`, `reward()`, `reward_with()`, `reward_hard_with()`, `reward_hard()`, `reward_soft()`
- Unfiltered `UniConstraintStream<..., TrueFilter, ...>` implements `CollectionExtract` by delegating to its source extractor. This lets keyed cross-join targets use generated/source-aware streams directly, preserving `ChangeSource` metadata.
- `join()` dispatch: `equal(|a| key)` → self-join `BiConstraintStream`; `(extractor_b, equal_bi(ka, kb))` → keyed `CrossBiConstraintStream`; `(other_stream, |a, b| pred)` → predicate `CrossBiConstraintStream`
- `into_parts()` → `(E, F)`, `from_parts(extractor, filter)` → `Self`, `extractor()` → `&E`

**`UniConstraintBuilder<S, A, E, F, W, Sc>`** — `named()` → `IncrementalUniConstraint`

**`Projection<A>`** — Typed retained projection contract for single-source `.project(...)`. Implementations define `type Out`, `const MAX_EMITS: usize`, and `project(&self, input: &A, sink: &mut impl ProjectionSink<Self::Out>)`. Projection implementations emit bounded scoring rows into the sink; Vec-returning closures are not part of the API. `Out` does not need `Clone`.

**`ProjectionSink<Out>`** — Emission sink used by `Projection<A>` implementations. `emit(output)` is the only projection output channel.

**`ProjectedConstraintStream<S, Out, Src, F, Sc>`** — Scoring rows from one or more source streams. Single-source output type is inferred from the named projection type passed to `project(...)`; keyed cross joins use `CrossBiConstraintStream::project(|left, right| row)` and emit exactly one scoring row per retained joined pair. Retained rows are cached by `ProjectedRowCoordinate` and indexed by one or two `ProjectedRowOwner` values. Single-source projected rows update incrementally from their source owner; joined-pair projected rows update incrementally from either joined source when that source is descriptor-localized. Projected self-join pair order follows `ProjectedRowCoordinate` ordering; retained storage row IDs are internal and never semantic. Projected rows can be self-joined by `equal(|row| key)` without materialized facts, and projected output rows plus projected self-join keys do not need `Clone`. Raw `for_each` extractors with `ChangeSource::Unknown` can evaluate and initialize projected constraints, but localized incremental callbacks panic because their entity indexes cannot be mapped safely.
- Operations: `filter()`, `merge(other)`, `group_by()`, `join(equal(...))`, `penalize_with()`, `penalize_hard_with()`

**`ProjectedRowCoordinate`** — Hidden support coordinate for projected rows:
`{ primary_owner, secondary_owner, emit_index }`. `primary_owner` is always
present; `secondary_owner` is present for joined-pair rows from
`CrossBiConstraintStream::project(...)`. It is used to keep projected self-join
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

factory.assignments().project(AssignmentLoadEntries)
```

**`ProjectedGroupedConstraintStream` / `ProjectedGroupedConstraintBuilder`** — Grouped projected rows using stock collectors such as `sum()` and `count()`. Grouped retained state uses the same `ProjectedRowOwner` ownership index as ungrouped projected rows. Collector values do not need `Clone`; retained grouped state stores the projected row once by `ProjectedRowCoordinate` and recomputes key/value on retract. `named()` → `ProjectedGroupedConstraint`.

**`BiConstraintStream<S, A, K, E, KE, F, Sc>`** — Self-join bi stream (macro-generated).
- Operations: `filter()`, `join()` → TriStream, `penalize()`, `penalize_with()`, `penalize_hard_with()`, `penalize_hard()`, `penalize_soft()`, `reward()`, `reward_with()`, `reward_hard_with()`, `reward_hard()`, `reward_soft()`

**`BiConstraintBuilder<S, A, K, E, KE, F, W, Sc>`** — `named()` → `IncrementalBiConstraint`

**`TriConstraintStream/Builder`** — Same pattern, tri-arity. `join()` → QuadStream. Same convenience methods.

**`QuadConstraintStream/Builder`** — Same pattern, quad-arity. `join()` → PentaStream. Same convenience methods.

**`PentaConstraintStream/Builder`** — Same pattern, penta-arity. Terminal (no further joins). Same convenience methods.

**`CrossBiConstraintStream<S, A, B, K, EA, EB, KA, KB, F, Sc>`** — Cross-collection bi stream.
- Operations: `filter()`, `project(|left, right| row)` → ProjectedConstraintStream, `penalize()`, `penalize_with()`, `penalize_hard_with()`, `penalize_hard()`, `penalize_soft()`, `reward()`, `reward_with()`, `reward_hard_with()`, `reward_hard()`, `reward_soft()`, `flatten_last()` → FlattenedBiStream

**`CrossBiConstraintBuilder`** — `named()` → `IncrementalCrossBiConstraint`

**`GroupedConstraintStream<S, A, K, E, Fi, KF, C, Sc>`** — Grouped stream.
- Operations: `penalize_with()`, `penalize_hard_with()`, `penalize_hard()`, `penalize_soft()`, `reward_with()`, `reward_hard_with()`, `reward_hard()`, `reward_soft()`, `complement()`, `complement_with_key()` → ComplementedStream

**`GroupedConstraintBuilder`** — `named()` → `GroupedUniConstraint`

**`BalanceConstraintStream/Builder`** — Balance stream. `penalize()`, `penalize_hard()`, `penalize_soft()`, `reward()`, `reward_hard()`, `reward_soft()`, `named()` → `BalanceConstraint`

**`ComplementedConstraintStream/Builder`** — Complemented stream. `penalize_with()`, `penalize_hard()`, `penalize_soft()`, `reward_with()`, `reward_hard()`, `reward_soft()`, `named()` → `ComplementedGroupConstraint`

**`FlattenedBiConstraintStream/Builder`** — Flattened bi stream. `filter()`, `penalize()`, `penalize_with()`, `penalize_hard()`, `penalize_soft()`, `reward()`, `reward_hard()`, `reward_soft()`, `named()` → `FlattenedBiConstraint`

**`ExistsConstraintStream/ExistsConstraintBuilder`** — Existence stream over source-aware direct or flattened collection targets. `penalize()`, `penalize_hard()`, `penalize_soft()`, `reward()`, `reward_hard()`, `reward_soft()`, `named()` → `IncrementalExistsConstraint`. There is no separate public indexed existence stream; storage selection is internal to `IncrementalExistsConstraint`.

### Extractor Types

**`CollectionExtract<S>`** — Trait for extracting an entity slice from the solution. All `E`/`EA`/`EB` type params in streams and constraints are bounded by `CollectionExtract<S, Item = A>` rather than raw `Fn(&S) -> &[A]`, allowing both closure forms.
- Associated type: `type Item` — the entity type yielded.
- Method: `fn extract<'s>(&self, s: &'s S) -> &'s [Self::Item]`
- Blanket impl for `Fn(&S) -> &[A] + Send + Sync` — plain slice closures `|s| s.field.as_slice()` work directly.

**`VecExtract<F>`** — Wraps `Fn(&S) -> &Vec<A>` closures so they satisfy `CollectionExtract<S>`. Construct via `vec(f)`.
- Users can write `|s| &s.field` without `.as_slice()`.

**`vec(f)`** — Free function: `fn vec<S, A, F>(f: F) -> VecExtract<F>`. Use when the extractor closure returns `&Vec<A>`:
```rust
factory.for_each(vec(|s: &Schedule| &s.employees))
// or in a join:
.join((vec(|s: &Schedule| &s.employees), equal_bi(...)))
```

**`ChangeSource`** — Enum describing whether a stream source can localize descriptor-owned incremental callbacks: `Unknown`, `Static`, or `Descriptor(idx)`. `Descriptor(idx)` owns localized events for that descriptor. `Static` never localizes. `Unknown` is non-localized metadata for raw/manual extraction: it is valid for `evaluate()` and `initialize()`, but localized `on_insert(...)` / `on_retract(...)` callbacks panic because the entity index cannot be safely mapped to a source.

**`SourceExtract<E>` / `source(...)`** — Descriptor-aware collection extraction used by generated accessors and structured source-aware streams. Planning entity collections carry `ChangeSource::Descriptor(idx)`; static fact collections carry `ChangeSource::Static`. Raw `for_each` closure extractors use `ChangeSource::Unknown`; wrap extractors with `source(..., ChangeSource::Descriptor(idx))` when they must participate in localized incremental mutation callbacks.

**`FlattenExtract<P>`** — Trait for flattening a parent entity into a child slice for existence filtering. Blanket impl for `Fn(&P) -> &[B] + Send + Sync`; `FlattenVecExtract<F>` adapts `Fn(&P) -> &Vec<B>`.

**`ExistenceTarget<S, A, E, F, Sc>`** — Trait for `.if_exists(...)` / `.if_not_exists(...)` dispatch on `UniConstraintStream`.
- Direct target: `(other_stream, equal_bi(left_key, right_key))`
- Flattened target: `(parent_stream, flatten, equal_bi(left_key, flattened_key))`

**`FlattenedCollectionTarget<S, P, B, EP, FP, Flatten, Sc>`** — Intermediate existence target produced by `UniConstraintStream::flattened(flatten)` for nested collection membership checks.

### Join Support Types

**`JoinTarget<S, A, E, F, Sc>`** — Trait for `.join()` dispatch on `UniConstraintStream`.
- Three impls: `EqualJoiner<KA, KA, K>` (self-join), `(EB, EqualJoiner<KA, KB, K>)` (keyed cross-join), `(UniConstraintStream<...>, P)` (predicate cross-join)

**`KeyExtract<S, A, K>`** — Trait for key extraction. Blanket impl for `Fn(&S, &A, usize) -> K + Send + Sync`. Used as the bound on `KE` type params in nary stream/constraint macros.
- Method: `fn extract(&self, s: &S, a: &A, idx: usize) -> K`

**`EntityKeyAdapter<KA>`** — Wraps `KA: Fn(&A) -> K` as a `KeyExtract`. Used in self-join `JoinTarget` impl to adapt entity-only key functions.
- `new(key_fn: KA)` → `EntityKeyAdapter<KA>`

### Filter Types

**`TrueFilter`** — Always-true filter. Implements all filter traits.

**`FnUniFilter<F>`**, **`FnBiFilter<F>`**, **`FnTriFilter<F>`**, **`FnQuadFilter<F>`**, **`FnPentaFilter<F>`** — Closure wrappers.

**`AndUniFilter<F1,F2>`**, **`AndBiFilter<F1,F2>`**, **`AndTriFilter<F1,F2>`**, **`AndQuadFilter<F1,F2>`**, **`AndPentaFilter<F1,F2>`** — Conjunctive composition.

**`UniBiFilter<F, A>`** — Adapts UniFilter to BiFilter (tests both args same predicate).

**`UniLeftBiFilter<F, B>`** — Adapts UniFilter to BiFilter (tests left arg only).

**`UniLeftPredBiFilter<F, P, A>`** — Combines UniFilter (left element) and predicate `Fn(&A, &B) -> bool`. Used in the predicate cross-join case to avoid `impl Trait` in associated type position.

### Joiner Types

**`EqualJoiner<Fa, Fb, T>`** — Join by key equality.
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

**`CountCollector<A>`** / **`CountAccumulator`** — Counts matching entities. Factory: `count()`

**`SumCollector<A, T, F>`** / **`SumAccumulator<T>`** — Sums mapped values. Factory: `sum(mapper)`

**`LoadBalanceCollector<A, K, F, M>`** / **`LoadBalanceAccumulator<K>`** / **`LoadBalance<K>`** — Load balance with unfairness metric.
- Factory: `load_balance(key_fn, metric_fn)`
- `LoadBalance<K>` has `loads()` and `unfairness()` methods.

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

`ConstraintSet` is implemented for tuples of up to 32 elements via a macro. Each tuple element must implement `IncrementalConstraint<S, Sc>`. Operations iterate over all tuple elements, summing scores. This is the zero-erasure alternative to `Vec<Box<dyn Constraint>>`.

### N-ary Constraint Macros

`IncrementalBiConstraint`, `IncrementalTriConstraint`, `IncrementalQuadConstraint`, `IncrementalPentaConstraint` are all generated by declarative macros (`impl_incremental_bi_constraint!`, etc.). They share the same structure:
- `entity_to_matches: HashMap<usize, HashSet<(usize, ...)>>` — per-entity match tracking
- `matches: HashSet<(usize, ...)>` — all current matches
- `key_to_indices: HashMap<K, HashSet<usize>>` — key-based index for join
- `index_to_key: HashMap<usize, K>` — reverse key lookup

### Stream Arity Macros

`impl_bi_arity_stream!`, `impl_tri_arity_stream!`, `impl_quad_arity_stream!`, `impl_penta_arity_stream!` generate the stream and builder structs for each arity level. All four macros are consolidated in `nary_stream.rs`. They share the same field layout and method pattern but differ in the number of entity arguments to filter/weight functions.

### PhantomData Pattern

All types use `PhantomData<(fn() -> S, fn() -> A, ...)>` to avoid inheriting bounds from phantom type parameters.

## Cross-Crate Dependencies

- **From `solverforge-core`:** `Score`, `PlanningSolution`, `ConstraintRef`, `ImpactType`, `SolutionDescriptor`, `EntityDescriptor`, `ProblemFactDescriptor`, `VariableDescriptor`, `EntityCollectionExtractor`
