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

## Canonical selector defaults

If `move_selector` is omitted, the runtime keeps one streaming-first default
story:

- scalar-only models default to `ChangeMoveSelector`
- list-only models default to `NearbyListChangeMoveSelector(20)`,
  `NearbyListSwapMoveSelector(20)`, and `ListReverseMoveSelector`
- mixed models use the list defaults first, then scalar change

`selected_count_limit_move_selector` remains supported for compatibility, but it
is no longer the main way to avoid broad generation. The canonical defaults are
already explicit streaming neighborhoods.

## Practical path

1. Start from the scaffolded project.
2. Change the domain model or constraints that encode your business rules.
3. Adjust solver configuration for search strategy and termination.
4. Add custom solver code only when configuration is not enough.
