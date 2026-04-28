# solverforge-cvrp WIREFRAME

Domain helpers for Capacitated Vehicle Routing Problems (CVRP).

**Location:** `crates/solverforge-cvrp/`
**Workspace Release:** `0.9.2`

## Dependencies

- `solverforge` (workspace) — Facade; re-exports `CrossEntityDistanceMeter`

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
| `distance` | `fn<S: VrpSolution>(plan: &S, i: usize, j: usize) -> i64` | Distance between two element indices via the first vehicle's data pointer |
| `depot_for_entity` | `fn<S: VrpSolution>(plan: &S, _entity_idx: usize) -> usize` | Depot index (same for all vehicles) |
| `depot_for_cw` | `fn<S: VrpSolution>(plan: &S) -> usize` | Depot index for Clarke-Wright (plan-level) |
| `element_load` | `fn<S: VrpSolution>(plan: &S, elem: usize) -> i64` | Demand for a single customer element |
| `capacity` | `fn<S: VrpSolution>(plan: &S) -> i64` | Vehicle capacity |
| `replace_route` | `fn<S: VrpSolution>(plan: &mut S, entity_idx: usize, route: Vec<usize>)` | Replace the current route for an entity |
| `get_route` | `fn<S: VrpSolution>(plan: &S, entity_idx: usize) -> Vec<usize>` | Current route for an entity |
| `is_time_feasible` | `fn<S: VrpSolution>(plan: &S, route: &[usize]) -> bool` | True if route satisfies all time-window constraints |
| `is_kopt_feasible` | `fn<S: VrpSolution>(plan: &S, _entity_idx: usize, route: &[usize]) -> bool` | K-opt feasibility gate; `entity_idx` ignored, delegates to `is_time_feasible` |

### Usage as macro attribute fn pointers

```rust
#[planning_list_variable(
    merge_feasible_fn = "solverforge_cvrp_lib::is_time_feasible",
    cw_depot_fn       = "solverforge_cvrp_lib::depot_for_cw",
    cw_distance_fn    = "solverforge_cvrp_lib::distance",
    cw_element_load_fn= "solverforge_cvrp_lib::element_load",
    cw_capacity_fn    = "solverforge_cvrp_lib::capacity",
    cw_assign_route_fn= "solverforge_cvrp_lib::replace_route",
    k_opt_get_route   = "solverforge_cvrp_lib::get_route",
    k_opt_set_route   = "solverforge_cvrp_lib::replace_route",
    k_opt_depot_fn    = "solverforge_cvrp_lib::depot_for_entity",
    k_opt_distance_fn = "solverforge_cvrp_lib::distance",
    k_opt_feasible_fn = "solverforge_cvrp_lib::is_kopt_feasible",
    // ...
)]
```
