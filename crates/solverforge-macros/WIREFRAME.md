# solverforge-macros WIREFRAME

Proc-macro crate providing attribute macros and derive macros for SolverForge domain model structs.

**Location:** `crates/solverforge-macros/`
**Workspace Release:** `0.11.1`

## Dependencies

- `syn` (workspace) — Rust syntax parsing
- `quote` (workspace) — Rust code generation
- `proc-macro2` (workspace) — Proc-macro token streams

No solverforge crate dependencies. Generated code references `::solverforge::__internal::*` and `::solverforge::*` at the call site.

## File Map

```
src/
├── attr_parse.rs          — Shared attribute parsing helpers
├── entrypoints.rs          — Shared proc-macro wrapper logic used by the crate root
├── lib.rs                  — Crate root; required proc-macro entry points only
├── planning_model.rs       — `planning_model!` manifest parser, file reader, metadata validator, and model-support generator
├── planning_model/*.rs     — Manifest parsing, module loading, metadata validation, scalar-group support generation, shadow generation, and tests split by responsibility
├── planning_entity.rs      — planning-entity support derive module root
├── planning_entity/*.rs    — Entity derive expansion, scalar/list-variable helpers, and utilities
├── planning_entity/expand/*.rs — Entity derive expansion and validation helpers
├── planning_entity_tests.rs — PlanningEntity derive tests
├── planning_solution.rs    — planning-solution support derive module root
├── planning_solution/*.rs  — Solution derive expansion, config/shadow/runtime/list roots, stream extensions, and type helpers
├── planning_solution/list_operations/*.rs — Entity-collection list runtime helpers and public list-method generation
├── planning_solution/runtime/*.rs — Runtime module root, solve generation, scalar setup, and helper declarations
├── planning_solution/runtime/helpers/*.rs — Runtime helper code split by candidate counts, phase support, and generated trait impls
├── planning_solution_tests.rs — PlanningSolution derive tests
└── problem_fact.rs         — problem-fact support derive: ProblemFact, PlanningId, problem_fact_descriptor()
```

## Function-like Macros

### `planning_model!`

Canonical domain manifest for a SolverForge model. It is normally declared in
`src/domain/mod.rs`:

```rust
solverforge::planning_model! {
    root = "src/domain";

    mod item;
    mod container;
    mod plan;

    pub use item::Item;
    pub use container::Container;
    pub use plan::Plan;
}
```

The macro accepts `root = "..."`, file-backed `mod name;` declarations, and
public `pub use ...;` exports. Public `type Alias = Type;` declarations and
public `pub use module::Type as Alias;` exports are resolved to the canonical
descriptor type name during validation. It reads the listed module files, emits
the same Rust modules and exports plus `include_str!` dependencies for every
read module, enforces exactly one `#[planning_solution]`, validates entity and
fact collection types, validates list element collection references, and
generates the hidden `PlanningModelSupport` impl used by the solution derive for
scalar hooks, runtime scalar contexts, model validation, and list-shadow updates.

## Attribute Macros (proc_macro_attribute)

### `#[planning_entity]` / `#[planning_entity(serde)]`

Applies to structs. Adds ordinary Rust derives plus hidden SolverForge support derive output. Optionally adds `serde::Serialize, serde::Deserialize` when `serde` flag is present.

### `#[planning_solution]` / `#[planning_solution(serde, constraints = "path", config = "path")]`

Applies to structs. Adds ordinary Rust derives plus hidden SolverForge support derive output. Optionally adds serde derives. The `constraints = "path"` flag embeds a `#[solverforge_constraints_path = "path"]` attribute for the derive to consume. The `config = "path"` flag embeds a `#[solverforge_config_path = "path"]` attribute for the derive to consume; the callback must have signature `fn(&Solution, SolverConfig) -> SolverConfig` and decorates the loaded `solver.toml` config instead of replacing it.

### `#[problem_fact]` / `#[problem_fact(serde)]`

Applies to structs. Adds ordinary Rust derives plus hidden SolverForge support derive output. Optionally adds serde derives.

## Derive Macros (proc_macro_derive)

### Planning Entity Support Derive

**Consumed attributes on fields:**
- `#[planning_id]` — marks the unique ID field
- `#[planning_variable(allows_unassigned = bool, chained = bool, value_range_provider = "name")]` — genuine planning variable
  canonical scalar candidate and nearby hooks are declared here as well:
  `candidate_values = "fn_name"`, `nearby_value_candidates = "fn_name"`,
  `nearby_entity_candidates = "fn_name"`,
  `nearby_value_distance_meter = "fn_name"` and `nearby_entity_distance_meter = "fn_name"`
  scalar construction ordering hooks are declared here too:
  `construction_entity_order_key = "fn_name"` and
  `construction_value_order_key = "fn_name"`. These are emitted for
  construction routing and are not local-search selector ordering hooks.
- `#[planning_list_variable(...)]` — list planning variable
  currently requires `Vec<usize>` and `element_collection = "solution_field"`
- `#[planning_pin]` — boolean field controlling entity pinning
- `#[inverse_relation_shadow_variable(source_variable_name = "field")]` — inverse relation shadow
- `#[previous_element_shadow_variable(source_variable_name = "field")]` — previous element shadow
- `#[next_element_shadow_variable(source_variable_name = "field")]` — next element shadow
- `#[cascading_update_shadow_variable]` — cascading update shadow

**Generated code:**
- `impl PlanningEntity for T` — `is_pinned()`, `as_any()`, `as_any_mut()`
- `impl PlanningId for T` (if `#[planning_id]` present) — `type Id` set to field type, `planning_id()` returns field value
- `impl T { pub fn entity_descriptor(solution_field: &'static str) -> EntityDescriptor }` — builds descriptor with all variable descriptors (genuine, list, shadow) and preserves `#[planning_id]` / `#[planning_pin]` metadata
- Hidden scalar metadata bridge: private indexed helpers for scalar variable count, name, allows-unassigned, value-source metadata, getter/setter, and entity-local value slices. Helper order matches `entity_descriptor()` genuine scalar variable order; the index is used for generated getter/setter dispatch, while manifest hook attachment resolves descriptor variables by descriptor index plus variable name.
- Hidden list metadata bridge (when the entity has a `#[planning_list_variable]` field): public cross-module `__SOLVERFORGE_LIST_VARIABLE_COUNT` plus private `__SOLVERFORGE_LIST_VARIABLE_NAME`, `__SOLVERFORGE_LIST_ELEMENT_COLLECTION`, `__solverforge_list_field()`, `__solverforge_list_field_mut()`, `__solverforge_list_metadata()`
- Hidden list metadata bridge implementation (when the entity has a `#[planning_list_variable]` field): `impl __internal::ListVariableEntity<Solution> for Entity`
- Hidden unassigned bridge (when the entity has exactly one `Option<_>` planning variable): `impl __internal::UnassignedEntity<Solution> for Entity`, enabling `.unassigned()` on `UniConstraintStream<_, Entity, ...>` without a generated public trait import

### Planning Solution Support Derive

**Consumed attributes on fields:**
- `#[planning_entity_collection]` — `Vec<Entity>` field containing planning entities
- `#[planning_list_element_collection(owner = "field")]` — `Vec<usize>` field containing all elements for the named list owner; optional when the solution has a matching `#[planning_entity_collection]` or `#[problem_fact_collection]` field whose name matches the entity list variable's `element_collection`
- `#[problem_fact_collection]` — `Vec<Fact>` field containing problem facts
- `#[planning_score]` — `Option<Score>` field (required)
- `#[value_range_provider]` — value range source

**Consumed attributes on struct:**
- `#[shadow_variable_updates(...)]` — configures descriptor-aware shadow updates for the canonical solver path
- `#[solverforge_constraints_path = "path"]` — path to constraint factory function
- `#[solverforge_config_path = "path"]` — path to a config callback with signature `fn(&Solution, SolverConfig) -> SolverConfig`; called with the loaded `solver.toml` config (or defaults if the file is missing)

**`#[shadow_variable_updates]` parameters:**
- `list_owner = "field"` — selects the `#[planning_entity_collection]` field whose entity owns the list shadow updates
- `inverse_field = "field"` — field on element for inverse mapping
- `previous_field = "field"` — field on element for previous pointer
- `next_field = "field"` — field on element for next pointer
- `cascading_listener = "method"` — method name for cascading updates per element
- `post_update_listener = "method"` — method name called after all shadow updates per entity
- `entity_aggregate = "target:sum:source"` — aggregate element field onto entity (sum only)
- `entity_compute = "target:method"` — compute entity field via method

**`#[planning_list_variable]` parameters:**
- `element_collection = "field"` — solution field with all list elements
- `distance_meter = "path"` — optional cross-entity distance meter type
- `intra_distance_meter = "path"` — optional intra-entity distance meter type
- `merge_feasible_fn = "path"` — optional Clarke-Wright feasibility gate
- `cw_depot_fn`, `cw_distance_fn`, `cw_element_load_fn`, `cw_capacity_fn`, `cw_assign_route_fn` — Clarke-Wright hooks
- `k_opt_get_route`, `k_opt_set_route`, `k_opt_depot_fn`, `k_opt_distance_fn`, `k_opt_feasible_fn` — K-opt hooks

**Generated code:**
- `impl PlanningSolution for T` — `type Score`, `score()`, `set_score()`, plus `update_entity_shadows()` / `update_all_shadows()` overrides when shadow updates are configured
- `impl T { pub fn descriptor() -> SolutionDescriptor }` — builds full descriptor with entity extractors and fact extractors, reusing entity-generated descriptors so field-level variable order and metadata are preserved
- `impl T { pub fn entity_count(&Self, descriptor_index: usize) -> usize }` — entity count by descriptor index
- Private owner-specific list operations used by the canonical runtime: `__solverforge_list_len_<owner>()`, `__solverforge_list_remove_<owner>()`, `__solverforge_list_insert_<owner>()`, `__solverforge_list_get_<owner>()`, `__solverforge_list_set_<owner>()`, `__solverforge_list_reverse_<owner>()`, `__solverforge_sublist_remove_<owner>()`, `__solverforge_sublist_insert_<owner>()`, `__solverforge_ruin_remove_<owner>()`, `__solverforge_ruin_insert_<owner>()`, `__solverforge_list_remove_for_construction_<owner>()`, `__solverforge_index_to_element_<owner>()`, `__solverforge_element_count_<owner>()`, `__solverforge_assigned_elements_<owner>()`, `__solverforge_n_entities_<owner>()`, `__solverforge_assign_element_<owner>()`, plus aggregate helpers `__solverforge_total_list_entities()` and `__solverforge_total_list_elements()`
- Public owner-scoped list operations are generated for each `#[planning_entity_collection]` field: `{owner}_list_len()`, `{owner}_list_len_static()`, `{owner}_list_remove()`, `{owner}_list_insert()`, `{owner}_list_get()`, `{owner}_list_set()`, `{owner}_list_reverse()`, `{owner}_sublist_remove()`, `{owner}_sublist_insert()`, `{owner}_ruin_remove()`, `{owner}_ruin_insert()`, `{owner}_list_remove_for_construction()`, `{owner}_index_to_element_static()`, `{owner}_list_variable_descriptor_index()`, `{owner}_element_count()`, `{owner}_assigned_elements()`, `{owner}_n_entities()`, `{owner}_assign_element()`. Calls reject non-list owners at runtime with an explicit panic instead of relying on proc-macro name registries.
- Generic single-owner convenience methods assert at runtime that the solution has exactly one actual list owner before dispatching: `list_len()`, `list_len_static()`, `list_remove()`, `list_insert()`, `list_get()`, `list_set()`, `list_reverse()`, `sublist_remove()`, `sublist_insert()`, `ruin_remove()`, `ruin_insert()`, `list_remove_for_construction()`, `index_to_element_static()`, `list_variable_descriptor_index()`, `element_count()`, `assigned_elements()`, `n_entities()`, `assign_element()`
- `impl SolvableSolution for T` — delegates to `descriptor()` and `entity_count()`
- `impl Solvable for T` (when constraints path specified) — `solve(self, runtime: SolverRuntime<Self>)` delegates to `solve_internal()`
- `impl Analyzable for T` (when constraints path specified) — `analyze()` creates `ScoreDirector` with canonical shadow support and returns `ScoreAnalysis`
- `fn solve_internal(self, runtime: SolverRuntime<Self>)` (when constraints path specified) — calls `run_solver()` for macro-generated solving, or loads `solver.toml` and passes it through the configured `config = "..."` callback before calling `run_solver_with_config()`; generated runtime helpers build one `ModelContext` containing scalar contexts plus zero or more owner-specific list contexts, delegate scalar hook attachment to the `planning_model!` support impl, sort those variable contexts to the descriptor-backed variable order emitted by the macros, compute hidden shape-aware solve-start telemetry (`__solverforge_total_list_elements()` for list models and `__solverforge_scalar_candidate_count()` for scalar models), and then call hidden `build_phases(config, &descriptor, &model)`
- Public solution source methods for all `#[planning_entity_collection]`, `#[problem_fact_collection]`, and streamable `#[planning_list_element_collection]` fields. Each method is inherent on the solution type, for example `Plan::tasks()`, returns `impl solverforge::stream::CollectionExtract<Plan, Item = Task>`, and carries hidden `ChangeSource::Descriptor(idx)` for planning entities or `ChangeSource::Static` for facts and list elements. User constraints call `ConstraintFactory::new().for_each(Plan::tasks())`; there is still only one public stream-entry verb.

### Problem Fact Support Derive

**Consumed attributes on fields:**
- `#[planning_id]` — marks the unique ID field

**Generated code:**
- `impl ProblemFact for T` — `as_any()`
- `impl PlanningId for T` (if `#[planning_id]` present) — same as entity version
- `impl T { pub fn problem_fact_descriptor(solution_field: &'static str) -> ProblemFactDescriptor }`

## Shared Helper Functions (`entrypoints.rs` / `attr_parse.rs`, private)

| Function | Signature | Note |
|----------|-----------|------|
| `has_serde_flag` | `fn(TokenStream) -> bool` | Checks attribute stream for `serde` flag |
| `parse_solution_flags` | `fn(TokenStream) -> (bool, Option<String>, Option<String>)` | Parses `serde`, `constraints = "path"`, and `config = "path"` |
| `has_attribute` | `fn(&[Attribute], &str) -> bool` | Checks if field has named attribute |
| `get_attribute` | `fn(&[Attribute], &str) -> Option<&Attribute>` | Gets named attribute |
| `parse_attribute_bool` | `fn(&Attribute, &str) -> Option<bool>` | Parses `key = true/false` from attribute |
| `parse_attribute_string` | `fn(&Attribute, &str) -> Option<String>` | Parses `key = "value"` from attribute |
| `parse_attribute_list` | `fn(&Attribute, &str) -> Vec<String>` | Collects all `key = "value"` pairs for same key |

## Internal Helper Functions (`planning_solution/*.rs`, private)

| Function | Signature | Note |
|----------|-----------|------|
| `parse_constraints_path` | `fn(&[Attribute]) -> Option<String>` | Extracts `#[solverforge_constraints_path = "..."]` |
| `parse_config_path` | `fn(&[Attribute]) -> Option<String>` | Extracts `#[solverforge_config_path = "..."]` |
| `parse_shadow_config` | `fn(&[Attribute]) -> ShadowConfig` | Parses `#[shadow_variable_updates(...)]` |
| `generate_runtime_phase_support` | `fn(&Fields, &Option<String>, &Ident) -> TokenStream` | Generates the single macro-side runtime model assembly and phase builder glue |
| `find_list_owner_config` | `fn(&ShadowConfig, &Fields) -> Result<Option<ListOwnerConfig>, Error>` | Resolves shadow `list_owner` to the entity collection field and element-collection metadata |
| `find_list_shadow_config` | `fn(ListOwnerConfig, &Fields) -> Result<ListShadowConfig, Error>` | Resolves the configured owner to the matching direct element collection field for shadow validation |
| `shadow_updates_requested` | `fn(&ShadowConfig) -> bool` | Detects whether real shadow update work is configured |
| `generate_list_operations` | `fn(&Fields) -> TokenStream` | Generates the private runtime helper family, public owner-scoped list methods, and guarded single-owner generic methods without relying on bare-name metadata lookup |
| `generate_solvable_solution` | `fn(&Ident, &Option<String>) -> TokenStream` | Generates SolvableSolution/Solvable/Analyzable impls |
| `generate_shadow_support` | `fn(&ShadowConfig, &Fields, &Ident) -> Result<TokenStream, Error>` | Generates `PlanningSolution` shadow method overrides |
| `generate_collection_source_methods` | `fn(&Fields) -> TokenStream` | Generates inherent solution source methods used with ConstraintFactory::for_each |
| `extract_option_inner_type` | `fn(&Type) -> Result<&Type, Error>` | Extracts `T` from `Option<T>` |
| `extract_collection_inner_type` | `fn(&Type) -> Option<&Type>` | Extracts `T` from `Vec<T>` |

## Internal Config Structs (`planning_solution/config.rs`, private)

### `ShadowConfig`

```rust
struct ShadowConfig {
    list_owner: Option<String>,
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
- `EntityCollectionExtractor`
- `ShadowVariableKind`, `SolvableSolution`
- `ScoreDirector`, `Director`

Trait impls like `Solvable`, `Analyzable`, and `ScoreAnalysis` reference `::solverforge::*` directly.

### Proc-macro Crate Root Constraint

`lib.rs` intentionally retains the thin `#[proc_macro_attribute]` and `#[proc_macro_derive]` functions because Rust requires proc-macro exports to live at the crate root. All reusable parsing and code generation logic lives in helper modules.

### Scalar Runtime Metadata

Scalar runtime assembly is index-based and manifest-owned. `#[planning_entity]`
emits hidden per-entity scalar helpers in declaration order; that compact
`variable_index` is the generated getter/setter index. `planning_model!` reads
the declared modules and generates the `PlanningModelSupport` impl that attaches
descriptor hooks and runtime `ScalarVariableContext` hooks by descriptor index
plus variable name, then orders runtime variables from descriptor order. The
modeling syntax is unchanged, and Rust module declaration order is not a user
contract.

### Shadow Variable Update Order

When `#[shadow_variable_updates]` is configured, `update_entity_shadows(descriptor_index, entity_idx)` first ignores non-owner descriptors, then executes in this order for the configured owner:
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
- `planning_entity_tests.rs` and `planning_solution_tests.rs` — token-level golden checks for generated code shape
