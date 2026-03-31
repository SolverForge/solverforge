# solverforge-core WIREFRAME

Core types and traits for the SolverForge constraint solver framework.

**Location:** `crates/solverforge-core/`

## Dependencies

- `thiserror` (workspace) — error derive macros
- `serde` (workspace, optional, feature-gated) — serialization for score types

**Features:** `default`, `decimal`, `serde`

## File Map

```
src/
├── lib.rs                                 — Crate root; module declarations, public re-exports
├── error.rs                               — SolverForgeError enum and Result type alias
├── constraint.rs                          — ConstraintRef (package/name identifier) and ImpactType enum
├── constraint_tests.rs                    — Tests for ConstraintRef and ImpactType
├── score/
│   ├── mod.rs                             — Module declarations and re-exports for score types
│   ├── traits.rs                          — Score trait, ParseableScore trait, ScoreParseError
│   ├── macros.rs                          — Declarative macros: impl_score_ops!, impl_score_scale!, impl_score_parse!
│   ├── level.rs                           — ScoreLevel enum (Hard, Medium, Soft)
│   ├── simple.rs                          — SoftScore: single i64 level
│   ├── hard_soft.rs                       — HardSoftScore: two i64 levels (hard, soft)
│   ├── hard_medium_soft.rs                — HardMediumSoftScore: three i64 levels
│   ├── hard_soft_decimal.rs               — HardSoftDecimalScore: two levels with x100000 scaling
│   ├── bendable.rs                        — BendableScore<H, S>: const-generic multi-level score
│   └── tests/
│       ├── mod.rs                         — Test module declarations
│       ├── simple_score.rs                — SoftScore tests
│       ├── hard_soft_score.rs             — HardSoftScore tests
│       ├── hard_medium_soft_score.rs      — HardMediumSoftScore tests
│       ├── hard_soft_decimal_score.rs     — HardSoftDecimalScore tests
│       └── bendable_score.rs              — BendableScore tests
├── domain/
│   ├── mod.rs                             — Module declarations and re-exports for domain types
│   ├── traits.rs                          — PlanningSolution, PlanningEntity, ProblemFact, PlanningId, ListVariableSolution
│   ├── entity_ref.rs                      — EntityRef, EntityExtractor trait, EntityCollectionExtractor
│   ├── variable.rs                        — VariableType, ShadowVariableKind, ValueRangeType, ChainedVariableInfo
│   ├── value_range.rs                     — ValueRangeProvider trait, FieldValueRangeProvider, ComputedValueRangeProvider, StaticValueRange, IntegerRange
│   ├── descriptor/
│   │   ├── mod.rs                         — Re-exports descriptor types
│   │   ├── entity.rs                      — EntityDescriptor struct
│   │   ├── solution.rs                    — SolutionDescriptor struct
│   │   ├── problem_fact.rs                — ProblemFactDescriptor struct
│   │   ├── var_descriptor.rs              — VariableDescriptor struct
│   │   └── tests/
│   │       ├── mod.rs                     — Shared test helpers and module declarations
│   │       ├── entity_descriptor.rs       — EntityDescriptor tests
│   │       ├── solution_descriptor.rs     — SolutionDescriptor tests
│   │       └── variable_descriptor.rs     — VariableDescriptor tests
│   ├── listener/
│   │   ├── mod.rs                         — Re-exports listener traits and notification enums
│   │   ├── traits.rs                      — VariableListener, ListVariableListener traits; VariableNotification, ListVariableNotification enums
│   │   └── tests.rs                       — Listener call counting tests
│   ├── supply/
│   │   ├── mod.rs                         — Re-exports supply types
│   │   ├── inverse.rs                     — InverseSupply<V>: value → entity_index mapping
│   │   ├── anchor.rs                      — AnchorSupply: entity_idx → anchor_idx mapping
│   │   ├── list_state.rs                  — ListStateSupply<E>: element → (entity_idx, list_idx) mapping; ElementPosition
│   │   └── tests.rs                       — Supply tests (inverse, anchor, list_state)
│   └── tests/
│       ├── mod.rs                         — Test module declarations
│       ├── entity_ref_tests.rs            — EntityCollectionExtractor tests
│       ├── value_range_tests.rs           — ValueRangeProvider tests
│       └── variable_tests.rs             — VariableType, ShadowVariableKind, ChainedVariableInfo tests
```

## Public Re-exports (lib.rs)

```rust
pub use constraint::{ConstraintRef, ImpactType};
pub use domain::{PlanningEntity, PlanningId, PlanningSolution, ProblemFact};
pub use error::SolverForgeError;
pub use score::{
    BendableScore, HardMediumSoftScore, HardSoftDecimalScore, HardSoftScore,
    ParseableScore, Score, ScoreParseError, SoftScore,
};
```

Additionally, `pub mod constraint`, `pub mod domain`, `pub mod error`, `pub mod score` are all public modules.

## Public Traits

### `Score`

**Bounds:** `Copy + Debug + Display + Default + Send + Sync + PartialEq + Eq + PartialOrd + Ord + Add<Output=Self> + Sub<Output=Self> + Neg<Output=Self> + 'static`

| Method | Signature | Note |
|--------|-----------|------|
| `is_feasible` | `fn is_feasible(&self) -> bool` | True when all hard scores >= 0 |
| `zero` | `fn zero() -> Self` | Additive identity |
| `levels_count` | `fn levels_count() -> usize` | Number of score levels |
| `to_level_numbers` | `fn to_level_numbers(&self) -> Vec<i64>` | High-priority first |
| `from_level_numbers` | `fn from_level_numbers(levels: &[i64]) -> Self` | Panics on wrong count |
| `multiply` | `fn multiply(&self, multiplicand: f64) -> Self` | Scalar multiply |
| `divide` | `fn divide(&self, divisor: f64) -> Self` | Scalar divide |
| `abs` | `fn abs(&self) -> Self` | Absolute value |
| `to_scalar` | `fn to_scalar(&self) -> f64` | Weighted single f64 for SA temperature |
| `level_label` | `fn level_label(index: usize) -> ScoreLevel` | Semantic label per level |
| `compare` | `fn compare(&self, other: &Self) -> Ordering` | Default: `self.cmp(other)` |
| `is_better_than` | `fn is_better_than(&self, other: &Self) -> bool` | Default: `self > other` |
| `is_worse_than` | `fn is_worse_than(&self, other: &Self) -> bool` | Default: `self < other` |
| `is_equal_to` | `fn is_equal_to(&self, other: &Self) -> bool` | Default: `self == other` |
| `one_hard` | `fn one_hard() -> Self` | Score with 1 at first Hard level, 0 elsewhere |
| `one_soft` | `fn one_soft() -> Self` | Score with 1 at last Soft level, 0 elsewhere |
| `one_medium` | `fn one_medium() -> Self` | Score with 1 at first Medium level, 0 elsewhere |

### `ParseableScore`

**Bounds:** `Score`

| Method | Signature | Note |
|--------|-----------|------|
| `parse` | `fn parse(s: &str) -> Result<Self, ScoreParseError>` | Parse from string (e.g. `"0hard/-100soft"`) |
| `to_string_repr` | `fn to_string_repr(&self) -> String` | Serialized string form |

### `PlanningSolution`

**Bounds:** `Clone + Send + Sync + 'static`

| Method | Signature | Note |
|--------|-----------|------|
| `score` | `fn score(&self) -> Option<Self::Score>` | Current score, None if unscored |
| `set_score` | `fn set_score(&mut self, score: Option<Self::Score>)` | Set the score |
| `is_initialized` | `fn is_initialized(&self) -> bool` | Default: `true` |

**Associated type:** `type Score: Score`

### `PlanningEntity`

**Bounds:** `Clone + Send + Sync + Any + 'static`

| Method | Signature | Note |
|--------|-----------|------|
| `is_pinned` | `fn is_pinned(&self) -> bool` | Default: `false` |
| `as_any` | `fn as_any(&self) -> &dyn Any` | Dynamic typing support |
| `as_any_mut` | `fn as_any_mut(&mut self) -> &mut dyn Any` | Mutable dynamic typing |

### `ProblemFact`

**Bounds:** `Clone + Send + Sync + Any + 'static`

| Method | Signature | Note |
|--------|-----------|------|
| `as_any` | `fn as_any(&self) -> &dyn Any` | Dynamic typing support |

### `PlanningId`

| Method | Signature | Note |
|--------|-----------|------|
| `planning_id` | `fn planning_id(&self) -> Self::Id` | Stable unique identifier |

**Associated type:** `type Id: Eq + Hash + Clone + Send + Sync + 'static`

### `ListVariableSolution`

**Bounds:** `PlanningSolution`

| Method | Signature | Note |
|--------|-----------|------|
| `entity_count` | `fn entity_count(&self) -> usize` | Number of list owners |
| `list_len` | `fn list_len(&self, entity_idx: usize) -> usize` | Length of entity's list |
| `list_get` | `fn list_get(&self, entity_idx: usize, position: usize) -> Self::Element` | Element at position |
| `list_push` | `fn list_push(&mut self, entity_idx: usize, elem: Self::Element)` | Append to list |
| `list_insert` | `fn list_insert(&mut self, entity_idx: usize, position: usize, elem: Self::Element)` | Insert at position |
| `list_remove` | `fn list_remove(&mut self, entity_idx: usize, position: usize) -> Self::Element` | Remove at position |
| `list_reverse` | `fn list_reverse(&mut self, entity_idx: usize, start: usize, end: usize)` | Reverse range [start, end) |
| `unassigned_elements` | `fn unassigned_elements(&self) -> Vec<Self::Element>` | Elements not in any list |

**Associated type:** `type Element: Copy + Send + Sync`

### `ValueRangeProvider<S, V>`

**Bounds:** `Send + Sync`

| Method | Signature | Note |
|--------|-----------|------|
| `get_values` | `fn get_values(&self, solution: &S) -> Vec<V>` | All possible values |
| `value_count` | `fn value_count(&self, solution: &S) -> usize` | Default: `get_values().len()` |
| `is_empty` | `fn is_empty(&self, solution: &S) -> bool` | Default: `value_count() == 0` |

### `EntityExtractor`

**Bounds:** `Send + Sync`

| Method | Signature | Note |
|--------|-----------|------|
| `count` | `fn count(&self, solution: &dyn Any) -> Option<usize>` | Entity count in collection |
| `get` | `fn get<'a>(&self, solution: &'a dyn Any, index: usize) -> Option<&'a dyn Any>` | Get entity by index |
| `get_mut` | `fn get_mut<'a>(&self, solution: &'a mut dyn Any, index: usize) -> Option<&'a mut dyn Any>` | Mutable entity by index |
| `entity_refs` | `fn entity_refs(&self, solution: &dyn Any) -> Vec<EntityRef>` | All entity references |
| `clone_box` | `fn clone_box(&self) -> Box<dyn EntityExtractor>` | Clone as trait object |
| `clone_entity_boxed` | `fn clone_entity_boxed(&self, solution: &dyn Any, index: usize) -> Option<Box<dyn Any + Send + Sync>>` | Clone entity as boxed Any |
| `entity_type_id` | `fn entity_type_id(&self) -> TypeId` | TypeId of entity type |

### `VariableListener<Solution, Entity>`

**Bounds:** `Send + Sync`

| Method | Signature | Note |
|--------|-----------|------|
| `before_variable_changed` | `fn before_variable_changed(&mut self, solution: &Solution, entity: &Entity)` | Capture old state |
| `after_variable_changed` | `fn after_variable_changed(&mut self, solution: &mut Solution, entity: &Entity)` | Update shadows |
| `requires_unique_entity_events` | `fn requires_unique_entity_events(&self) -> bool` | Default: `false` |
| `reset_working_solution` | `fn reset_working_solution(&mut self, _solution: &Solution)` | Default: no-op |
| `close` | `fn close(&mut self)` | Default: no-op |

### `ListVariableListener<Solution, Entity, Element>`

**Bounds:** `Send + Sync`

| Method | Signature | Note |
|--------|-----------|------|
| `after_element_unassigned` | `fn after_element_unassigned(&mut self, solution: &mut Solution, element: &Element)` | Reset shadow vars on element |
| `before_list_variable_changed` | `fn before_list_variable_changed(&mut self, solution: &Solution, entity: &Entity, from_index: usize, to_index: usize)` | Capture state before range change |
| `after_list_variable_changed` | `fn after_list_variable_changed(&mut self, solution: &mut Solution, entity: &Entity, from_index: usize, to_index: usize)` | Update shadows after range change |
| `reset_working_solution` | `fn reset_working_solution(&mut self, _solution: &Solution)` | Default: no-op |
| `close` | `fn close(&mut self)` | Default: no-op |

## Public Structs

### Score Types

#### `SoftScore`

Fields: `score: i64` (private)

Constants: `ZERO`, `ONE`

Constructors: `of(i64)`, `From<i64>`

Accessors: `score() -> i64`

Implements: `Score`, `ParseableScore`, `Ord`, `Copy`, `Eq`, `Hash`, `Default`, `Debug`, `Display`

Optional: `serde::Serialize + Deserialize` (feature `serde`)

#### `HardSoftScore`

Fields: `hard: i64`, `soft: i64` (private)

Constants: `ZERO`, `ONE_HARD`, `ONE_SOFT`

Constructors: `of(i64, i64)`, `of_hard(i64)`, `of_soft(i64)`

Accessors: `hard() -> i64`, `soft() -> i64`, `hard_score() -> HardSoftScore`, `soft_score() -> HardSoftScore`

Implements: `Score`, `ParseableScore`, `Ord`, `Copy`, `Eq`, `Hash`, `Default`, `Debug`, `Display`

#### `HardMediumSoftScore`

Fields: `hard: i64`, `medium: i64`, `soft: i64` (private)

Constants: `ZERO`, `ONE_HARD`, `ONE_MEDIUM`, `ONE_SOFT`

Constructors: `of(i64, i64, i64)`, `of_hard(i64)`, `of_medium(i64)`, `of_soft(i64)`

Accessors: `hard() -> i64`, `medium() -> i64`, `soft() -> i64`

Implements: `Score`, `ParseableScore`, `Ord`, `Copy`, `Eq`, `Hash`, `Default`, `Debug`, `Display`

#### `HardSoftDecimalScore`

Fields: `hard: i64`, `soft: i64` (private, stored pre-scaled by 100,000)

Constants: `ZERO`, `ONE_HARD`, `ONE_SOFT`

Constructors: `of(i64, i64)` (unscaled), `of_scaled(i64, i64)` (pre-scaled), `of_hard(i64)`, `of_soft(i64)`, `of_hard_scaled(i64)`, `of_soft_scaled(i64)`

Accessors: `hard_scaled() -> i64`, `soft_scaled() -> i64`, `hard_score() -> HardSoftDecimalScore`, `soft_score() -> HardSoftDecimalScore`, `has_hard_component() -> bool`

Implements: `Score`, `ParseableScore`, `Ord`, `Copy`, `Eq`, `Hash`, `Default`, `Debug`, `Display`

#### `BendableScore<const H: usize, const S: usize>`

Fields: `hard: [i64; H]`, `soft: [i64; S]` (private)

Constructors: `of([i64; H], [i64; S])`, `zero()`, `one_hard(usize)`, `one_soft(usize)`

Accessors: `hard_levels_count() -> usize`, `soft_levels_count() -> usize`, `hard_score(usize) -> i64`, `soft_score(usize) -> i64`, `hard_scores() -> &[i64; H]`, `soft_scores() -> &[i64; S]`

Implements: `Score`, `Ord`, `Copy`, `Eq`, `Hash`, `Default`, `Debug`, `Display`

Does **not** implement `ParseableScore`.

### Other Score Types

#### `ScoreLevel`

```rust
enum ScoreLevel { Hard, Medium, Soft }
```

Derives: `Debug, Clone, Copy, PartialEq, Eq, Hash`

#### `ScoreParseError`

Fields: `pub message: String`

Implements: `Debug`, `Clone`, `PartialEq`, `Eq`, `Display`, `std::error::Error`

### Domain Types

#### `EntityRef`

```rust
pub struct EntityRef {
    pub index: usize,
    pub type_name: &'static str,
    pub collection_field: &'static str,
}
```

Derives: `Debug, Clone`

#### `EntityCollectionExtractor<S, E>`

```rust
pub struct EntityCollectionExtractor<S, E> {
    type_name: &'static str,                       // private
    collection_field: &'static str,                 // private
    get_collection: fn(&S) -> &Vec<E>,              // private
    get_collection_mut: fn(&mut S) -> &mut Vec<E>,  // private
}
```

Bounds on impl: `S: Send + Sync + 'static`, `E: Clone + Send + Sync + 'static`

Implements: `EntityExtractor`, `Debug`

#### `VariableType`

```rust
pub enum VariableType {
    Genuine,
    Chained,
    List,
    Shadow(ShadowVariableKind),
}
```

Methods: `is_genuine()`, `is_shadow()`, `is_list()`, `is_chained()`, `is_basic()`

#### `ShadowVariableKind`

```rust
pub enum ShadowVariableKind {
    Custom, InverseRelation, Index, NextElement,
    PreviousElement, Anchor, Cascading, Piggyback,
}
```

Methods: `requires_listener()`, `is_automatic()`, `is_piggyback()`

#### `ValueRangeType`

```rust
pub enum ValueRangeType {
    Collection,
    CountableRange { from: i64, to: i64 },
    EntityDependent,
}
```

#### `ChainedVariableInfo`

```rust
pub struct ChainedVariableInfo {
    pub anchor_type_id: TypeId,
    pub entity_type_id: TypeId,
    pub has_anchor_shadow: bool,
}
```

Methods: `new::<Anchor, Entity>()`, `with_anchor_shadow::<Anchor, Entity>()`, `is_anchor_type(TypeId)`, `is_entity_type(TypeId)`

### Value Range Providers

#### `FieldValueRangeProvider<S, V, F>`

Generic over `F: Fn(&S) -> &Vec<V> + Send + Sync`. Uses `PhantomData<(fn() -> S, fn() -> V)>`.

#### `ComputedValueRangeProvider<S, V, F>`

Generic over `F: Fn(&S) -> Vec<V> + Send + Sync`. Uses `PhantomData<(fn() -> S, fn() -> V)>`.

Static method: `value_range_type() -> ValueRangeType` (returns `EntityDependent`).

#### `StaticValueRange<V>`

Fields: `values: Vec<V>` (private)

#### `IntegerRange`

Fields: `start: i64`, `end: i64` (private)

Constructors: `new(i64, i64)`, `from_zero(i64)`

Method: `value_range_type() -> ValueRangeType`

Implements `ValueRangeProvider<S, i64>` and `ValueRangeProvider<S, i32>` for any `S: Send + Sync`.

### Descriptors

#### `EntityDescriptor`

```rust
pub struct EntityDescriptor {
    pub type_name: &'static str,
    pub type_id: TypeId,
    pub solution_field: &'static str,
    pub is_collection: bool,
    pub variable_descriptors: Vec<VariableDescriptor>,
    pub extractor: Option<Box<dyn EntityExtractor>>,
    pub id_field: Option<&'static str>,
    pub pin_field: Option<&'static str>,
}
```

Builder methods: `with_extractor()`, `with_variable()`, `with_id_field()`, `with_pin_field()`

Query methods: `genuine_variable_descriptors()`, `shadow_variable_descriptors()`, `find_variable(&str)`, `has_genuine_variables()`, `has_extractor()`, `entity_count(&dyn Any)`, `get_entity()`, `get_entity_mut()`, `entity_refs()`, `for_each_entity()`, `for_each_entity_mut()`

Manual `Clone` and `Debug` impls.

#### `SolutionDescriptor`

```rust
pub struct SolutionDescriptor {
    pub type_name: &'static str,
    pub type_id: TypeId,
    pub entity_descriptors: Vec<EntityDescriptor>,
    pub problem_fact_descriptors: Vec<ProblemFactDescriptor>,
    pub score_field: &'static str,
    pub score_is_optional: bool,
    entity_type_index: HashMap<TypeId, usize>,  // private, O(1) lookup
}
```

Builder methods: `with_entity()`, `with_problem_fact()`, `with_score_field()`

Query methods: `find_entity_descriptor(&str)`, `find_entity_descriptor_by_type(TypeId)`, `genuine_variable_descriptors()`, `shadow_variable_descriptors()`, `total_entity_count(&dyn Any)`, `all_entity_refs(&dyn Any)`, `for_each_entity()`, `get_entity()`, `get_entity_mut()`, `entity_descriptor_count()`, `problem_fact_descriptor_count()`, `all_extractors_configured()`

#### `ProblemFactDescriptor`

```rust
pub struct ProblemFactDescriptor {
    pub type_name: &'static str,
    pub type_id: TypeId,
    pub solution_field: &'static str,
    pub is_collection: bool,
    pub id_field: Option<&'static str>,
    pub extractor: Option<Box<dyn EntityExtractor>>,
}
```

Builder methods: `with_extractor()`, `single()`, `with_id_field()`

#### `VariableDescriptor`

```rust
pub struct VariableDescriptor {
    pub name: &'static str,
    pub variable_type: VariableType,
    pub allows_unassigned: bool,
    pub value_range_provider: Option<&'static str>,
    pub value_range_type: ValueRangeType,
    pub source_variable: Option<&'static str>,
    pub source_entity: Option<&'static str>,
}
```

Constructors: `genuine(&str)`, `chained(&str)`, `list(&str)`, `shadow(&str, ShadowVariableKind)`, `piggyback(&str, &str)`

Builder methods: `with_value_range()`, `with_allows_unassigned()`, `with_value_range_type()`, `with_source()`

### Supply Types

#### `InverseSupply<V>` where `V: Eq + Hash`

Index-based mapping: value → entity_index. Backed by `HashMap<V, usize>`.

Methods: `new()`, `with_capacity(usize)`, `get(&V) -> Option<usize>`, `insert(V, usize) -> Option<usize>`, `remove(&V) -> Option<usize>`, `update(Option<&V>, V, usize)`, `clear()`, `len()`, `is_empty()`, `iter()`

#### `AnchorSupply`

Index-based mapping: entity_idx → anchor_idx. Backed by `HashMap<usize, usize>`.

Methods: `new()`, `with_capacity(usize)`, `get(usize) -> Option<usize>`, `set(usize, usize) -> Option<usize>`, `remove(usize) -> Option<usize>`, `cascade(impl IntoIterator<Item=usize>, usize)`, `clear()`, `len()`, `is_empty()`, `iter()`, `entities_for_anchor(usize) -> Vec<usize>`

#### `ListStateSupply<E>` where `E: Eq + Hash`

Index-based mapping: element → `ElementPosition { entity_idx, list_idx }`. Backed by `HashMap<E, ElementPosition>`.

Methods: `new()`, `with_unassigned(usize)`, `initialize(usize)`, `assign(E, usize, usize)`, `unassign(&E) -> Option<ElementPosition>`, `update(&E, usize, usize) -> bool`, `get_position(&E) -> Option<ElementPosition>`, `get_entity(&E) -> Option<usize>`, `get_list_index(&E) -> Option<usize>`, `is_assigned(&E) -> bool`, `unassigned_count()`, `assigned_count()`, `clear()`, `iter()`, `elements_for_entity(usize) -> Vec<&E>`

#### `ElementPosition`

```rust
pub struct ElementPosition {
    pub entity_idx: usize,
    pub list_idx: usize,
}
```

Derives: `Debug, Clone, Copy, PartialEq, Eq`

### Notification Enums

#### `VariableNotification`

```rust
pub enum VariableNotification { EntityAdded, EntityRemoved, VariableChanged }
```

#### `ListVariableNotification`

```rust
pub enum ListVariableNotification {
    ElementUnassigned,
    RangeChanged { from_index: usize, to_index: usize },
}
```

### Error Types

#### `SolverForgeError`

```rust
pub enum SolverForgeError {
    Config(String),
    DomainModel(String),
    ScoreCalculation(String),
    Cancelled,
    InvalidState(String),
    Internal(String),
}
```

Implements `thiserror::Error` + `Debug`.

Type alias: `pub type Result<T> = std::result::Result<T, SolverForgeError>`

### Constraint Types

#### `ConstraintRef`

```rust
pub struct ConstraintRef {
    pub package: String,
    pub name: String,
}
```

Methods: `new(impl Into<String>, impl Into<String>)`, `full_name() -> String`

Derives: `Debug, Clone, PartialEq, Eq, Hash`

#### `ImpactType`

```rust
pub enum ImpactType { Penalty, Reward }
```

Derives: `Debug, Clone, Copy, PartialEq, Eq, Hash`

## Architectural Notes

### Score Macros

Three declarative macros reduce boilerplate for score types:

- `impl_score_ops!($type { $fields } => $ctor)` — generates `PartialOrd`, `Add`, `Sub`, `Neg`
- `impl_score_scale!($type { $fields } => $ctor)` — generates `multiply`, `divide`, `abs` method bodies (used inside `impl Score`)
- `impl_score_parse!($type { $field => $suffix } => $ctor)` — generates `ParseableScore` impl for slash-separated formats

`SoftScore` and `HardSoftDecimalScore` have custom parse logic and do not use `impl_score_parse!`.

### Score Scalar Weighting

`to_scalar()` uses powers of 10^6 to separate levels:
- `HardSoftScore`: `hard * 1_000_000 + soft`
- `HardMediumSoftScore`: `hard * 1_000_000_000_000 + medium * 1_000_000 + soft`
- `BendableScore`: dynamic weighting via `10^((total_levels - 1 - i) * 6)`

### HardSoftDecimalScore Scaling

Internal values are stored multiplied by 100,000 (`SCALE` constant). `of()` auto-scales; `of_scaled()` takes pre-scaled values. Display strips trailing zeros.

### BendableScore Const Generics

`BendableScore<const H: usize, const S: usize>` uses fixed-size arrays `[i64; H]` and `[i64; S]`. This enables `Copy` with zero heap allocation. It cannot use the declarative macros due to const generic array iteration.

### Supply Zero-Erasure Design

All three supply types (InverseSupply, AnchorSupply, ListStateSupply) follow strict zero-erasure:
- Index-based: store `usize` indices, not cloned domain objects
- Owned: no `Arc`, `Box`, or `dyn`
- Mutation via `&mut self`: no interior mutability
- Generic type parameters preserved

### EntityExtractor as Intentional dyn Boundary

`EntityExtractor` is a `dyn` trait object — this is an **intentional type-erasure boundary**. Descriptors need to work with entities of unknown concrete type at runtime. The `EntityCollectionExtractor<S, E>` provides the concrete implementation that downcasts via `Any`. `Box<dyn EntityExtractor>` appears in `EntityDescriptor` and `ProblemFactDescriptor`.

### PhantomData Pattern

`FieldValueRangeProvider` and `ComputedValueRangeProvider` use `PhantomData<(fn() -> S, fn() -> V)>` (function pointer form) to avoid inheriting `Clone`/`Send`/`Sync` bounds from phantom type parameters.

## Cross-Crate Dependencies

This is the leaf crate — it depends on no other solverforge crates. All other solverforge crates depend on it.
