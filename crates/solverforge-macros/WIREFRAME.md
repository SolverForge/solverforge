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
├── problem_fact.rs       — ProblemFactImpl derive: PlanningId for problem facts
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
- `#[planning_list_variable]` — list planning variable
- `#[planning_pin]` — boolean field controlling entity pinning
- `#[inverse_relation_shadow_variable(source_variable_name = "field")]` — inverse relation shadow
- `#[previous_element_shadow_variable(source_variable_name = "field")]` — previous element shadow
- `#[next_element_shadow_variable(source_variable_name = "field")]` — next element shadow
- `#[cascading_update_shadow_variable]` — cascading update shadow

**Generated code:**
- `impl PlanningEntity for T` — `is_pinned()`, `as_any()`, `as_any_mut()`
- `impl PlanningId for T` (if `#[planning_id]` present) — `type Id` set to field type, `planning_id()` returns field value
- `impl T { pub fn entity_descriptor(solution_field: &'static str) -> EntityDescriptor }` — builds descriptor with all variable descriptors (genuine, list, shadow)

### `PlanningSolutionImpl`

**Consumed attributes on fields:**
- `#[planning_entity_collection]` — `Vec<Entity>` field containing planning entities
- `#[problem_fact_collection]` — `Vec<Fact>` field containing problem facts
- `#[planning_score]` — `Option<Score>` field (required)
- `#[value_range_provider]` — value range source

**Consumed attributes on struct:**
- `#[shadow_variable_updates(...)]` — configures shadow variable update generation
- `#[basic_variable_config(...)]` — configures basic variable operations
- `#[solverforge_constraints_path = "path"]` — path to constraint factory function

**`#[shadow_variable_updates]` parameters:**
- `list_owner = "field"` — entity collection field containing list owners
- `list_field = "field"` — field on entity containing the list (Vec)
- `element_collection = "field"` — solution field with all elements
- `element_type = "Type"` — element type name
- `inverse_field = "field"` — field on element for inverse mapping
- `previous_field = "field"` — field on element for previous pointer
- `next_field = "field"` — field on element for next pointer
- `cascading_listener = "method"` — method name for cascading updates per element
- `post_update_listener = "method"` — method name called after all shadow updates per entity
- `entity_aggregate = "target:sum:source"` — aggregate element field onto entity (sum only)
- `entity_compute = "target:method"` — compute entity field via method

**`#[basic_variable_config]` parameters:**
- `entity_collection = "field"` — entity collection field name
- `variable_field = "field"` — planning variable field name on entity
- `variable_type = "Type"` — variable type name
- `value_range = "field"` — value range source field name

**Generated code:**
- `impl PlanningSolution for T` — `type Score`, `score()`, `set_score()`
- `impl T { pub fn descriptor() -> SolutionDescriptor }` — builds full descriptor with entity extractors and fact extractors
- `impl T { pub fn entity_count(&Self, descriptor_index: usize) -> usize }` — entity count by descriptor index
- List operations (when shadow_variable_updates configured): `list_len()`, `list_remove()`, `list_insert()`, `sublist_remove()`, `sublist_insert()`, `list_variable_descriptor_index()`, `element_count()`, `assigned_elements()`, `n_entities()`, `assign_element()`
- Basic variable operations (when basic_variable_config configured): `basic_get_variable()`, `basic_set_variable()`, `basic_value_count()`, `basic_entity_count()`, `basic_variable_descriptor_index()`, `basic_variable_field_name()`, `finalize_all()`
- `impl ShadowVariableSupport for T` — `update_entity_shadows()` (no-op if no shadow config; generates inverse/previous/next/cascading/aggregate/compute updates otherwise)
- `impl SolvableSolution for T` (when any variable config present) — delegates to `descriptor()` and `entity_count()`
- `impl Solvable for T` (when constraints path specified) — `solve()` calls `solve_internal()`
- `impl Analyzable for T` (when constraints path specified) — `analyze()` creates `TypedScoreDirector` and returns `ScoreAnalysis`
- `fn solve_internal()` (when constraints path specified) — calls `run_solver()`

### `ProblemFactImpl`

**Consumed attributes on fields:**
- `#[planning_id]` — marks the unique ID field

**Generated code:**
- `impl PlanningId for T` (if `#[planning_id]` present) — same as entity version

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
| `parse_basic_variable_config` | `fn(&[Attribute]) -> BasicVariableConfig` | Parses `#[basic_variable_config(...)]` |
| `generate_list_operations` | `fn(&ShadowConfig, &Fields) -> TokenStream` | Generates list variable methods |
| `generate_basic_variable_operations` | `fn(&BasicVariableConfig, &Fields, &Option<String>, &Ident) -> TokenStream` | Generates basic variable methods |
| `generate_solvable_solution` | `fn(&ShadowConfig, &BasicVariableConfig, &Ident, &Option<String>) -> TokenStream` | Generates SolvableSolution/Solvable/Analyzable impls |
| `generate_shadow_support` | `fn(&ShadowConfig, &Ident) -> TokenStream` | Generates ShadowVariableSupport impl |
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
    element_type: Option<String>,
    entity_aggregates: Vec<String>,   // "target:sum:source" format
    entity_computes: Vec<String>,     // "target:method" format
}
```

### `BasicVariableConfig`

```rust
struct BasicVariableConfig {
    entity_collection: Option<String>,
    variable_field: Option<String>,
    variable_type: Option<String>,
    value_range: Option<String>,
}
```

## Architectural Notes

### Code Generation Targets

All generated code references types via `::solverforge::__internal::*` paths, meaning the generated code depends on the `solverforge` facade crate re-exporting core types under `__internal`. Key referenced types:
- `PlanningEntity`, `PlanningSolution`, `PlanningId`, `ProblemFact`
- `EntityDescriptor`, `SolutionDescriptor`, `ProblemFactDescriptor`, `VariableDescriptor`
- `TypedEntityExtractor`
- `ShadowVariableKind`, `ShadowVariableSupport`, `SolvableSolution`
- `TypedScoreDirector`, `ScoreDirector`

Trait impls like `Solvable`, `Analyzable`, and `ScoreAnalysis` reference `::solverforge::*` directly.

### Shadow Variable Update Order

When `#[shadow_variable_updates]` is configured, `update_entity_shadows(entity_idx)` executes in this order:
1. Collect `element_indices` from `entity.list_field.clone()`
2. Inverse field update (set element's inverse to entity_idx)
3. Previous element update (chain previous pointers)
4. Next element update (chain next pointers)
5. Cascading listener (call method per element)
6. Entity aggregates (sum element fields onto entity)
7. Entity computes (call method to compute entity fields)
8. Post-update listener (call method once per entity)

### No Tests in Crate

This is a proc-macro crate. Tests are integration-level, run via the consuming crates (examples and the facade).
