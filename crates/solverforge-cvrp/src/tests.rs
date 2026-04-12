use super::*;
use solverforge_solver::CrossEntityDistanceMeter;

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

struct NullDataSolution {
    routes: Vec<Vec<usize>>,
}

impl VrpSolution for NullDataSolution {
    fn vehicle_data_ptr(&self, _entity_idx: usize) -> *const ProblemData {
        std::ptr::null()
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
#[should_panic(expected = "vehicle_data_ptr(0) returned null")]
fn helpers_reject_missing_problem_data_for_non_empty_fleets() {
    let solution = NullDataSolution {
        routes: vec![vec![1, 2]],
    };

    let _ = distance(&solution, 1, 2);
}

#[test]
fn route_helpers_replace_and_clone_routes() {
    let mut solution = TestSolution::new(vec![vec![1, 2], vec![3]]);

    replace_route(&mut solution, 0, vec![2, 3]);
    assert_eq!(solution.routes[0], vec![2, 3]);
    assert_eq!(get_route(&solution, 0), vec![2, 3]);

    replace_route(&mut solution, 1, vec![1]);
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
