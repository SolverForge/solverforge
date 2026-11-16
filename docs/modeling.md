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

### Cascading shadows across lists

A cascading shadow is an element value computed from the element's previous
element in its list and from elements it waits on, which other owners' lists
may hold: a start time with precedence across machines, employees or
vehicles. Declare the field, the per-element compute and the dependencies,
and SolverForge owns the ordering, propagation and change reporting:

```rust
#[planning_solution]
#[shadow_variable_updates(
    list_owner = "machines",
    inverse_field = "machine",
    previous_field = "previous",
    next_field = "next",
    cascading_field = "start",
    cascading_compute = "compute_start",
    cascading_dependencies = "task_dependencies"
)]
pub struct Shop {
    #[planning_entity_collection]
    pub machines: Vec<Machine>,

    #[problem_fact_collection]
    pub tasks: Vec<Task>,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}

impl Shop {
    fn end(&self, task: usize) -> Option<i64> {
        let task = &self.tasks[task];
        task.start.map(|start| start + task.duration)
    }

    // After the previous task on the machine and the task it waits on.
    pub fn compute_start(&self, task: usize) -> Option<i64> {
        let mut start = 0;
        if let Some(previous) = self.tasks[task].previous {
            start = start.max(self.end(previous)?);
        }
        if let Some(dependency) = self.tasks[task].waits_on {
            start = start.max(self.end(dependency)?);
        }
        Some(start)
    }

    pub fn task_dependencies(&self, task: usize) -> Option<usize> {
        self.tasks[task].waits_on
    }
}
```

- `cascading_field` names an `Option<_>` element field; its value type must
  implement `PartialEq`.
- `cascading_compute` names a solution method `fn(&self, element: usize) ->
  Option<_>`. It runs only for an element held by a list, after the previous
  element in that list and every dependency hold their final values for the
  move.
- `cascading_dependencies` (optional) names a solution method returning the
  elements `element` waits on, as any `IntoIterator<Item = usize>` (an
  `Option<usize>`, a slice, a `Vec`). Dependencies are problem facts; moves
  never change them, and a problem change that does is applied through a
  reset.
- `cascading_peers` (optional) names a solution method returning the
  elements computed together with `element`, as any
  `IntoIterator<Item = usize>`. See [Joint groups](#joint-groups).
- `inverse_field`, `previous_field` and `next_field` are required.
  `cascading_listener`, `post_update_listener`, `entity_aggregate` and
  `entity_compute` cannot be combined with a cascading shadow: they run per
  changed owner and would not see the values a move changes in other
  owners' lists.

After each move the score director recomputes the downstream cone of the
changed elements in dependency order: each element once, after its inputs,
and only when an input changed, so propagation stops where a value is
unchanged. Every element whose value changes is rescored exactly once. An
element in no list, or on a dependency cycle a move creates (a task that
waits on a task queued after it), is unset (`None`), which constraints can
penalize. Models without a cascading shadow compile none of this into their
move path.

#### Joint groups

Some elements in different lists must be timed together: a job that needs
two technicians starts when the later of them arrives, so each technician's
visit waits on the other's arrival. Declaring that as two dependencies would
be a cycle. Declare the visits peers instead:

```rust
#[planning_solution]
#[shadow_variable_updates(
    list_owner = "technicians",
    inverse_field = "technician",
    previous_field = "previous",
    next_field = "next",
    cascading_field = "start",
    cascading_compute = "compute_start",
    cascading_peers = "visit_partners"
)]
pub struct Crew {
    #[planning_entity_collection]
    pub technicians: Vec<Technician>,

    #[problem_fact_collection]
    pub visits: Vec<Visit>,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}

impl Crew {
    // When the technician can be at the visit.
    fn arrival(&self, visit: usize) -> Option<i64> {
        match self.visits[visit].previous {
            Some(previous) => {
                let left = self.visits[previous].start? + self.visits[previous].duration;
                Some(left + self.visits[visit].travel_from[previous])
            }
            None => Some(0),
        }
    }

    // When the last assigned technician of the job arrives.
    pub fn compute_start(&self, visit: usize) -> Option<i64> {
        let mut start = self.arrival(visit)?;
        for &partner in &self.visits[visit].partners {
            if self.visits[partner].technician.is_some() {
                start = start.max(self.arrival(partner)?);
            }
        }
        Some(start)
    }

    pub fn visit_partners(&self, visit: usize) -> Vec<usize> {
        self.visits[visit].partners.clone()
    }
}
```

Peers are joined transitively into a joint group, and propagation treats
the group as one node: it waits until the previous element and every
dependency of every member hold their final values, then computes the
members once each, in ascending element order. A member's compute may read
those inputs of its peers; it must not read a peer's cascading value, which
is being computed in the same step. Peers are problem facts, like
dependencies.

A group whose members wait on each other through their lists (two
members on one route, or a member after an element that waits on its peer)
is a cycle, and its members are unset. A model without peers maps every
element to its own node and pays nothing for the feature.

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

### Element groups

Some elements must travel together: the pick-up and drop-off of one job, or
every stop of a multi-stop job. Declare them with
`#[planning_list_variable(element_collection = "...", element_group_fn = "...")]`.
The hook has the signature `fn(&Solution, usize, &mut Vec<usize>)` and appends
the members of the element's group, in order and including the element, or
nothing for an element without a group:

```rust
/// A vehicle that drives to its stops in list order.
#[planning_entity]
pub struct Vehicle {
    /// Stable vehicle id.
    #[planning_id]
    pub id: usize,

    /// The stops the vehicle makes, in order.
    #[planning_list_variable(element_collection = "stops", element_group_fn = "job_stops")]
    pub stops: Vec<usize>,
}

/// A job's pick-up and drop-off, in that order; a single stop has no group.
pub(super) fn job_stops(plan: &super::Deliveries, stop: usize, members: &mut Vec<usize>) {
    if let Some(job) = plan.stops[stop].job {
        members.extend([2 * job, 2 * job + 1]);
    }
}
```

Only groups of two or more members count, and every member must report the
same group; a conflicting answer panics. Groups are read once per step and
once per construction phase.

A group's members share one owner and keep their declared order. Other
elements may sit between them, as a shared ride needs:

- `list_group_change_move_selector` moves a whole group to positions of one
  owner, its own included; `max_member_distance` caps the positions from
  one member to the next.
- `list_group_swap_move_selector` exchanges two groups of the same size on
  different owners, member slot for member slot.
- `list_ruin_move_selector` ruins whole groups and recreates each group
  jointly, choosing all of its members' positions together.
- `list_cheapest_insertion` and `list_regret_insertion` place a group as one
  unit. A member is never placed alone; members already assigned pin the rest
  of their group to that owner.
- The single-element list moves keep a grouped element on its owner, so they
  can still reorder it within its route but never split a group.

A group moves only to an owner that every member allows under
`element_owner_fn` and that is not pinned. Joint placement of a group of `m`
members tries O(L^m) position tuples per owner; `max_member_distance` and
`max_group_owners` (an owner shortlist, by best contiguous fit) bound it. See
[Configuration](configuration.md#element-groups) for the defaults.

`element_group_fn` cannot be combined with precedence hooks, and
`list_round_robin`, `list_clarke_wright` and the global `first_fit` and
`cheapest_insertion` construction reject a list with groups, since they place
elements one at a time. A list without the hook compiles none of this.

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
