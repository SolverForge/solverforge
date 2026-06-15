use std::sync::Arc;

use solverforge::cvrp::ProblemData;
use solverforge::prelude::*;

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
    fn vehicle_data_ptr(&self, _entity_idx: usize) -> *const ProblemData {
        Arc::as_ptr(&self.shared)
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

fn constraints() -> impl ConstraintSet<Plan, HardSoftScore> {}

pub fn build_plan() -> Plan {
    let mut distance_matrix = vec![vec![100_i64; 5]; 5];
    for (idx, row) in distance_matrix.iter_mut().enumerate() {
        row[idx] = 0;
    }

    distance_matrix[0][1] = 1;
    distance_matrix[1][3] = 50;
    distance_matrix[3][2] = 1;
    distance_matrix[2][4] = 50;
    distance_matrix[4][0] = 1;

    distance_matrix[1][2] = 1;
    distance_matrix[2][3] = 1;
    distance_matrix[3][4] = 1;

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
