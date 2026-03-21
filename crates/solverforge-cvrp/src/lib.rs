/* CVRP domain helpers for SolverForge.

Provides `ProblemData`, `MatrixDistanceMeter`, `MatrixIntraDistanceMeter`,
the `VrpSolution` trait, and a suite of free functions for Clarke-Wright
and k-opt construction phases.
*/

use solverforge_solver::CrossEntityDistanceMeter;

/* ============================================================================
Problem data
============================================================================
*/

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

/* ============================================================================
VrpSolution trait
============================================================================
*/

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

/* ============================================================================
Free functions (callable as fn-pointer fields in ListSpec)
============================================================================
*/

/// Distance between two element indices using the first vehicle's data pointer.
pub fn distance<S: VrpSolution>(plan: &S, i: usize, j: usize) -> i64 {
    if plan.vehicle_count() == 0 {
        return 0;
    }
    // SAFETY: pointer is valid for the lifetime of solve (guaranteed by VrpSolution contract)
    let data = unsafe { &*plan.vehicle_data_ptr(0) };
    data.distance_matrix[i][j]
}

pub fn depot_for_entity<S: VrpSolution>(plan: &S, _entity_idx: usize) -> usize {
    if plan.vehicle_count() == 0 {
        return 0;
    }
    // SAFETY: see distance()
    let data = unsafe { &*plan.vehicle_data_ptr(0) };
    data.depot
}

pub fn depot_for_cw<S: VrpSolution>(plan: &S) -> usize {
    if plan.vehicle_count() == 0 {
        return 0;
    }
    // SAFETY: see distance()
    let data = unsafe { &*plan.vehicle_data_ptr(0) };
    data.depot
}

pub fn element_load<S: VrpSolution>(plan: &S, elem: usize) -> i64 {
    if plan.vehicle_count() == 0 {
        return 0;
    }
    // SAFETY: see distance()
    let data = unsafe { &*plan.vehicle_data_ptr(0) };
    data.demands[elem] as i64
}

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

/* ============================================================================
Distance meters
============================================================================
*/

// Cross-entity distance meter backed by the solution's distance matrix.
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

// Intra-entity distance meter backed by the solution's distance matrix.
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

#[cfg(test)]
mod tests {
    use super::*;

    struct TestSolution {
        data: Box<ProblemData>,
        routes: Vec<Vec<usize>>,
    }

    impl TestSolution {
        fn new(routes: Vec<Vec<usize>>) -> Self {
            Self {
                data: Box::new(ProblemData {
                    capacity: 10,
                    depot: 0,
                    demands: vec![0, 2, 3, 4],
                    distance_matrix: vec![
                        vec![0, 5, 7, 9],
                        vec![5, 0, 4, 6],
                        vec![7, 4, 0, 3],
                        vec![9, 6, 3, 0],
                    ],
                    time_windows: vec![(0, 100), (0, 10), (7, 14), (0, 12)],
                    service_durations: vec![0, 2, 2, 3],
                    travel_times: vec![
                        vec![0, 5, 7, 9],
                        vec![5, 0, 4, 6],
                        vec![7, 4, 0, 3],
                        vec![9, 6, 3, 0],
                    ],
                    vehicle_departure_time: 0,
                }),
                routes,
            }
        }
    }

    impl VrpSolution for TestSolution {
        fn vehicle_data_ptr(&self, _entity_idx: usize) -> *const ProblemData {
            self.data.as_ref() as *const ProblemData
        }

        fn vehicle_visits(&self, entity_idx: usize) -> &[usize] {
            &self.routes[entity_idx]
        }

        fn vehicle_visits_mut(&mut self, entity_idx: usize) -> &mut Vec<usize> {
            &mut self.routes[entity_idx]
        }

        fn vehicle_count(&self) -> usize {
            self.routes.len()
        }
    }

    #[test]
    fn helpers_use_problem_data_from_first_vehicle() {
        let solution = TestSolution::new(vec![vec![1, 2], vec![3]]);

        assert_eq!(distance(&solution, 1, 3), 6);
        assert_eq!(depot_for_entity(&solution, 1), 0);
        assert_eq!(depot_for_cw(&solution), 0);
        assert_eq!(element_load(&solution, 2), 3);
        assert_eq!(capacity(&solution), 10);
    }

    #[test]
    fn helpers_handle_empty_fleets() {
        let solution = TestSolution::new(vec![]);

        assert_eq!(distance(&solution, 1, 2), 0);
        assert_eq!(depot_for_entity(&solution, 0), 0);
        assert_eq!(depot_for_cw(&solution), 0);
        assert_eq!(element_load(&solution, 1), 0);
        assert_eq!(capacity(&solution), i64::MAX);
        assert!(is_time_feasible(&solution, &[1, 2]));
        assert!(is_kopt_feasible(&solution, 0, &[1, 2]));
    }

    #[test]
    fn route_helpers_replace_and_clone_routes() {
        let mut solution = TestSolution::new(vec![vec![1, 2], vec![3]]);

        assign_route(&mut solution, 0, vec![2, 3]);
        assert_eq!(solution.routes[0], vec![2, 3]);
        assert_eq!(get_route(&solution, 0), vec![2, 3]);

        set_route(&mut solution, 1, vec![1]);
        assert_eq!(solution.routes[1], vec![1]);
    }

    #[test]
    fn time_feasibility_checks_waiting_and_deadlines() {
        let solution = TestSolution::new(vec![vec![1, 2], vec![3]]);

        assert!(
            is_time_feasible(&solution, &[1, 2]),
            "route should wait for customer 2 and still finish in time"
        );
        assert!(
            !is_time_feasible(&solution, &[2, 3]),
            "route should miss customer 3's latest end"
        );
        assert!(is_kopt_feasible(&solution, 1, &[1, 2]));
    }

    #[test]
    fn distance_meters_cover_invalid_positions() {
        let solution = TestSolution::new(vec![vec![1, 2], vec![3]]);

        assert_eq!(MatrixDistanceMeter.distance(&solution, 0, 0, 1, 0), 6.0);
        assert_eq!(
            MatrixIntraDistanceMeter.distance(&solution, 0, 0, 0, 1),
            4.0
        );
        assert!(MatrixDistanceMeter
            .distance(&solution, 0, 4, 1, 0)
            .is_infinite());
        assert!(MatrixIntraDistanceMeter
            .distance(&solution, 0, 0, 0, 4)
            .is_infinite());
    }
}
