# solverforge-solver

`solverforge-solver` contains SolverForge's runtime phase assembly, move
implementations, selector builders, construction heuristics, local search, and
retained solver lifecycle.

## Grouped Scalar Reachability

Normal scalar construction is single-slot: each nullable scalar variable is
assigned independently through its descriptor/runtime scalar binding. That
behavior remains the default whenever a construction heuristic has no
`group_name`.

Use grouped scalar construction when a model has nullable scalar variables that
must change together before a hard-feasible state is reachable. A model declares
named `ScalarGroup` entries, and a construction heuristic opts in with
`group_name`. Candidate-backed groups evaluate a whole `ScalarCandidate` as one
compound scalar move, apply all legal edits atomically, and mark every touched
scalar slot complete through the normal committed mutation path.

Assignment-backed groups are declared with `ScalarGroup::assignment(...)` over
one nullable scalar target. They generate stock grouped construction candidates
for required and optional scalar assignments, use the same grouped selection
engine as candidate-backed groups, and support assignment-aware local-search
moves for required slots, capacity conflicts, reassignments, and bounded
sequence/position rematches.

`GroupedScalarMoveSelector` exposes the same declared groups during local search.
It is a first-class scalar neighborhood, not cartesian-product composition.

## Compound Conflict Repair

`CompoundConflictRepairMoveSelector` is the stock conflict-aware repair
primitive. Domain providers still supply candidate edit hints, but the framework
owns selector limits, duplicate filtering, legality checks, not-doable filtering,
hard-improvement filtering, scoring, tabu identity, and affected-entity
reporting through `CompoundScalarMove`.

Configured constraint keys resolve against scoring metadata by exact identity:
package-qualified constraints use `ConstraintRef::full_name()` strings, while
package-less constraints use the short name.

This keeps app-specific logic as domain candidate generation only. Applications
should not add seed repair, relaxed hard constraints, fake variables, or private
solver orchestration to make coupled scalar states reachable.
