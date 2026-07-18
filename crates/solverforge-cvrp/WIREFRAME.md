# solverforge-cvrp WIREFRAME

Domain helpers for Capacitated Vehicle Routing Problems (CVRP), including the
stock `domain = "cvrp"` list-variable profile internals.

**Location:** `crates/solverforge-cvrp/`
**Workspace Release:** `0.19.2`

## Dependencies

- `solverforge-solver` (path) — `CrossEntityDistanceMeter`

## File Map

```
src/
├── helpers.rs       — Public CVRP free functions, hook bundles, and private pointer helpers
├── lib.rs           — Module exports and public re-exports
├── meters.rs        — Distance meter implementations
├── problem_data.rs  — `ProblemData`
├── solution.rs      — `VrpSolution`
└── tests.rs         — CVRP helper and route-feasibility tests
```

## Types

### `ProblemData`

Immutable problem data shared by all vehicles. Derives: `Clone, Debug`.

Fields:

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

`UNREACHABLE` is exported as `i64::MAX` for non-traversable matrix entries.
`route_feasible` rejects routes that contain unreachable travel-time legs or
overflowing time arithmetic. Distance hooks convert unreachable or malformed
distance entries into a large finite cost so construction and local-search
distance arithmetic stays panic-free.

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

Cross-entity distance meter backed by the solution's distance matrix. Implements `CrossEntityDistanceMeter<S: VrpSolution>`. `#[derive(Clone, Debug, Default)]`.

### `MatrixIntraDistanceMeter`

Intra-entity distance meter backed by the solution's distance matrix. Implements `CrossEntityDistanceMeter<S: VrpSolution>`. `#[derive(Clone, Debug, Default)]`.

## Free Functions

All functions are generic over `S: VrpSolution`.

| Function | Signature | Description |
|----------|-----------|-------------|
| `depot_for_entity` | `fn<S: VrpSolution>(plan: &S, entity_idx: usize) -> usize` | Depot index for the route owner |
| `route_distance` | `fn<S: VrpSolution>(plan: &S, entity_idx: usize, from: usize, to: usize) -> i64` | Distance between two element indices for the route owner |
| `route_feasible` | `fn<S: VrpSolution>(plan: &S, entity_idx: usize, route: &[usize]) -> bool` | Strict route-local CVRP feasibility: structural validity, capacity, and time windows |
| `replace_route` | `fn<S: VrpSolution>(plan: &mut S, entity_idx: usize, route: Vec<usize>)` | Replace the current route for an entity |
| `get_route` | `fn<S: VrpSolution>(plan: &S, entity_idx: usize) -> Vec<usize>` | Current route for an entity |
| `savings_depot_for_entity` | `fn<S: VrpSolution>(plan: &S, entity_idx: usize) -> usize` | Construction depot adapter for models that share exact CVRP route data |
| `savings_metric_class` | `fn<S: VrpSolution>(plan: &S, entity_idx: usize) -> usize` | Clarke-Wright metric class for owners that share backing `ProblemData` |
| `savings_distance` | `fn<S: VrpSolution>(plan: &S, entity_idx: usize, from: usize, to: usize) -> i64` | Construction distance adapter for models that share exact CVRP route data |
| `savings_feasible` | `fn<S: VrpSolution>(plan: &S, entity_idx: usize, route: &[usize]) -> bool` | Construction admissibility adapter that rejects only non-evaluable stock CVRP routes |

### Usage as a macro domain profile

```rust
#[planning_list_variable(
    element_collection = "deliveries",
    domain = "cvrp"
)]
```

The profile expands to the stock meters, `VrpSolution` bound, route hooks,
savings hooks, and savings metric class. Route-local phases use strict stock
CVRP route feasibility. Clarke-Wright construction uses relaxed savings
feasibility so assignment can stay broad while the score model compares
capacity, lateness, and unreachable-leg violations against leaving work
unassigned.

### Advanced macro hook bundles

Public hook modules:

- `route_hooks` — exports `get`, `set`, `depot`, `distance`, and `feasible`
- `savings_hooks` — exports `depot`, `distance`, and `feasible`

`route_hooks` exports `get`, `set`, `depot`, `distance`, and `feasible` for
route-local behavior, including strict CVRP capacity and time-window checks.
`savings_hooks` exports `depot`, `distance`, and `feasible` for Clarke-Wright
construction when that construction metric shares stock CVRP data; its
feasibility hook rejects malformed owners/data/visit ids but admits scoreable
capacity and time-window violations. Custom hook modules are the advanced
escape hatch for non-CVRP route semantics or different pruning policies; use
them by omitting `domain = "cvrp"` and declaring explicit macro hook paths.
