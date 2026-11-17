use std::sync::Arc;

use solverforge::cvrp::{DenseMatrix, ProblemData};
use solverforge::prelude::*;
use solverforge::stream::joiner::equal_bi;
use solverforge::stream::ConstraintFactory;
use solverforge::SolverConfig;

use super::{Customer, Route};

#[planning_solution(
    constraints = "define_constraints",
    config = "solver_config_for_plan"
)]
pub struct FleetPlan {
    #[problem_fact_collection]
    pub customers: Vec<Customer>,

    #[planning_list_element_collection(owner = "routes")]
    pub customer_values: Vec<usize>,

    #[planning_entity_collection]
    pub routes: Vec<Route>,

    #[planning_score]
    pub score: Option<HardSoftScore>,

    pub shared: Arc<ProblemData>,
    pub time_limit_secs: u64,
}

impl solverforge::cvrp::VrpSolution for FleetPlan {
    type Metric = DenseMatrix;

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

fn define_constraints() -> impl ConstraintSet<FleetPlan, HardSoftScore> {
    (ConstraintFactory::<FleetPlan, HardSoftScore>::new()
        .for_each(FleetPlan::customers())
        .if_not_exists((
            ConstraintFactory::<FleetPlan, HardSoftScore>::new()
                .for_each(FleetPlan::routes())
                .flattened(|route: &Route| &route.visits),
            equal_bi(
                |customer: &Customer| customer.id,
                |assigned: &usize| *assigned,
            ),
        ))
        .penalize(HardSoftScore::ONE_HARD)
        .named("all_customers_assigned"),)
}

fn solver_config_for_plan(plan: &FleetPlan, config: SolverConfig) -> SolverConfig {
    config.with_termination_seconds(plan.time_limit_secs)
}

/// A fleet of `vehicle_count` routes with `capacity` each over unit demands;
/// `window_end` closes every even customer's time window at that time.
pub fn build_fleet_plan(
    customer_count: usize,
    vehicle_count: usize,
    capacity: i64,
    window_end: Option<i64>,
) -> FleetPlan {
    let dimension = customer_count + 1;
    let mut distance_matrix = vec![vec![0_i64; dimension]; dimension];
    for (i, row) in distance_matrix.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            let gap = i.abs_diff(j) as i64;
            *cell = if i == j { 0 } else { gap + 1 };
        }
    }
    let travel_times = distance_matrix.clone();
    let time_windows = (0..dimension)
        .map(|node| match window_end {
            Some(end) if node > 0 && node % 2 == 0 => (0, end),
            _ => (0, i64::MAX),
        })
        .collect();

    let shared = Arc::new(ProblemData {
        capacity,
        depot: 0,
        demands: std::iter::once(0)
            .chain(std::iter::repeat_n(1, customer_count))
            .collect(),
        metric: DenseMatrix::new(distance_matrix, travel_times),
        time_windows,
        service_durations: vec![0; dimension],
        vehicle_departure_time: 0,
    });
    let data_addr = Arc::as_ptr(&shared) as usize;
    let customer_values: Vec<usize> = (1..=customer_count).collect();

    FleetPlan {
        customers: customer_values
            .iter()
            .copied()
            .map(|id| Customer { id })
            .collect(),
        customer_values,
        routes: (0..vehicle_count)
            .map(|id| Route {
                id,
                visits: Vec::new(),
                data_addr,
            })
            .collect(),
        score: None,
        shared,
        time_limit_secs: 30,
    }
}
