# solverforge-scoring WIREFRAME

Zero-erasure incremental constraint scoring infrastructure for SolverForge.

**Location:** `crates/solverforge-scoring/`

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
│   │   ├── mod.rs                                  — Re-exports ConstraintSet, IncrementalConstraint, ConstraintResult
│   │   ├── incremental.rs                          — IncrementalConstraint trait, ConstraintSet trait, tuple impls (0..16)
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
│   ├── complemented.rs                             — ComplementedGroupConstraint<S,A,B,K,EA,EB,KA,KB,C,D,W,Sc>
│   ├── cross_bi_incremental.rs                     — IncrementalCrossBiConstraint<S,A,B,K,EA,EB,KA,KB,F,W,Sc>
│   ├── flattened_bi.rs                             — FlattenedBiConstraint<S,A,B,C,K,CK,EA,EB,KA,KB,Flatten,CKeyFn,ALookup,F,W,Sc>
│   ├── if_exists.rs                                — IfExistsUniConstraint<S,A,B,K,EA,EB,KA,KB,FA,W,Sc>, ExistenceMode enum
│   ├── nary_incremental/
│   │   ├── mod.rs                                  — Re-exports all nary constraint macros
│   │   ├── bi.rs                                   — impl_incremental_bi_constraint! macro → IncrementalBiConstraint
│   │   └── nary_unified.rs                         — impl_incremental_tri/quad/penta_constraint! macros → IncrementalTriConstraint, IncrementalQuadConstraint, IncrementalPentaConstraint
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
│       └── if_exists.rs                            — IfExistsUniConstraint tests
├── director/
│   ├── mod.rs                                      — Re-exports all director types and traits
│   ├── traits.rs                                   — Director<S> trait
│   ├── typed.rs                                    — ScoreDirector<S,C> (zero-erasure incremental)
│   ├── recording.rs                                — RecordingDirector<'a,S,D> (automatic undo tracking)
│   ├── shadow_aware.rs                             — ShadowVariableSupport trait, SolvableSolution trait
│   └── tests/
│       ├── mod.rs                                  — Test module declarations
│       ├── bench.rs                                — Benchmark test module declarations
│       ├── benchmarks.rs                           — Performance comparison tests
│       ├── typed.rs                                — ScoreDirector tests
│       ├── recording.rs                            — RecordingDirector tests
│       ├── shadow.rs                               — Shadow-aware director tests

├── stream/
│   ├── mod.rs                                      — Module declarations and re-exports for all stream types
│   ├── factory.rs                                  — ConstraintFactory<S,Sc>
│   ├── uni_stream.rs                               — UniConstraintStream, UniConstraintBuilder
│   ├── bi_stream.rs                                — BiConstraintStream, BiConstraintBuilder (via macro)
│   ├── tri_stream.rs                               — TriConstraintStream, TriConstraintBuilder (via macro)
│   ├── quad_stream.rs                              — QuadConstraintStream, QuadConstraintBuilder (via macro)
│   ├── penta_stream.rs                             — PentaConstraintStream, PentaConstraintBuilder (via macro)
│   ├── grouped_stream.rs                           — GroupedConstraintStream, GroupedConstraintBuilder
│   ├── balance_stream.rs                           — BalanceConstraintStream, BalanceConstraintBuilder
│   ├── complemented_stream.rs                      — ComplementedConstraintStream, ComplementedConstraintBuilder
│   ├── cross_bi_stream.rs                          — CrossBiConstraintStream, CrossBiConstraintBuilder
│   ├── flattened_bi_stream.rs                      — FlattenedBiConstraintStream, FlattenedBiConstraintBuilder
│   ├── if_exists_stream.rs                         — IfExistsStream, IfExistsBuilder
│   ├── collection_extract.rs                       — CollectionExtract trait, VecExtract wrapper, vec() constructor
│   ├── join_target.rs                              — JoinTarget trait + 3 impls (self-join, keyed cross-join, predicate cross-join)
│   ├── key_extract.rs                              — KeyExtract trait, EntityKeyAdapter struct
│   ├── arity_stream_macros/
│   │   ├── mod.rs                                  — impl_arity_stream! dispatcher macro
│   │   └── nary_stream.rs                          — impl_bi/tri/quad/penta_arity_stream! macros
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
    IncrementalUniConstraint,
};

// Constraint Set
pub use api::constraint_set::{ConstraintResult, ConstraintSet, IncrementalConstraint};
pub use api::weight_overrides::{ConstraintWeightOverrides, WeightProvider};

// Score Directors
pub use director::score_director::ScoreDirector;
pub use director::{
    RecordingDirector, Director, ShadowVariableSupport, SolvableSolution,
};

// Analysis
pub use api::analysis::{
    ConstraintAnalysis, ConstraintJustification, DetailedConstraintEvaluation,
    DetailedConstraintMatch, EntityRef, Indictment, IndictmentMap, ScoreExplanation,
};

// Fluent Stream API
pub use stream::{
    BiConstraintBuilder, BiConstraintStream, ConstraintFactory, GroupedConstraintBuilder,
    GroupedConstraintStream, UniConstraintBuilder, UniConstraintStream,
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
| `is_incremental` | `fn is_incremental(&self) -> bool` | Default: false |
| `reset` | `fn reset(&mut self)` | Default: no-op |
| `register_undo` | `fn register_undo(&mut self, _undo: Box<dyn FnOnce(&mut S) + Send>)` | Default: no-op |

### `IncrementalConstraint<S, Sc: Score>` — `Send + Sync`

| Method | Signature | Note |
|--------|-----------|------|
| `evaluate` | `fn evaluate(&self, solution: &S) -> Sc` | Full recalculation |
| `match_count` | `fn match_count(&self, solution: &S) -> usize` | Number of matches |
| `initialize` | `fn initialize(&mut self, solution: &S) -> Sc` | Initialize state for incremental |
| `on_insert` | `fn on_insert(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc` | Incremental insert delta |
| `on_retract` | `fn on_retract(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc` | Incremental retract delta |
| `reset` | `fn reset(&mut self)` | Clear incremental state |
| `name` | `fn name(&self) -> &str` | Constraint name |
| `is_hard` | `fn is_hard(&self) -> bool` | Default: false |
| `constraint_ref` | `fn constraint_ref(&self) -> ConstraintRef` | Default: from name |
| `get_matches` | `fn get_matches(&self, _solution: &S) -> Vec<DetailedConstraintMatch<Sc>>` | Default: empty |
| `weight` | `fn weight(&self) -> Sc` | Default: Sc::zero() |

### `ConstraintSet<S, Sc: Score>` — `Send + Sync`

| Method | Signature | Note |
|--------|-----------|------|
| `evaluate_all` | `fn evaluate_all(&self, solution: &S) -> Sc` | Sum all constraints |
| `constraint_count` | `fn constraint_count(&self) -> usize` | Number of constraints |
| `evaluate_each` | `fn evaluate_each(&self, solution: &S) -> Vec<ConstraintResult<Sc>>` | Per-constraint results |
| `evaluate_detailed` | `fn evaluate_detailed(&self, solution: &S) -> Vec<ConstraintAnalysis<Sc>>` | With match details |
| `initialize_all` | `fn initialize_all(&mut self, solution: &S) -> Sc` | Initialize all for incremental |
| `on_insert_all` | `fn on_insert_all(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc` | Incremental insert all |
| `on_retract_all` | `fn on_retract_all(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc` | Incremental retract all |
| `reset_all` | `fn reset_all(&mut self)` | Reset all |

Implemented for tuples `()` through `(C0, C1, ..., C15)` where each `Ci: IncrementalConstraint<S, Sc>`.

### `ShadowVariableSupport` — `: PlanningSolution`

| Method | Signature | Note |
|--------|-----------|------|
| `update_entity_shadows` | `fn update_entity_shadows(&mut self, entity_index: usize)` | Update shadows for one entity |
| `update_all_shadows` | `fn update_all_shadows(&mut self)` | Default: no-op |

### `SolvableSolution` — `: ShadowVariableSupport`

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
- Key methods: `new()`, `with_descriptor()`, `simple()` (convenience for `ScoreDirector<S, ()>`), `simple_zero()` (test helper with empty descriptor), `calculate_score()`, `before_variable_changed()`, `after_variable_changed()`, `after_variable_changed_with_shadows()` (where `S: ShadowVariableSupport`), `do_change()`, `do_change_with_shadows()`, `get_score()`, `constraint_match_totals()`, `into_working_solution()`, `take_solution()`
- `simple(solution, descriptor, entity_counter)` — creates `ScoreDirector<S, ()>` with empty constraint set
- `simple_zero(solution)` — creates `ScoreDirector<S, ()>` with empty descriptor and zero entity counter
- Implements `Director<S>`

**`RecordingDirector<'a, S, D>`** where `S: PlanningSolution`, `D: Director<S>`
- Wraps any director with automatic undo tracking.
- Key methods: `new()`, `undo_changes()`, `reset()`, `change_count()`, `is_empty()`
- Implements `Director<S>`

### Constraint Types

All implement `IncrementalConstraint<S, Sc>`.

**`IncrementalUniConstraint<S, A, E, F, W, Sc>`** — Single-collection constraint with filter and weight.

**`IncrementalBiConstraint<S, A, K, E, KE, F, W, Sc>`** — Self-join bi constraint (pairs from same collection).

**`IncrementalTriConstraint<S, A, K, E, KE, F, W, Sc>`** — Self-join tri constraint (triples).

**`IncrementalQuadConstraint<S, A, K, E, KE, F, W, Sc>`** — Self-join quad constraint.

**`IncrementalPentaConstraint<S, A, K, E, KE, F, W, Sc>`** — Self-join penta constraint.

**`IncrementalCrossBiConstraint<S, A, B, K, EA, EB, KA, KB, F, W, Sc>`** — Cross-collection bi constraint (two different collections joined by key).

**`GroupedUniConstraint<S, A, K, E, Fi, KF, C, W, Sc>`** where `C: UniCollector<A>` — Group-by with collector and weight on result.

**`BalanceConstraint<S, A, K, E, F, KF, Sc>`** — Load balancing using sum-of-squared-deviations.

**`ComplementedGroupConstraint<S, A, B, K, EA, EB, KA, KB, C, D, W, Sc>`** where `C: UniCollector<A>` — Group-by complemented against a second collection (for supply vs demand).

**`FlattenedBiConstraint<S, A, B, C, K, CK, EA, EB, KA, KB, Flatten, CKeyFn, ALookup, F, W, Sc>`** — Cross-collection with nested collection flattening.

**`IfExistsUniConstraint<S, A, B, K, EA, EB, KA, KB, FA, W, Sc>`** — Existence/non-existence check.

**`ExistenceMode`** — `enum { Exists, NotExists }`

### Analysis Types

**`ConstraintResult<Sc>`** — `{ name: String, score: Sc, match_count: usize, is_hard: bool }`

**`EntityRef`** — `{ type_name: String, display: String, entity: Arc<dyn Any + Send + Sync> }`
- Methods: `new()`, `with_display()`, `as_entity::<T>()`, `short_type_name()`
- Implements `Hash + Eq` (by display string)

**`ConstraintJustification`** — `{ entities: Vec<EntityRef>, description: String }`

**`DetailedConstraintMatch<Sc: Score>`** — `{ constraint_ref: ConstraintRef, score: Sc, justification: ConstraintJustification }`

**`DetailedConstraintEvaluation<Sc: Score>`** — `{ total_score: Sc, match_count: usize, matches: Vec<DetailedConstraintMatch<Sc>> }`

**`ConstraintAnalysis<Sc: Score>`** — `{ constraint_ref: ConstraintRef, weight: Sc, score: Sc, matches: Vec<DetailedConstraintMatch<Sc>>, is_hard: bool }`

**`ScoreExplanation<Sc: Score>`** — `{ score: Sc, constraint_analyses: Vec<ConstraintAnalysis<Sc>> }`
- Methods: `total_match_count()`, `non_zero_constraints()`, `all_matches()`

**`Indictment<Sc: Score>`** — `{ entity: EntityRef, score: Sc, constraint_matches: HashMap<ConstraintRef, Vec<DetailedConstraintMatch<Sc>>> }`
- Methods: `add_match()`, `match_count()`, `violated_constraints()`, `constraint_count()`

**`IndictmentMap<Sc: Score>`** — `{ indictments: HashMap<EntityRef, Indictment<Sc>> }`
- Methods: `from_matches()`, `get()`, `entities()`, `worst_entities()`, `len()`, `is_empty()`

**`ConstraintWeightOverrides<Sc: Score>`** — `{ weights: HashMap<String, Sc> }`
- Methods: `new()`, `from_pairs()`, `put()`, `remove()`, `get_or_default()`, `get()`, `contains()`, `len()`, `is_empty()`, `clear()`, `into_arc()`

### Stream Builders (Fluent API)

**`ConstraintFactory<S, Sc: Score>`** — Entry point.
- `new()`, `for_each()` → `UniConstraintStream`

**`UniConstraintStream<S, A, E, F, Sc>`** — Single collection stream.
- Operations: `filter()`, `join(target)` (unified dispatch via `JoinTarget`), `group_by()`, `balance()`, `if_exists_filtered()`, `if_not_exists_filtered()`, `penalize()`, `penalize_with()`, `penalize_hard_with()`, `penalize_hard()`, `penalize_soft()`, `reward()`, `reward_with()`, `reward_hard_with()`, `reward_hard()`, `reward_soft()`
- `join()` dispatch: `equal(|a| key)` → self-join `BiConstraintStream`; `(extractor_b, equal_bi(ka, kb))` → keyed `CrossBiConstraintStream`; `(other_stream, |a, b| pred)` → predicate `CrossBiConstraintStream`
- `into_parts()` → `(E, F)`, `from_parts(extractor, filter)` → `Self`, `extractor()` → `&E`

**`UniConstraintBuilder<S, A, E, F, W, Sc>`** — `for_descriptor()`, `named()` → `IncrementalUniConstraint`

**`BiConstraintStream<S, A, K, E, KE, F, Sc>`** — Self-join bi stream (macro-generated).
- Operations: `filter()`, `join()` → TriStream, `penalize()`, `penalize_with()`, `penalize_hard_with()`, `penalize_hard()`, `penalize_soft()`, `reward()`, `reward_with()`, `reward_hard_with()`, `reward_hard()`, `reward_soft()`

**`BiConstraintBuilder<S, A, K, E, KE, F, W, Sc>`** — `named()` → `IncrementalBiConstraint`

**`TriConstraintStream/Builder`** — Same pattern, tri-arity. `join()` → QuadStream. Same convenience methods.

**`QuadConstraintStream/Builder`** — Same pattern, quad-arity. `join()` → PentaStream. Same convenience methods.

**`PentaConstraintStream/Builder`** — Same pattern, penta-arity. Terminal (no further joins). Same convenience methods.

**`CrossBiConstraintStream<S, A, B, K, EA, EB, KA, KB, F, Sc>`** — Cross-collection bi stream.
- Operations: `filter()`, `penalize()`, `penalize_with()`, `penalize_hard_with()`, `penalize_hard()`, `penalize_soft()`, `reward()`, `reward_with()`, `reward_hard_with()`, `reward_hard()`, `reward_soft()`, `flatten_last()` → FlattenedBiStream

**`CrossBiConstraintBuilder`** — `named()` → `IncrementalCrossBiConstraint`

**`GroupedConstraintStream<S, A, K, E, Fi, KF, C, Sc>`** — Grouped stream.
- Operations: `penalize_with()`, `penalize_hard_with()`, `penalize_hard()`, `penalize_soft()`, `reward_with()`, `reward_hard_with()`, `reward_hard()`, `reward_soft()`, `complement()`, `complement_with_key()` → ComplementedStream

**`GroupedConstraintBuilder`** — `named()` → `GroupedUniConstraint`

**`BalanceConstraintStream/Builder`** — Balance stream. `penalize()`, `penalize_hard()`, `penalize_soft()`, `reward()`, `reward_hard()`, `reward_soft()`, `named()` → `BalanceConstraint`

**`ComplementedConstraintStream/Builder`** — Complemented stream. `penalize_with()`, `penalize_hard()`, `penalize_soft()`, `reward_with()`, `reward_hard()`, `reward_soft()`, `named()` → `ComplementedGroupConstraint`

**`FlattenedBiConstraintStream/Builder`** — Flattened bi stream. `filter()`, `penalize()`, `penalize_with()`, `penalize_hard()`, `penalize_soft()`, `reward()`, `reward_hard()`, `reward_soft()`, `named()` → `FlattenedBiConstraint`

**`IfExistsStream/IfExistsBuilder`** — If-exists stream. `penalize()`, `penalize_hard()`, `penalize_soft()`, `reward()`, `reward_hard()`, `reward_soft()`, `named()` → `IfExistsUniConstraint`

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

### Join Support Types

**`JoinTarget<S, A, E, F, Sc>`** — Trait for unified `.join()` dispatch on `UniConstraintStream`.
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

`ConstraintSet` is implemented for tuples of up to 16 elements via a macro. Each tuple element must implement `IncrementalConstraint<S, Sc>`. Operations iterate over all tuple elements, summing scores. This is the zero-erasure alternative to `Vec<Box<dyn Constraint>>`.

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
