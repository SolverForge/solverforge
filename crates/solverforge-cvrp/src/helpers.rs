use crate::{ProblemData, VrpSolution};

#[inline]
pub(crate) fn problem_data_for_entity<S: VrpSolution>(
    plan: &S,
    entity_idx: usize,
) -> Option<&ProblemData> {
    if entity_idx >= plan.vehicle_count() {
        return None;
    }
    let ptr = plan.vehicle_data_ptr(entity_idx);
    assert!(
        !ptr.is_null(),
        "VrpSolution::vehicle_data_ptr({entity_idx}) returned null for a non-empty fleet"
    );
    // SAFETY: VrpSolution implementors guarantee valid pointers for the duration
    // of the solve call; null for a non-empty fleet is rejected above.
    unsafe { ptr.as_ref() }
}

pub fn depot_for_entity<S: VrpSolution>(plan: &S, entity_idx: usize) -> usize {
    problem_data_for_entity(plan, entity_idx).map_or(0, |data| data.depot)
}

/// Construction metric class for the route owner.
///
/// Owners that share the same `ProblemData` pointer share depot and distance
/// behavior, so Clarke-Wright can compute their savings rows once.
pub fn savings_metric_class<S: VrpSolution>(plan: &S, entity_idx: usize) -> usize {
    if entity_idx >= plan.vehicle_count() {
        return entity_idx;
    }

    let ptr = plan.vehicle_data_ptr(entity_idx);
    assert!(
        !ptr.is_null(),
        "VrpSolution::vehicle_data_ptr({entity_idx}) returned null for a non-empty fleet"
    );
    ptr as usize
}

/// Depot token used by Clarke-Wright construction.
pub fn savings_depot_for_entity<S: VrpSolution>(plan: &S, entity_idx: usize) -> usize {
    depot_for_entity(plan, entity_idx)
}

/// Construction distance used by Clarke-Wright for models that share CVRP route data.
pub fn savings_distance<S: VrpSolution>(
    plan: &S,
    entity_idx: usize,
    from: usize,
    to: usize,
) -> i64 {
    route_distance(plan, entity_idx, from, to)
}

/// Construction feasibility used by Clarke-Wright for models that share CVRP route data.
pub fn savings_feasible<S: VrpSolution>(plan: &S, entity_idx: usize, route: &[usize]) -> bool {
    route_feasible(plan, entity_idx, route)
}

/// Distance between two element indices for the route owner.
pub fn route_distance<S: VrpSolution>(plan: &S, entity_idx: usize, from: usize, to: usize) -> i64 {
    problem_data_for_entity(plan, entity_idx).map_or(0, |data| data.distance_matrix[from][to])
}

/// Replaces the current route for entity `entity_idx`.
///
/// Callers must pass a valid `entity_idx` for the current solution.
pub fn replace_route<S: VrpSolution>(plan: &mut S, entity_idx: usize, route: Vec<usize>) {
    *plan.vehicle_visits_mut(entity_idx) = route;
}

/// Returns a cloned snapshot of the route for entity `entity_idx`.
///
/// Callers must pass a valid `entity_idx` for the current solution.
pub fn get_route<S: VrpSolution>(plan: &S, entity_idx: usize) -> Vec<usize> {
    plan.vehicle_visits(entity_idx).to_vec()
}

/// Returns `true` if the route satisfies capacity and time-window constraints.
pub fn route_feasible<S: VrpSolution>(plan: &S, entity_idx: usize, route: &[usize]) -> bool {
    if route.is_empty() {
        return true;
    }
    match problem_data_for_entity(plan, entity_idx) {
        Some(data) => check_capacity_feasible(route, data) && check_time_feasible(route, data),
        None => true,
    }
}

/// Route-local hook bundle for `#[planning_list_variable(route_hooks = "...")]`.
pub mod route_hooks {
    pub use super::depot_for_entity as depot;
    pub use super::get_route as get;
    pub use super::replace_route as set;
    pub use super::route_distance as distance;
    pub use super::route_feasible as feasible;
}

/// Clarke-Wright savings hook bundle for `#[planning_list_variable(savings_hooks = "...")]`.
///
/// Use this only when construction should share the same CVRP data as exact
/// route-local behavior.
pub mod savings_hooks {
    pub use super::savings_depot_for_entity as depot;
    pub use super::savings_distance as distance;
    pub use super::savings_feasible as feasible;
}

fn check_capacity_feasible(route: &[usize], data: &ProblemData) -> bool {
    route
        .iter()
        .map(|&visit| data.demands[visit] as i64)
        .sum::<i64>()
        <= data.capacity
}

fn check_time_feasible(route: &[usize], data: &ProblemData) -> bool {
    let mut current_time = data.vehicle_departure_time;
    let mut prev = data.depot;

    for &visit in route {
        current_time += data.travel_times[prev][visit];

        let (min_start, max_end) = data.time_windows[visit];

        if current_time < min_start {
            current_time = min_start;
        }

        let service_end = current_time + data.service_durations[visit];

        if service_end > max_end {
            return false;
        }

        current_time = service_end;
        prev = visit;
    }

    true
}
