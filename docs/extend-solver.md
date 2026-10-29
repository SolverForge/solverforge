# Extend the Solver

Use the generated project as the starting point, then grow solver behavior in the
application that depends on `solverforge`.

## When to change solver behavior

- Add or refine constraints when the default business rules are incomplete.
- Tune `solver.toml` when the search strategy, termination, or runtime settings
  need to change.
- Use `#[planning_solution(config = "...")]` when a retained solve needs a
  per-solution override; the callback should decorate the base config (the
  embedded `solver_toml` file, or the defaults) rather than replace it.
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
- Keep configured solver and phase limits binding during construction. The
  compiled runtime withholds best-solution publication until every list element,
  required assignment row, and non-optional scalar row is assigned. Reaching a
  limit first uses the existing `Failed` lifecycle, and a pause before structural
  completion remains resumable without exposing a partial solution snapshot.
- Prefer small, app-specific extensions over forking the scaffold templates.

## Committed writers and model callbacks

Every committed change to the working solution — accepted construction and
local-search moves, configuration-list commits, partition merges, and direct
apply closures — flows through one committed-mutation path that clears the
cached current score and advances the working-solution revision exactly once.
Speculative evaluation never advances the revision: it applies the move, scores,
undoes with the typed undo value, and restores the snapshotted committed score
state, leaving no residue.

Model callbacks (`value_candidates`, `construction_order_key`,
`search_order_key` hooks, `value_is_legal`, ownership policies) may read the
whole solution. That dependency is unrestricted by default: never rely on a
callback's signature to infer row-locality, because a committed change anywhere
can change any callback's output. Construction enumeration and placement
therefore re-read committed global state per pull instead of trusting a cached
snapshot. A narrow-impact declaration is not inferred from signatures; until a
callback explicitly declares a narrow impact, assume it depends on all of `S`.

Working-solution replacement is the wholesale writer:
`replace_working_solution_and_reinitialize(solution)` resets the score director,
forces recalculation, returns the revision to its initial value, and invalidates
descriptor-driven construction completion markers.

Sequential composite moves check doability on the caller's live director with
`is_doable_with_working_preview`: the left leg is applied on a snapshotted score
state, the right leg is checked against that previewed state, and the left leg
is undone and the score state restored before the check returns. It never
clones the solution for a mutable preview. Compiled Cartesian discovery uses the
same live-director transaction for right-child opening and every right pull.
Resource-aware cursor implementations receive `&mut D` rather than a solution
snapshot; they must restore all trial writes and score state before returning a
candidate. Ordinary `MoveCursor` extensions continue to receive `&S`; the public
non-compiled Cartesian selector still uses its owned preview director.

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
  construction hooks are not selector-order hints for local search;
  local-search ranking uses the dedicated `search_entity_order_key` /
  `search_value_order_key` hooks, and a variable that declares neither keeps
  the canonical seeded-random candidate order.
- `list_round_robin`, `list_cheapest_insertion`, `list_regret_insertion`,
  `list_clarke_wright`, and `list_k_opt` are list-only. The runtime validates
  the required route or savings hooks before phase build instead of failing deep
  inside the algorithm.
- `group_name` selects a named `ScalarGroup` for grouped scalar construction.
  Candidate-backed groups apply arbitrary compound scalar candidates atomically.
  Assignment-backed groups generate stock nullable scalar candidates and feed
  the same grouped construction heuristics: required entities first, optional
  entities only when score-improving, and capacity blockers repaired through
  bounded augmenting paths. Decreasing and strength-based assignment heuristics
  require the corresponding `with_entity_order` and `with_value_order` hooks.
  Keeping an optional assignment unassigned completes that decision, not the
  optional pass. Completed optional roots are excluded before candidate-domain
  reads and move generation, so they do not consume later assignments' budget.
  Assigned occupants remain available to required augmenting rematches.

## Canonical selector defaults

If `move_selector` is omitted, the runtime keeps one streaming-first default
story:

- non-assignment-owned scalar slots receive nearby change/swap selectors when
  their matching model hooks are declared, followed by plain change/swap
  selectors; assignment-owned slots remain on their named grouped-scalar path
- list slots with the complete precedence bundle receive precedence repair plus
  list permutation; slots with cross-position distance receive nearby change
  and, when set access exists, nearby swap with a limit of 20, while slots
  without that metric receive the corresponding plain change/swap families
- sublist change/swap, reversal, and k-opt are added only when their required
  list operations are declared; k-opt uses the nearby form when intra-position
  distance is available and the unbounded form otherwise; every bound list slot
  receives list ruin
- mixed models preserve the compiled policy order: list families, nearby scalar
  families, grouped scalar, compound conflict repair, then ordinary scalar
  families. Leaf order is seeded `Random`; one family is `Sequential`, while a
  multi-family union is `StratifiedRandom`

Omitted phases run state-aware construction. One streaming default local-search
phase is eligible only when top-level termination has an effective parsed limit.
Absent termination, an empty termination object, or invalid score-only
termination stays construction-only. Broad
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

## Exact typed phases

Exact tree search is available as a typed extension point, not as a runtime
fallback built from generated scalar slots. If a model needs exact search, add a
custom phase that owns its concrete decider and register it through the typed
search function. Assignment-backed scalar variables still belong to their
`ScalarGroup`; exact scalar assignment should not be configured as a second
path beside grouped assignment.

## Partitioned search

Use `partitioned_search` only when the application can provide a typed
`SolutionPartitioner` that splits and merges independent subproblems. SolverForge
does not infer partitions from a count and does not claim transparent parallel
solving for globally coupled models.

Partitioned search is a poor fit when hard constraints couple entities across
partition boundaries, when merge cost dominates child solving, or when child
solutions need global coordination to stay feasible. Typed search registration
must name the partitioner explicitly; unregistered config fails at build time.

Partition children are internal compute scopes, not retained-job publication
owners. They inherit cancellation, environment mode, remaining time limit,
in-phase limits, and deterministic seeds, but pause checkpoints are emitted only
by the parent scope while it still owns a full public solution. If a child yields
to pause, cancel, configured termination, or a queued real-time problem change,
partitioned search abandons the partial child results instead of merging them.
Children never apply problem changes; the configured runtime applies them to the
unmerged solution and restarts its phase sequence.

## Real-time problem changes

`SolverManager::add_problem_change` queues a `ProblemChange<S>` (or a one-shot
`ClosureProblemChange`) on a running or paused job. Built-in phases stop at
their next control poll; custom phases must poll `solver_scope.should_terminate()`
in their step loop the same way they already must for pause and cancel.
The runtime then applies the batch to the best solution, rescores from scratch,
and restarts the phase sequence, calling `CustomSearchPhase::on_problem_change`
first so a custom phase can drop state sized or keyed by the previous solution.
A change mutates `score_director.working_solution_mut()` directly and must keep
fact indexes and planning-variable references valid. Run real-time jobs without
a solver-level time limit and end them with `cancel()`: configured limits stay
binding during re-construction.

## Practical path

1. Start from the scaffolded project.
2. Change the domain model or constraints that encode your business rules.
3. Adjust solver configuration for search strategy and termination.
4. Add custom solver code only when configuration is not enough.
