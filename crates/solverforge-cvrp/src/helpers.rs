use crate::{ProblemData, VrpSolution};

#[inline]
pub(crate) fn problem_data_for_entity<S: VrpSolution>(
    plan: &S,
    entity_idx: usize,
) -> Option<&ProblemData> {
    let ptr = plan.vehicle_data_ptr(entity_idx);
    assert!(
        !ptr.is_null(),
        "VrpSolution::vehicle_data_ptr({entity_idx}) returned null for a non-empty fleet"
    );
    // SAFETY: VrpSolution implementors guarantee valid pointers for the duration
    // of the solve call; null for a non-empty fleet is rejected above.
    unsafe { ptr.as_ref() }
}

#[inline]
fn first_problem_data<S: VrpSolution>(plan: &S) -> Option<&ProblemData> {
    if plan.vehicle_count() == 0 {
        return None;
    }
    problem_data_for_entity(plan, 0)
}

/// Distance between two element indices using the first vehicle's data pointer.
pub fn distance<S: VrpSolution>(plan: &S, i: usize, j: usize) -> i64 {
    first_problem_data(plan).map_or(0, |data| data.distance_matrix[i][j])
}

pub fn depot_for_entity<S: VrpSolution>(plan: &S, _entity_idx: usize) -> usize {
    first_problem_data(plan).map_or(0, |data| data.depot)
}

pub fn depot_for_cw<S: VrpSolution>(plan: &S) -> usize {
    first_problem_data(plan).map_or(0, |data| data.depot)
}

pub fn element_load<S: VrpSolution>(plan: &S, elem: usize) -> i64 {
    first_problem_data(plan).map_or(0, |data| data.demands[elem] as i64)
}

pub fn capacity<S: VrpSolution>(plan: &S) -> i64 {
    first_problem_data(plan).map_or(i64::MAX, |data| data.capacity)
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

/// Returns `true` if the route satisfies all time-window constraints.
pub fn is_time_feasible<S: VrpSolution>(plan: &S, route: &[usize]) -> bool {
    if route.is_empty() {
        return true;
    }
    match first_problem_data(plan) {
        Some(data) => check_time_feasible(route, data),
        None => true,
    }
}

/// K-opt feasibility gate: returns `true` if the route satisfies all time-window constraints.
/// The `entity_idx` parameter is ignored — time windows are uniform across vehicles.
pub fn is_kopt_feasible<S: VrpSolution>(plan: &S, _entity_idx: usize, route: &[usize]) -> bool {
    is_time_feasible(plan, route)
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
