# Extend the Domain

Use the scaffold as a thin starter, then model the real problem in your app.

## What belongs in the domain

- Add entities, problem facts, and planning variables for the real data shape.
- Use field metadata to model scalar variables and one or more independent
  list owners in the same project when needed.
- Scalar `#[planning_variable]` fields are candidate indexes and must be
  `Option<usize>`. Keep external IDs on problem facts or entities, and map them
  through the value range provider collection.
- Keep `src/domain/mod.rs` as a `solverforge::planning_model!` manifest with
  `root = "src/domain"`, normal `mod name;` declarations, and the public
  exports for the model. Entity, fact, and solution files stay separate.
- Keep normal Rust module organization. SolverForge does not require entity
  modules to be declared before solution modules; scalar runtime metadata is
  generated from descriptor order and variable names, not expansion order. The
  compact scalar `variable_index` remains an internal getter/setter index.
- Public aliases are fine at the Rust boundary, including `type Alias = Type;`
  and `pub use module::Type as Alias;`. Solver configuration targets still use
  the canonical descriptor type name, not the alias used by a collection field.
- When a scalar variable will use nearby local-search selectors, declare the
  nearby candidate hook directly on that variable so the solver policy stays
  explicit and model-owned. Use `nearby_value_candidates` for nearby scalar
  change and `nearby_entity_candidates` for nearby scalar swap.
- Use `candidate_values` when construction, scalar change, pillar change, or
  scalar ruin-recreate should consume an ordered bounded value neighborhood
  instead of the full legal domain.
- When a scalar construction heuristic needs sorted entity or value order,
  declare that on the same `#[planning_variable]` with
  `construction_entity_order_key = "fn_name"` and/or
  `construction_value_order_key = "fn_name"`.
- Keep list construction capabilities on `#[planning_list_variable]`. Clarke-Wright
  and k-opt construction consume the same owner-aware route hooks:
  `route_get_fn`, `route_set_fn`, `route_depot_fn`, `route_distance_fn`, and
  `route_feasible_fn`. They do not infer scalar order keys or alternate list
  hooks.
- Add derived fields, validation helpers, and sample data beside the domain
  model, not in the scaffold templates.

## Choose the right hook

- Use `nearby_entity_distance_meter` or `nearby_value_distance_meter` only to
  rank or filter the bounded nearby candidates. A distance meter by itself does
  not bound candidate generation.
- Use `construction_entity_order_key` when construction must rank entities
  before generating placements.
- Use `construction_value_order_key` when construction must rank candidate
  values per entity, such as weakest-fit, strongest-fit, or queue-style
  allocation.
- Keep these separate. Nearby hooks guide local-search neighborhood shape;
  construction order keys guide construction-phase ordering and are not read by
  local-search scalar selectors.

## When the scaffold is no longer enough

- Create app-specific modules for larger domain logic.
- Move example constraints and sample fixtures into the app once they stop being
  representative of the starter project.
- Keep the generated scaffold thin so it stays a starter, not the source of
  truth.

## Practical path

1. Keep the scaffolded project.
2. Add your real entities, facts, and variable declarations.
3. Replace example data and example constraints with production domain logic.
4. Split large domain code into app modules as it grows.
