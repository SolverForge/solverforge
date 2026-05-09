# Extend the Solver

Use the generated project as the starting point, then grow solver behavior in the
application that depends on `solverforge`.

## When to change solver behavior

- Add or refine constraints when the default business rules are incomplete.
- Tune `solver.toml` when the search strategy, termination, or runtime settings
  need to change.
- Use `#[planning_solution(config = "...")]` when a retained solve needs a
  per-solution override; the callback should decorate the loaded `solver.toml`
  config rather than replace it.
- Introduce custom phases, selectors, or acceptors when the built-in search flow
  is not a fit for the problem shape.

## What to keep in mind

- Keep domain modeling in Rust and solver policy in configuration separate;
  use per-solution config callbacks only to layer runtime-specific adjustments on
  top of `solver.toml`.
- Preserve structured solver events and telemetry so the UI and service layer
  remain accurate. Generated moves, evaluated moves, accepted moves, and
  generation/evaluation `Duration`s now flow through retained telemetry exactly;
  only human-facing rates are derived at the edge.
- Prefer small, app-specific extensions over forking the scaffold templates.

## Construction routing

Construction heuristics now route by validated model capability rather than
ad hoc special cases:

- `first_fit` and `cheapest_insertion` stay generic for mixed or list-bearing
  targets, but scalar-only construction targets use the descriptor boundary.
- `first_fit_decreasing`, `weakest_fit*`, `strongest_fit*`,
  `allocate_entity_from_queue`, and `allocate_to_value_from_queue` are scalar-only.
  The targeted scalar variable must declare the required
  `construction_entity_order_key` and/or `construction_value_order_key`.
  Generated model assembly provides the same hooks at runtime; the scalar route
  resolves one binding set from descriptor metadata plus runtime scalar-variable
  hooks by descriptor index and variable name, then uses that
  same resolved set for validation and execution. The compact scalar
  `variable_index` remains the generated getter/setter dispatch index. Those
  construction hooks are not selector-order hints for local search; local-search
  scalar selectors keep canonical bounded candidate order.
- `list_round_robin`, `list_cheapest_insertion`, `list_regret_insertion`,
  `list_clarke_wright`, and `list_k_opt` are list-only. The runtime validates
  the required list hooks before phase build instead of failing deep inside the
  algorithm.
- `group_name` selects a named `ScalarGroup` for grouped scalar construction.
  Candidate-backed groups apply arbitrary compound scalar candidates atomically.
  Assignment-backed groups generate stock nullable scalar candidates and feed
  the same grouped construction heuristics: required entities first, optional
  entities only when score-improving, and capacity blockers repaired through
  bounded augmenting paths. Decreasing and strength-based assignment heuristics
  require the corresponding `with_entity_order` and `with_value_order` hooks.

## Canonical selector defaults

If `move_selector` is omitted, the runtime keeps one streaming-first default
story:

- scalar-only models default to `ChangeMoveSelector` plus `SwapMoveSelector`
- list-only models default to `NearbyListChangeMoveSelector(20)`,
  `NearbyListSwapMoveSelector(20)`, `SublistChangeMoveSelector`,
  `SublistSwapMoveSelector`, and `ListReverseMoveSelector`, with k-opt and
  list ruin enabled only when their hooks exist
- mixed models use the list defaults first, then the scalar defaults

Omitted config builds construction plus one streaming local-search phase. Broad
stock unions use fair ordering and finite accepted-count horizons; explicit
`limited_neighborhood` remains the user-facing cap when a configured selector
would otherwise be exhaustive. VND is still available, but only when the local
search phase explicitly selects `variable_neighborhood_descent`.

Nearby scalar selectors are explicit model capabilities. If the search policy
uses `nearby_change_move_selector` or `nearby_swap_move_selector`, the matching
scalar variable must provide the corresponding nearby candidate hook; distance
meters may rank or filter those bounded candidates, but the runtime does not use
them as candidate-discovery hooks.

Canonical local search uses the monomorphized variable plan published by macro
runtime assembly. The descriptor boundary remains explicit for scalar-only
construction and callers that intentionally assemble descriptor selectors; it is
not a fallback path for normal local search.

`limited_neighborhood` applies a fixed move cap to one configured neighborhood
when that cap is part of the search policy. The canonical defaults are already
explicit streaming neighborhoods, so use the cap deliberately instead of
wrapping broad generation by default.

Scalar value generation is separate from scalar legality. `value_candidate_limit`
caps candidate values for scalar construction, scalar change, pillar change, and
scalar ruin-recreate. Nearby scalar selectors require `nearby_*_candidates`;
distance meters only rank or filter those bounded candidates. Scalar
`cheapest_insertion` is accepted only when a bounded candidate source is present.

Cartesian neighborhoods compose selectors sequentially. The left child is
previewed first, the right child is opened against that preview state, and the
runtime only materializes the selected winning composite move by ownership after
forager choice. Left children that require full score evaluation during preview
are rejected up front.

## Practical path

1. Start from the scaffolded project.
2. Change the domain model or constraints that encode your business rules.
3. Adjust solver configuration for search strategy and termination.
4. Add custom solver code only when configuration is not enough.
