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
`group_name`. The grouped route evaluates a whole `ScalarCandidate` as one
compound scalar move, applies all legal edits atomically, and marks every
touched scalar slot complete through the normal committed mutation path.

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
