# solverforge-macros WIREFRAME

Proc-macro crate providing attribute macros and derive macros for SolverForge domain model structs.

**Location:** `crates/solverforge-macros/`

## Dependencies

- `syn` (workspace) — Rust syntax parsing
- `quote` (workspace) — Rust code generation
- `proc-macro2` (workspace) — Proc-macro token streams

No solverforge crate dependencies. Generated code references `::solverforge::__internal::*` and `::solverforge::*` at the call site.

## File Map

```
src/
├── lib.rs                — Crate root; attribute macros, derive macro entry points, shared helpers
├── planning_entity.rs    — PlanningEntityImpl derive: PlanningEntity trait, PlanningId, entity_descriptor()
├── planning_solution.rs  — PlanningSolutionImpl derive: PlanningSolution trait, descriptor(), VariableOperations, shadow support, Solvable/Analyzable
├── problem_fact.rs       — ProblemFactImpl derive: ProblemFact, PlanningId, problem_fact_descriptor()
```

## Attribute Macros (proc_macro_attribute)

### `#[planning_entity]` / `#[planning_entity(serde)]`

Applies to structs. Adds derives: `Clone, Debug, PartialEq, Eq, Hash, PlanningEntityImpl`. Optionally adds `serde::Serialize, serde::Deserialize` when `serde` flag is present.

### `#[planning_solution]` / `#[planning_solution(serde, constraints = "path")]`

Applies to structs. Adds derives: `Clone, Debug, PlanningSolutionImpl`. Optionally adds serde derives. The `constraints = "path"` flag embeds a `#[solverforge_constraints_path = "path"]` attribute for the derive to consume.

### `#[problem_fact]` / `#[problem_fact(serde)]`

Applies to structs. Adds derives: `Clone, Debug, PartialEq, Eq, ProblemFactImpl`. Optionally adds serde derives.

## Derive Macros (proc_macro_derive)

### `PlanningEntityImpl`

**Consumed attributes on fields:**
- `#[planning_id]` — marks the unique ID field
- `#[planning_variable(allows_unassigned = bool, chained = bool, value_range_provider = "name")]` — genuine planning variable
  `value_range = "name"` is accepted as an alias for `value_range_provider`
- `#[planning_list_variable(...)]` — list planning variable
  stock solving currently requires `Vec<usize>` and `element_collection = "solution_field"`
- `#[planning_pin]` — boolean field controlling entity pinning
- `#[inverse_relation_shadow_variable(source_variable_name = "field")]` — inverse relation shadow
- `#[previous_element_shadow_variable(source_variable_name = "field")]` — previous element shadow
- `#[next_element_shadow_variable(source_variable_name = "field")]` — next element shadow
- `#[cascading_update_shadow_variable]` — cascading update shadow

**Generated code:**
- `impl PlanningEntity for T` — `is_pinned()`, `as_any()`, `as_any_mut()`
- `impl PlanningId for T` (if `#[planning_id]` present) — `type Id` set to field type, `planning_id()` returns field value
- `impl T { pub fn entity_descriptor(solution_field: &'static str) -> EntityDescriptor }` — builds descriptor with all variable descriptors (genuine, list, shadow) and preserves `#[planning_id]` / `#[planning_pin]` metadata
- Hidden list stock registry (when the entity has a `#[planning_list_variable]` field): `__SOLVERFORGE_LIST_VARIABLE_COUNT`, `__SOLVERFORGE_LIST_VARIABLE_NAME`, `__SOLVERFORGE_LIST_ELEMENT_COLLECTION`, `__solverforge_list_field()`, `__solverforge_list_field_mut()`, `__solverforge_list_stock_metadata()`
- Hidden typed stock bridge (when the entity has a `#[planning_list_variable]` field): `impl __internal::StockListEntity<Solution> for Entity`
- `pub trait {Entity}UnassignedFilter<...>` (when the entity has exactly one `Option<_>` planning variable) — `.unassigned()` on `UniConstraintStream<_, Entity, ...>`

### `PlanningSolutionImpl`

**Consumed attributes on fields:**
- `#[planning_entity_collection]` — `Vec<Entity>` field containing planning entities
- `#[problem_fact_collection]` — `Vec<Fact>` field containing problem facts
- `#[planning_score]` — `Option<Score>` field (required)
- `#[value_range_provider]` — value range source

**Consumed attributes on struct:**
- `#[shadow_variable_updates(...)]` — configures shadow variable update generation only
- `#[solverforge_constraints_path = "path"]` — path to constraint factory function

**`#[shadow_variable_updates]` parameters:**
- `list_owner = "field"` — selects the `#[planning_entity_collection]` field whose entity owns the list shadow updates
- `inverse_field = "field"` — field on element for inverse mapping
- `previous_field = "field"` — field on element for previous pointer
- `next_field = "field"` — field on element for next pointer
- `cascading_listener = "method"` — method name for cascading updates per element
- `post_update_listener = "method"` — method name called after all shadow updates per entity
- `entity_aggregate = "target:sum:source"` — aggregate element field onto entity (sum only)
- `entity_compute = "target:method"` — compute entity field via method

**`#[planning_list_variable]` stock parameters:**
- `element_collection = "field"` — solution field with all list elements
- `distance_meter = "path"` — optional cross-entity distance meter type
- `intra_distance_meter = "path"` — optional intra-entity distance meter type
- `merge_feasible_fn = "path"` — optional Clarke-Wright feasibility gate
- `cw_depot_fn`, `cw_distance_fn`, `cw_element_load_fn`, `cw_capacity_fn`, `cw_assign_route_fn` — Clarke-Wright hooks
- `k_opt_get_route`, `k_opt_set_route`, `k_opt_depot_fn`, `k_opt_distance_fn`, `k_opt_feasible_fn` — K-opt hooks

**Generated code:**
- `impl PlanningSolution for T` — `type Score`, `score()`, `set_score()`
- `impl T { pub fn descriptor() -> SolutionDescriptor }` — builds full descriptor with entity extractors and fact extractors, reusing entity-generated descriptors so field-level variable metadata is preserved
- `impl T { pub fn entity_count(&Self, descriptor_index: usize) -> usize }` — entity count by descriptor index
- List operations (when `list_owner` is configured): `list_len()`, `list_len_static()`, `list_remove()`, `list_insert()`, `list_get()`, `list_set()`, `list_reverse()`, `sublist_remove()`, `sublist_insert()`, `ruin_remove()`, `ruin_insert()`, `list_remove_for_construction()`, `index_to_element_static()`, `list_variable_descriptor_index()`, `element_count()`, `assigned_elements()`, `n_entities()`, `assign_element()`
- `impl ShadowVariableSupport for T` — `update_entity_shadows()` (no-op if no shadow config; generates inverse/previous/next/cascading/aggregate/compute updates otherwise)
- `impl SolvableSolution for T` — delegates to `descriptor()` and `entity_count()`
- `impl Solvable for T` (when constraints path specified) — `solve()` calls `solve_internal()`
- `impl Analyzable for T` (when constraints path specified) — `analyze()` creates `ScoreDirector` and returns `ScoreAnalysis`
- `fn solve_internal()` (when constraints path specified) — calls `run_stock_solver()` for macro-generated stock solving (standard-only and list/mixed stock paths); explicit low-level `ProblemSpec` use remains on `run_solver()`
- `pub trait {Name}ConstraintStreams<Sc>` — accessor methods for all `#[planning_entity_collection]` and `#[problem_fact_collection]` fields; implemented on `ConstraintFactory<{Name}, Sc>`

### `ProblemFactImpl`

**Consumed attributes on fields:**
- `#[planning_id]` — marks the unique ID field

**Generated code:**
- `impl ProblemFact for T` — `as_any()`
- `impl PlanningId for T` (if `#[planning_id]` present) — same as entity version
- `impl T { pub fn problem_fact_descriptor(solution_field: &'static str) -> ProblemFactDescriptor }`

## Shared Helper Functions (lib.rs, private)

| Function | Signature | Note |
|----------|-----------|------|
| `has_serde_flag` | `fn(TokenStream) -> bool` | Checks attribute stream for `serde` flag |
| `parse_solution_flags` | `fn(TokenStream) -> (bool, Option<String>)` | Parses `serde` and `constraints = "path"` |
| `has_attribute` | `fn(&[Attribute], &str) -> bool` | Checks if field has named attribute |
| `get_attribute` | `fn(&[Attribute], &str) -> Option<&Attribute>` | Gets named attribute |
| `parse_attribute_bool` | `fn(&Attribute, &str) -> Option<bool>` | Parses `key = true/false` from attribute |
| `parse_attribute_string` | `fn(&Attribute, &str) -> Option<String>` | Parses `key = "value"` from attribute |
| `parse_attribute_list` | `fn(&Attribute, &str) -> Vec<String>` | Collects all `key = "value"` pairs for same key |

## Internal Helper Functions (planning_solution.rs, private)

| Function | Signature | Note |
|----------|-----------|------|
| `parse_constraints_path` | `fn(&[Attribute]) -> Option<String>` | Extracts `#[solverforge_constraints_path = "..."]` |
| `parse_shadow_config` | `fn(&[Attribute]) -> ShadowConfig` | Parses `#[shadow_variable_updates(...)]` |
| `find_list_owner_config` | `fn(&ShadowConfig, &Fields) -> Result<Option<ListOwnerConfig>, Error>` | Resolves `list_owner` to the entity collection field and descriptor index |
| `vec_usize_fields` | `fn(&Fields) -> Vec<&Field>` | Collects `Vec<usize>` solution fields for list element dispatch |
| `shadow_updates_requested` | `fn(&ShadowConfig) -> bool` | Detects whether real shadow update work is configured |
| `generate_list_operations` | `fn(&ShadowConfig, &Fields, &Ident) -> Result<TokenStream, Error>` | Generates list variable methods from the entity-side stock registry |
| `generate_solvable_solution` | `fn(&Ident, &Option<String>) -> TokenStream` | Generates SolvableSolution/Solvable/Analyzable impls |
| `generate_shadow_support` | `fn(&ShadowConfig, &Ident) -> TokenStream` | Generates ShadowVariableSupport impl |
| `generate_constraint_stream_extensions` | `fn(&Fields, &Ident) -> TokenStream` | Generates `{Name}ConstraintStreams` trait + impl on ConstraintFactory |
| `extract_option_inner_type` | `fn(&Type) -> Result<&Type, Error>` | Extracts `T` from `Option<T>` |
| `extract_collection_inner_type` | `fn(&Type) -> Option<&Type>` | Extracts `T` from `Vec<T>` |

## Internal Config Structs (planning_solution.rs, private)

### `ShadowConfig`

```rust
struct ShadowConfig {
    list_owner: Option<String>,
    list_field: Option<String>,
    element_collection: Option<String>,
    inverse_field: Option<String>,
    previous_field: Option<String>,
    next_field: Option<String>,
    cascading_listener: Option<String>,
    post_update_listener: Option<String>,
    entity_aggregates: Vec<String>,   // "target:sum:source" format
    entity_computes: Vec<String>,     // "target:method" format
}
```

## Architectural Notes

### Code Generation Targets

All generated code references types via `::solverforge::__internal::*` paths, meaning the generated code depends on the `solverforge` facade crate re-exporting core types under `__internal`. Key referenced types:
- `PlanningEntity`, `PlanningSolution`, `PlanningId`, `ProblemFact`
- `EntityDescriptor`, `SolutionDescriptor`, `ProblemFactDescriptor`, `VariableDescriptor`
- `TypedEntityExtractor`
- `ShadowVariableKind`, `ShadowVariableSupport`, `SolvableSolution`
- `ScoreDirector`, `Director`

Trait impls like `Solvable`, `Analyzable`, and `ScoreAnalysis` reference `::solverforge::*` directly.

### Shadow Variable Update Order

When `#[shadow_variable_updates]` is configured, `update_entity_shadows(entity_idx)` executes in this order:
1. Collect `element_indices` from the configured list owner's list variable
2. Inverse field update (set element's inverse to entity_idx)
3. Previous element update (chain previous pointers)
4. Next element update (chain next pointers)
5. Cascading listener (call method per element)
6. Entity aggregates (sum element fields onto entity)
7. Entity computes (call method to compute entity fields)
8. Post-update listener (call method once per entity)

## Test Coverage

- `tests/trybuild.rs` — compile-pass and compile-fail coverage for the public macros
- Unit tests in `planning_entity.rs` and `planning_solution.rs` — token-level golden checks for generated code shape
