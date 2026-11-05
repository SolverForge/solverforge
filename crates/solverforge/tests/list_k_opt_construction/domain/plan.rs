use std::sync::Arc;

use solverforge::cvrp::ProblemData;
use solverforge::prelude::*;
use solverforge::stream::ConstraintFactory;

use super::Route;

#[planning_solution(constraints = "constraints")]
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

/// Distance along a line from the depot at 0 and back, the same metric as
/// the problem data's distance matrix.
fn line_distance(route: &Route) -> i64 {
    let mut previous = 0usize;
    let mut total = 0i64;
    for &visit in route.visits.iter().chain(std::iter::once(&0)) {
        total += (previous as i64 - visit as i64).abs();
        previous = visit;
    }
    total
}

fn constraints() -> impl ConstraintSet<Plan, HardSoftScore> {
    (ConstraintFactory::<Plan, HardSoftScore>::new()
        .for_each(Plan::routes())
        .penalize(|route: &Route| HardSoftScore::of(0, line_distance(route)))
        .named("route distance"),)
}

/// Six customers on a line, travel time 10 per hop, customer `c` due at
/// `10 * c`: only the sorted route is on time. From `[1, 4, 5, 2, 3, 6]` no
/// single reversal reaches it, one 3-opt segment move does.
pub fn sorting_plan() -> Plan {
    let size = 7;
    let distance_matrix = (0..size)
        .map(|from| (0..size).map(|to| (from as i64 - to as i64).abs()).collect())
        .collect();
    let travel_times = (0..size)
        .map(|from| (0..size).map(|to| if from == to { 0 } else { 10 }).collect())
        .collect();
    let time_windows = (0..size)
        .map(|customer| (0, if customer == 0 { 1_000 } else { 10 * customer as i64 }))
        .collect();
    let shared = Arc::new(ProblemData {
        capacity: 100,
        depot: 0,
        demands: vec![0, 1, 1, 1, 1, 1, 1],
        distance_matrix,
        time_windows,
        service_durations: vec![0; size],
        travel_times,
        vehicle_departure_time: 0,
    });
    Plan {
        customer_values: vec![1, 2, 3, 4, 5, 6],
        routes: vec![Route {
            id: 0,
            visits: vec![1, 4, 5, 2, 3, 6],
        }],
        score: None,
        shared,
    }
}
