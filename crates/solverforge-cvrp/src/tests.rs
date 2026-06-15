use super::*;
use solverforge_solver::CrossEntityDistanceMeter;

struct TestSolution {
    data: Vec<ProblemData>,
    routes: Vec<Vec<usize>>,
}

struct SharedDataSolution {
    data: ProblemData,
    routes: Vec<Vec<usize>>,
}

impl TestSolution {
    fn new(routes: Vec<Vec<usize>>) -> Self {
        let data = base_problem_data();
        let vehicle_data = (0..routes.len()).map(|_| data.clone()).collect();
        Self {
            data: vehicle_data,
            routes,
        }
    }

    fn with_data(routes: Vec<Vec<usize>>, data: Vec<ProblemData>) -> Self {
        assert_eq!(routes.len(), data.len());
        Self { data, routes }
    }
}

fn base_problem_data() -> ProblemData {
    ProblemData {
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
    }
}

impl VrpSolution for TestSolution {
    fn vehicle_data_ptr(&self, entity_idx: usize) -> *const ProblemData {
        &self.data[entity_idx] as *const ProblemData
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

impl VrpSolution for SharedDataSolution {
    fn vehicle_data_ptr(&self, _entity_idx: usize) -> *const ProblemData {
        &self.data as *const ProblemData
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
fn helpers_use_problem_data_for_route_owner() {
    let mut owner_one_data = base_problem_data();
    owner_one_data.depot = 3;
    owner_one_data.distance_matrix[1][3] = 42;
    let solution = TestSolution::with_data(
        vec![vec![1, 2], vec![3]],
        vec![base_problem_data(), owner_one_data],
    );

    assert_eq!(route_distance(&solution, 1, 1, 3), 42);
    assert_eq!(depot_for_entity(&solution, 0), 0);
    assert_eq!(depot_for_entity(&solution, 1), 3);
    assert!(route_feasible(&solution, 0, &[1, 2]));
}

#[test]
fn helpers_handle_empty_fleets() {
    let solution = TestSolution::new(vec![]);

    assert_eq!(route_distance(&solution, 0, 1, 2), 0);
    assert_eq!(depot_for_entity(&solution, 0), 0);
    assert_eq!(savings_metric_class(&solution, 3), 3);
    assert!(!route_feasible(&solution, 0, &[1, 2]));
    assert!(route_feasible(&solution, 0, &[]));
}

#[test]
fn savings_metric_class_groups_shared_problem_data() {
    let solution = SharedDataSolution {
        data: base_problem_data(),
        routes: vec![vec![1], vec![2], vec![3]],
    };

    assert_eq!(
        savings_metric_class(&solution, 0),
        savings_metric_class(&solution, 1)
    );
    assert_eq!(
        savings_metric_class(&solution, 1),
        savings_metric_class(&solution, 2)
    );
}

#[test]
fn savings_metric_class_separates_distinct_problem_data() {
    let solution = TestSolution::new(vec![vec![1], vec![2]]);

    assert_ne!(
        savings_metric_class(&solution, 0),
        savings_metric_class(&solution, 1)
    );
}

#[test]
fn clarke_wright_adapters_share_exact_cvrp_data_when_requested() {
    let solution = TestSolution::new(vec![vec![1, 2], vec![3]]);

    assert_eq!(
        savings_depot_for_entity(&solution, 0),
        depot_for_entity(&solution, 0)
    );
    assert_eq!(
        savings_distance(&solution, 0, 1, 2),
        route_distance(&solution, 0, 1, 2)
    );
    assert_eq!(
        savings_feasible(&solution, 0, &[1, 2]),
        route_feasible(&solution, 0, &[1, 2])
    );
}

#[test]
fn route_feasibility_rejects_unreachable_depot_leg_without_panic() {
    let mut data = base_problem_data();
    data.distance_matrix[0][1] = UNREACHABLE;
    data.travel_times[0][1] = UNREACHABLE;
    let solution = TestSolution::with_data(vec![vec![1]], vec![data]);

    let outcome = std::panic::catch_unwind(|| route_feasible(&solution, 0, &[1]));

    assert!(matches!(outcome, Ok(false)));
}

#[test]
fn route_feasibility_rejects_unreachable_inter_visit_leg_without_panic() {
    let mut data = base_problem_data();
    data.distance_matrix[1][2] = UNREACHABLE;
    data.travel_times[1][2] = UNREACHABLE;
    let solution = TestSolution::with_data(vec![vec![1, 2]], vec![data]);

    let outcome = std::panic::catch_unwind(|| route_feasible(&solution, 0, &[1, 2]));

    assert!(matches!(outcome, Ok(false)));
}

#[test]
fn route_feasibility_rejects_unreachable_return_depot_leg_without_panic() {
    let mut data = base_problem_data();
    data.distance_matrix[1][0] = UNREACHABLE;
    data.travel_times[1][0] = UNREACHABLE;
    let solution = TestSolution::with_data(vec![vec![1]], vec![data]);

    let outcome = std::panic::catch_unwind(|| route_feasible(&solution, 0, &[1]));

    assert!(matches!(outcome, Ok(false)));
}

#[test]
fn route_feasibility_rejects_time_overflow_without_wrapping() {
    let mut data = base_problem_data();
    data.vehicle_departure_time = 20;
    data.travel_times[0][1] = i64::MAX - 10;
    let solution = TestSolution::with_data(vec![vec![1]], vec![data]);

    assert!(
        !route_feasible(&solution, 0, &[1]),
        "overflowing travel accumulation must not wrap into a feasible time"
    );
}

#[test]
fn route_feasibility_rejects_service_overflow_without_wrapping() {
    let mut data = base_problem_data();
    data.time_windows[1] = (0, i64::MAX);
    data.service_durations[1] = i64::MAX;
    let solution = TestSolution::with_data(vec![vec![1]], vec![data]);

    assert!(
        !route_feasible(&solution, 0, &[1]),
        "overflowing service accumulation must not wrap into a feasible time"
    );
}

#[test]
fn stock_savings_feasibility_stays_structural_for_unreachable_routes() {
    let mut data = base_problem_data();
    data.distance_matrix[0][1] = UNREACHABLE;
    data.travel_times[0][1] = UNREACHABLE;
    let solution = TestSolution::with_data(vec![vec![1]], vec![data]);

    assert!(savings_feasible(&solution, 0, &[1]));
    assert!(savings_hooks::feasible(&solution, 0, &[1]));
}

#[test]
fn stock_distances_convert_unreachable_or_malformed_legs_to_finite_costs() {
    let mut data = base_problem_data();
    data.distance_matrix[0][1] = UNREACHABLE;
    let solution = TestSolution::with_data(vec![vec![1]], vec![data]);

    let unreachable_cost = route_distance(&solution, 0, 0, 1);
    let malformed_cost = route_distance(&solution, 0, 99, 1);

    assert!(unreachable_cost > 0);
    assert!(unreachable_cost < UNREACHABLE);
    assert_eq!(savings_distance(&solution, 0, 0, 1), unreachable_cost);
    assert_eq!(malformed_cost, unreachable_cost);
}

#[test]
fn hook_bundles_expose_route_and_savings_semantics() {
    let mut solution = TestSolution::new(vec![vec![1, 2], vec![3]]);

    assert_eq!(route_hooks::get(&solution, 0), get_route(&solution, 0));
    route_hooks::set(&mut solution, 1, vec![2, 1]);
    assert_eq!(get_route(&solution, 1), vec![2, 1]);
    assert_eq!(
        route_hooks::depot(&solution, 0),
        depot_for_entity(&solution, 0)
    );
    assert_eq!(
        route_hooks::distance(&solution, 0, 1, 2),
        route_distance(&solution, 0, 1, 2)
    );
    assert_eq!(
        route_hooks::feasible(&solution, 0, &[1, 2]),
        route_feasible(&solution, 0, &[1, 2])
    );
    assert_eq!(
        savings_hooks::depot(&solution, 0),
        savings_depot_for_entity(&solution, 0)
    );
    assert_eq!(
        savings_hooks::distance(&solution, 0, 1, 2),
        savings_distance(&solution, 0, 1, 2)
    );
    assert_eq!(
        savings_hooks::feasible(&solution, 0, &[1, 2]),
        savings_feasible(&solution, 0, &[1, 2])
    );
}

#[test]
#[should_panic(expected = "vehicle_data_ptr(0) returned null")]
fn helpers_reject_missing_problem_data_for_non_empty_fleets() {
    let solution = NullDataSolution {
        routes: vec![vec![1, 2]],
    };

    let _ = route_distance(&solution, 0, 1, 2);
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
fn route_feasibility_rejects_time_violations_while_savings_admits_them() {
    let solution = TestSolution::new(vec![vec![1, 2], vec![3]]);

    assert!(
        route_feasible(&solution, 0, &[1, 2]),
        "route should wait for customer 2 and still finish in time"
    );
    assert!(
        !route_feasible(&solution, 0, &[2, 3]),
        "route-local feasibility should reject missed time windows"
    );
    assert!(!route_hooks::feasible(&solution, 0, &[2, 3]));
    assert!(savings_feasible(&solution, 0, &[2, 3]));
    assert!(savings_hooks::feasible(&solution, 0, &[2, 3]));
}

#[test]
fn route_feasibility_rejects_capacity_violations_while_savings_admits_them() {
    let mut owner_one_data = base_problem_data();
    owner_one_data.capacity = 4;
    let solution = TestSolution::with_data(
        vec![vec![1, 2], vec![3]],
        vec![base_problem_data(), owner_one_data],
    );

    assert!(route_feasible(&solution, 0, &[1, 2]));
    assert!(
        !route_feasible(&solution, 1, &[1, 2]),
        "route-local feasibility should reject over-capacity routes"
    );
    assert!(!route_hooks::feasible(&solution, 1, &[1, 2]));
    assert!(savings_feasible(&solution, 1, &[1, 2]));
    assert!(savings_hooks::feasible(&solution, 1, &[1, 2]));
}

#[test]
fn route_feasibility_rejects_structurally_invalid_routes() {
    let solution = TestSolution::new(vec![vec![1, 2], vec![3]]);
    let null_data = NullDataSolution {
        routes: vec![vec![1, 2]],
    };

    assert!(!route_feasible(&solution, 0, &[4]));
    assert!(!savings_feasible(&solution, 0, &[4]));
    assert!(!route_hooks::feasible(&solution, 0, &[4]));
    assert!(!savings_hooks::feasible(&solution, 0, &[4]));
    assert!(!route_feasible(&solution, 2, &[1]));
    assert!(!savings_feasible(&solution, 2, &[1]));
    assert!(!route_feasible(&null_data, 0, &[1]));
    assert!(!savings_feasible(&null_data, 0, &[1]));
    assert!(route_feasible(&null_data, 0, &[]));
    assert!(savings_feasible(&null_data, 0, &[]));
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

#[test]
fn distance_meters_treat_unreachable_legs_as_infinite() {
    let mut data = base_problem_data();
    data.distance_matrix[1][2] = UNREACHABLE;
    let solution = TestSolution::with_data(vec![vec![1, 2], vec![2]], vec![data.clone(), data]);

    assert!(MatrixDistanceMeter
        .distance(&solution, 0, 0, 1, 0)
        .is_infinite());
    assert!(MatrixIntraDistanceMeter
        .distance(&solution, 0, 0, 0, 1)
        .is_infinite());
}
