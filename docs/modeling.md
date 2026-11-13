# Modeling

A SolverForge model is plain Rust: problem facts that never change, planning
entities whose variables the solver assigns, and one planning solution that
holds both plus the score. This guide covers how those pieces are declared and
what each kind of planning variable stores. For hook-by-hook guidance inside a
generated app, see [Extend the domain](extend-domain.md).

## The model manifest

Model metadata belongs to the model, not to helper imports scattered around the
app. `src/domain/mod.rs` is a `solverforge::planning_model!` manifest that lists
ordinary Rust modules; entity, fact and solution types stay in their own files.

The attribute macros on those types generate the rest:

| Macro | Goes on | Purpose |
|-------|---------|---------|
| `planning_model!` | `src/domain/mod.rs` | Model manifest: lists the domain modules and their exports |
| `#[planning_solution]` | the solution struct | Collections, score, constraints, config hooks |
| `#[planning_entity]` | each entity struct | Planning variables, pins, shadows |
| `#[problem_fact]` | each fact struct | Read-only input data |
| `#[solverforge_constraints]` | the constraint function | Whole-function constraint compiler, see [Constraints](constraints.md) |

`#[planning_solution]` generates a collection source function per collection,
such as `Schedule::shifts()`, which constraints consume through
`ConstraintFactory::for_each(...)`. It also generates a matching
extension trait, ScheduleConstraintStreams for a `Schedule` solution; with that
trait imported you can write the shorthand `ConstraintFactory::new().shifts()`.

The [`minimal-shift-scheduling`](../examples/minimal-shift-scheduling/) example
is the smallest complete model in the repository. Read its files in this order:

1. [`src/domain/mod.rs`](../examples/minimal-shift-scheduling/src/domain/mod.rs)
   declares the model with `solverforge::planning_model!`.
2. [`src/domain/nurse.rs`](../examples/minimal-shift-scheduling/src/domain/nurse.rs)
   defines the problem facts.
3. [`src/domain/shift.rs`](../examples/minimal-shift-scheduling/src/domain/shift.rs)
   defines the planning entities and the `nurse_idx` planning variable.
4. [`src/domain/schedule.rs`](../examples/minimal-shift-scheduling/src/domain/schedule.rs)
   owns the solution, the constraints, the `solver.toml` link and the
   assignment-backed `ScalarGroup` hooks.
5. [`solver.toml`](../examples/minimal-shift-scheduling/solver.toml) configures
   required-shift assignment construction and grouped scalar local search.
6. [`src/main.rs`](../examples/minimal-shift-scheduling/src/main.rs) builds a
   small schedule, starts `SolverManager`, consumes solver events and prints
   the completed assignment.

## Scalar variables

A `#[planning_variable]` field holds one value per entity. It stores a
candidate index as `Option<usize>`; keep external IDs on facts or entities and
map them through the value range collection.

- `value_range_provider` defines the legal values. Point it at a solution
  collection for one shared domain, or at a `Vec<usize>` field on the entity
  for a per-entity domain.
- `candidate_values` only orders and bounds the values that construction and
  change-style moves try. It does not restrict swaps. Forbidden values belong
  in the legal domain.
- `allows_unassigned = true` makes the variable nullable.

Assignment-backed `ScalarGroup` declarations cover the common "assign each row
to one resource, some rows are mandatory" shape. They are described in
[Extend the domain](extend-domain.md#assignment-backed-scalar-groups).

## List variables

Use a list variable whenever the solver decides assignment and order together:
routes, sequences, machine queues. The owning entity stores element indexes
directly, so the route has one canonical representation instead of a scalar
predecessor graph.

```rust
#[planning_entity]
pub struct Route {
    #[planning_id]
    pub id: usize,

    #[planning_list_variable(element_collection = "visits")]
    pub visit_order: Vec<usize>,
}

#[planning_solution]
pub struct Plan {
    #[problem_fact_collection]
    pub visits: Vec<Visit>,

    #[planning_entity_collection]
    pub routes: Vec<Route>,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}
```

List construction assigns unassigned elements to owners. List change, swap,
sublist, reversal, k-opt, ruin/recreate and nearby selectors then work on that
same representation and keep exact undo.

Inverse, index, previous, next, cascading, custom and piggyback shadows are
derived views. Configure them only when constraints or application logic read
them. Generated index maintenance writes an `Option<usize>` field on each
element and is enabled on the planning solution with
`#[shadow_variable_updates(list_owner = "routes", index_field = "index")]`.

A model can mix scalar variables with any number of independent list owners.

### Fixed element owners

A list variable can pin elements to owners with
`#[planning_list_variable(element_collection = "...", element_owner_fn = "...")]`.
The hook has the signature `fn(&Solution, usize) -> Option<usize>`:

- `Some(owner)` is the only owner entity index allowed for that element;
- `None` leaves the element unrestricted;
- an out-of-range owner index is invalid. It is never widened to
  unrestricted.

When the model is assembled through `planning_model!`, the hook must be visible
from the manifest root module. Construction, Clarke-Wright construction,
ruin/recreate and owner-changing list neighborhoods honor the hook. Moves that
only reorder within one owner stay available.

### Source identity in list construction

Cheapest insertion, regret insertion and Clarke-Wright key every declared
element, every currently assigned element and every precedence-successor value
by one unique, stable `usize` source identity. With `planning_model!` that
identity is the element index the list variable already stores, so there is
nothing to write.

SolverForge freezes that binding before construction and then uses source
indexes for ordering, ownership, precedence, candidate traces and static or
dynamic execution. It never recovers identity through payload equality or
hashing. Duplicate, missing or inconsistent keys are a construction bind error.
When several construction phases target the same list slot, they share the
frozen declaration binding but refresh current assignments before each phase,
so a later phase cannot reinsert earlier work or reread declaration callbacks.

## Pinning

Add `#[planning_pin]` to a boolean entity field to keep that entity's input
planning variables during solving. A pinned route owner keeps its list contents
and order; a pinned scalar keeps its input value.

A pinned required assignment that starts unassigned cannot be completed, so the
solve fails. An optional pinned scalar may stay unassigned.

## Vehicle routing (CVRP profile)

`#[planning_list_variable(domain = "cvrp")]` wires the stock CVRP distance
meters and two separate hook bundles:

- route hooks, used by route-local phases such as `ListKOpt`, enforce strict
  capacity, time-window and unreachable-leg feasibility;
- savings hooks, used by Clarke-Wright construction, only reject routes that
  stock CVRP data cannot evaluate safely. Capacity and time-window violations
  stay scoreable, so assignment remains score-comparable.

The `solverforge-cvrp` crate holds `VrpSolution`, `ProblemData`, the distance
meters and those hook bundles. `ProblemData` reads leg distances and travel
times from its `metric`, named by `VrpSolution::Metric`:

- `DenseMatrix::new(distances, travel_times)` holds supplied `[from][to]`
  tables, such as road-network matrices; memory grows with the square of the
  location count.
- `EuclideanMetric::new(coordinates, speed)` computes straight-line legs on
  demand (TSPLIB `EUC_2D`: `sqrt(dx² + dy²)` rounded half away from zero,
  travel time = length / speed, rounded the same way); memory
  grows linearly, for models too large for a matrix.

`UNREACHABLE` or a negative leg value marks a leg that cannot be traversed.

For models past roughly 10k stops, pair `EuclideanMetric` with bounded
Clarke-Wright savings (`savings_neighbors`, see
[Configuration](configuration.md#bounded-clarke-wright-savings)): the profile's
neighbour hook asks the metric for each stop's nearest stops, which
`EuclideanMetric` answers from a spatial grid, so construction memory stays
linear in the stops. Custom routing domains omit `domain = "cvrp"`
and supply their own `route_hooks` and `savings_hooks`; see
[Extend the domain](extend-domain.md).

## Host-language bindings

`solverforge-bridge` holds the contracts that host-language bindings (such as
the Python bindings) build on: logical entity, fact and variable IDs, dynamic
score support, descriptor-resolved scalar and list slots, lazy nearby-source
adapters, and explicit dynamic-list access and metadata. Bindings depend on it
directly; Rust applications do not need it.
