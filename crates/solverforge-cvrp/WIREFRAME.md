# solverforge-cvrp WIREFRAME

Domain helpers for Capacitated Vehicle Routing Problems (CVRP).

**Location:** `crates/solverforge-cvrp/`
**Workspace Release:** `0.14.0`

## Dependencies

- `solverforge-solver` (path) — `CrossEntityDistanceMeter`

## File Map

```
src/
├── helpers.rs       — Public CVRP free functions and private pointer helpers
├── lib.rs           — Module exports and public re-exports
├── meters.rs        — Distance meter implementations
├── problem_data.rs  — `ProblemData`
└── solution.rs      — `VrpSolution`
```

## Types

### `ProblemData`

Immutable problem data shared by all vehicles. Fields:

| Field | Type | Description |
|-------|------|-------------|
| `capacity` | `i64` | Vehicle capacity |
| `depot` | `usize` | Depot node index |
| `demands` | `Vec<i32>` | Demand per node |
| `distance_matrix` | `Vec<Vec<i64>>` | Distance matrix |
| `time_windows` | `Vec<(i64, i64)>` | `(min_start, max_end)` per node |
| `service_durations` | `Vec<i64>` | Service duration per node |
| `travel_times` | `Vec<Vec<i64>>` | Travel time matrix |
| `vehicle_departure_time` | `i64` | Departure time from depot |

### `VrpSolution` (trait)

Trait implemented by a planning solution that holds a fleet of vehicles.

| Method | Signature |
|--------|-----------|
| `vehicle_data_ptr` | `fn(&self, entity_idx: usize) -> *const ProblemData` |
| `vehicle_visits` | `fn(&self, entity_idx: usize) -> &[usize]` |
| `vehicle_visits_mut` | `fn(&mut self, entity_idx: usize) -> &mut Vec<usize>` |
| `vehicle_count` | `fn(&self) -> usize` |

**Safety:** Implementors must ensure every `vehicle_data_ptr` points to a valid `ProblemData` for the entire duration of a solve call.

### `MatrixDistanceMeter`

Cross-entity distance meter backed by the solution's distance matrix. Implements `CrossEntityDistanceMeter<S: VrpSolution>`. `#[derive(Clone, Default)]`.

### `MatrixIntraDistanceMeter`

Intra-entity distance meter backed by the solution's distance matrix. Implements `CrossEntityDistanceMeter<S: VrpSolution>`. `#[derive(Clone, Default)]`.

## Free Functions

All functions are generic over `S: VrpSolution`.

| Function | Signature | Description |
|----------|-----------|-------------|
| `depot_for_entity` | `fn<S: VrpSolution>(plan: &S, entity_idx: usize) -> usize` | Depot index for the route owner |
| `route_distance` | `fn<S: VrpSolution>(plan: &S, entity_idx: usize, from: usize, to: usize) -> i64` | Distance between two element indices for the route owner |
| `route_feasible` | `fn<S: VrpSolution>(plan: &S, entity_idx: usize, route: &[usize]) -> bool` | True if the route satisfies the owner's capacity and time-window constraints |
| `replace_route` | `fn<S: VrpSolution>(plan: &mut S, entity_idx: usize, route: Vec<usize>)` | Replace the current route for an entity |
| `get_route` | `fn<S: VrpSolution>(plan: &S, entity_idx: usize) -> Vec<usize>` | Current route for an entity |

### Usage as macro attribute fn pointers

```rust
#[planning_list_variable(
    route_get_fn      = "solverforge_cvrp::get_route",
    route_set_fn      = "solverforge_cvrp::replace_route",
    route_depot_fn    = "solverforge_cvrp::depot_for_entity",
    route_distance_fn = "solverforge_cvrp::route_distance",
    route_feasible_fn = "solverforge_cvrp::route_feasible",
    // ...
)]
```
