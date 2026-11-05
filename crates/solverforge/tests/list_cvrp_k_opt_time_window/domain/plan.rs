use std::sync::Arc;

use solverforge::cvrp::ProblemData;
use solverforge::prelude::*;
use solverforge::stream::ConstraintFactory;

use super::Route;

#[planning_solution(
    constraints = "constraints",
    solver_toml = "../../fixtures/list_cvrp_k_opt_time_window_solver.toml"
)]
pub struct Plan {
    #[planning_list_element_collection(owner = "routes")]
    pub customer_values: Vec<usize>,

    #[planning_entity_collection]
    pub routes: Vec<Route>,

    #[planning_score]
    pub score: Option<HardSoftScore>,

    pub shared: Arc<ProblemData>,
}

impl solverforge::cvrp::VrpSolution for Plan {
    fn vehicle_data(&self, _entity_idx: usize) -> Option<&ProblemData> {
        Some(&self.shared)
    }

    fn vehicle_visits(&self, entity_idx: usize) -> &[usize] {
        &self.routes[entity_idx].visits
    }

    fn vehicle_visits_mut(&mut self, entity_idx: usize) -> &mut Vec<usize> {
        &mut self.routes[entity_idx].visits
    }

    fn vehicle_count(&self) -> usize {
        self.routes.len()
    }
}

/// Directed distance between two nodes, the depot being node 0: 1 along
/// `0 → 1 → 2 → 3 → 4 → 0` and for `3 → 2`, 50 for `1 → 3` and `2 → 4`, 100
/// otherwise.
fn distance(from: usize, to: usize) -> i64 {
    match (from, to) {
        _ if from == to => 0,
        (0, 1) | (1, 2) | (2, 3) | (3, 4) | (4, 0) | (3, 2) => 1,
        (1, 3) | (2, 4) => 50,
        _ => 100,
    }
}

/// Each route's travelled distance, depot to depot.
fn route_distance(route: &Route) -> i64 {
    let mut previous = 0;
    let mut total = 0;
    for &visit in route.visits.iter().chain(std::iter::once(&0)) {
        total += distance(previous, visit);
        previous = visit;
    }
    total
}

fn constraints() -> impl ConstraintSet<Plan, HardSoftScore> {
    (ConstraintFactory::<Plan, HardSoftScore>::new()
        .for_each(Plan::routes())
        .penalize(|route: &Route| HardSoftScore::of(0, route_distance(route)))
        .named("route distance"),)
}

pub fn build_plan() -> Plan {
    let distance_matrix = (0..5)
        .map(|from| (0..5).map(|to| distance(from, to)).collect())
        .collect();

    let mut travel_times = vec![vec![0_i64; 5]; 5];
    travel_times[1][2] = 10;
    travel_times[2][3] = 10;

    let shared = Arc::new(ProblemData {
        capacity: 100,
        depot: 0,
        demands: vec![0, 1, 1, 1, 1],
        distance_matrix,
        time_windows: vec![(0, 100), (0, 100), (0, 100), (0, 5), (0, 100)],
        service_durations: vec![0; 5],
        travel_times,
        vehicle_departure_time: 0,
    });

    Plan {
        customer_values: vec![1, 2, 3, 4],
        routes: vec![Route {
            id: 0,
            visits: vec![1, 3, 2, 4],
        }],
        score: None,
        shared,
    }
}
