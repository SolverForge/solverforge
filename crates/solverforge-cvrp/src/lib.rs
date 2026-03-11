//! CVRP domain helpers for SolverForge.
//!
//! Provides `ProblemData`, `MatrixDistanceMeter`, `MatrixIntraDistanceMeter`,
//! the `VrpSolution` trait, and a suite of free functions for Clarke-Wright
//! and k-opt construction phases.

use solverforge::CrossEntityDistanceMeter;

// ============================================================================
// Problem data
// ============================================================================

/// Immutable problem data shared by all vehicles.
///
/// Stored via raw pointer in each vehicle so the framework can clone vehicles
/// freely during local search without copying matrices.
pub struct ProblemData {
    pub capacity: i64,
    pub depot: usize,
    pub demands: Vec<i32>,
    pub distance_matrix: Vec<Vec<i64>>,
    pub time_windows: Vec<(i64, i64)>,
    pub service_durations: Vec<i64>,
    pub travel_times: Vec<Vec<i64>>,
    pub vehicle_departure_time: i64,
}

// ============================================================================
// VrpSolution trait
// ============================================================================

/// Trait implemented by a planning solution that holds a fleet of vehicles,
/// each carrying a `*const ProblemData` pointer and a list of visited stops.
///
/// # Safety
/// Implementors must ensure every `vehicle_data_ptr` points to a valid
/// `ProblemData` for the entire duration of a solve call.
pub trait VrpSolution {
    fn vehicle_data_ptr(&self, entity_idx: usize) -> *const ProblemData;
    fn vehicle_visits(&self, entity_idx: usize) -> &[usize];
    fn vehicle_visits_mut(&mut self, entity_idx: usize) -> &mut Vec<usize>;
    fn vehicle_count(&self) -> usize;
}

// ============================================================================
// Free functions (callable as fn-pointer fields in ListSpec)
// ============================================================================

/// Distance between two element indices using the first vehicle's data pointer.
pub fn distance<S: VrpSolution>(plan: &S, i: usize, j: usize) -> i64 {
    if plan.vehicle_count() == 0 {
        return 0;
    }
    // SAFETY: pointer is valid for the lifetime of solve (guaranteed by VrpSolution contract)
    let data = unsafe { &*plan.vehicle_data_ptr(0) };
    data.distance_matrix[i][j]
}

/// Returns the depot index (same for all vehicles).
pub fn depot_for_entity<S: VrpSolution>(plan: &S, _entity_idx: usize) -> usize {
    if plan.vehicle_count() == 0 {
        return 0;
    }
    // SAFETY: see distance()
    let data = unsafe { &*plan.vehicle_data_ptr(0) };
    data.depot
}

/// Returns the depot index for Clarke-Wright (plan-level, not per-entity).
pub fn depot_for_cw<S: VrpSolution>(plan: &S) -> usize {
    if plan.vehicle_count() == 0 {
        return 0;
    }
    // SAFETY: see distance()
    let data = unsafe { &*plan.vehicle_data_ptr(0) };
    data.depot
}

/// Returns the demand (load) for a single customer element.
pub fn element_load<S: VrpSolution>(plan: &S, elem: usize) -> i64 {
    if plan.vehicle_count() == 0 {
        return 0;
    }
    // SAFETY: see distance()
    let data = unsafe { &*plan.vehicle_data_ptr(0) };
    data.demands[elem] as i64
}

/// Returns the vehicle capacity.
pub fn capacity<S: VrpSolution>(plan: &S) -> i64 {
    if plan.vehicle_count() == 0 {
        return i64::MAX;
    }
    // SAFETY: see distance()
    let data = unsafe { &*plan.vehicle_data_ptr(0) };
    data.capacity
}

/// Assigns a constructed route to the given vehicle.
pub fn assign_route<S: VrpSolution>(plan: &mut S, entity_idx: usize, route: Vec<usize>) {
    *plan.vehicle_visits_mut(entity_idx) = route;
}

/// Returns the current route for entity `entity_idx`.
pub fn get_route<S: VrpSolution>(plan: &S, entity_idx: usize) -> Vec<usize> {
    plan.vehicle_visits(entity_idx).to_vec()
}

/// Replaces the current route for entity `entity_idx`.
pub fn set_route<S: VrpSolution>(plan: &mut S, entity_idx: usize, route: Vec<usize>) {
    *plan.vehicle_visits_mut(entity_idx) = route;
}

/// Returns `true` if the route satisfies all time-window constraints.
pub fn is_time_feasible<S: VrpSolution>(plan: &S, route: &[usize]) -> bool {
    if route.is_empty() || plan.vehicle_count() == 0 {
        return true;
    }
    // SAFETY: see distance()
    let data = unsafe { &*plan.vehicle_data_ptr(0) };
    check_time_feasible(route, data)
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

// ============================================================================
// Distance meters
// ============================================================================

/// Cross-entity distance meter backed by the solution's distance matrix.
#[derive(Clone, Default)]
pub struct MatrixDistanceMeter;

impl<S: VrpSolution> CrossEntityDistanceMeter<S> for MatrixDistanceMeter {
    fn distance(
        &self,
        solution: &S,
        src_entity: usize,
        src_pos: usize,
        dst_entity: usize,
        dst_pos: usize,
    ) -> f64 {
        let src_visits = solution.vehicle_visits(src_entity);
        let dst_visits = solution.vehicle_visits(dst_entity);
        if src_pos >= src_visits.len() || dst_pos >= dst_visits.len() {
            return f64::INFINITY;
        }
        // SAFETY: see distance()
        let data = unsafe { &*solution.vehicle_data_ptr(src_entity) };
        data.distance_matrix[src_visits[src_pos]][dst_visits[dst_pos]] as f64
    }
}

/// Intra-entity distance meter backed by the solution's distance matrix.
#[derive(Clone, Default)]
pub struct MatrixIntraDistanceMeter;

impl<S: VrpSolution> CrossEntityDistanceMeter<S> for MatrixIntraDistanceMeter {
    fn distance(
        &self,
        solution: &S,
        src_entity: usize,
        src_pos: usize,
        _dst_entity: usize,
        dst_pos: usize,
    ) -> f64 {
        let visits = solution.vehicle_visits(src_entity);
        if src_pos >= visits.len() || dst_pos >= visits.len() {
            return f64::INFINITY;
        }
        // SAFETY: see distance()
        let data = unsafe { &*solution.vehicle_data_ptr(src_entity) };
        data.distance_matrix[visits[src_pos]][visits[dst_pos]] as f64
    }
}
