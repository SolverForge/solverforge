//! Clarke-Wright hooks that reject a merge breaking capacity or a time
//! window, so the savings routes can outnumber a fixed fleet.

use solverforge::cvrp::VrpSolution;

/// The depot of the route's vehicle.
pub fn depot<S: VrpSolution>(plan: &S, entity_idx: usize) -> usize {
    solverforge::cvrp::savings_depot_for_entity(plan, entity_idx)
}

/// The leg distance the savings are computed from.
pub fn distance<S: VrpSolution>(plan: &S, entity_idx: usize, from: usize, to: usize) -> i64 {
    solverforge::cvrp::savings_distance(plan, entity_idx, from, to)
}

/// Admits a merged route only when it keeps capacity and time windows.
pub fn feasible<S: VrpSolution>(plan: &S, entity_idx: usize, route: &[usize]) -> bool {
    solverforge::cvrp::route_feasible(plan, entity_idx, route)
}
