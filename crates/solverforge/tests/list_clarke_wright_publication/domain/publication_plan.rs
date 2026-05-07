use std::sync::Arc;

use solverforge::cvrp::ProblemData;
use solverforge::prelude::*;
use solverforge::stream::joiner::equal_bi;
use solverforge::stream::ConstraintFactory;
use solverforge::SolverConfig;

use super::{Customer, Route};

#[planning_solution(
    constraints = "define_constraints",
    config = "solver_config_for_plan",
    solver_toml = "../../fixtures/list_clarke_wright_publication_solver.toml"
)]
pub struct PublicationPlan {
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

impl solverforge::cvrp::VrpSolution for PublicationPlan {
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

fn define_constraints() -> impl ConstraintSet<PublicationPlan, HardSoftScore> {
    (ConstraintFactory::<PublicationPlan, HardSoftScore>::new()
        .for_each(PublicationPlan::customers())
        .if_not_exists((
            ConstraintFactory::<PublicationPlan, HardSoftScore>::new()
                .for_each(PublicationPlan::routes())
                .flattened(|route: &Route| &route.visits),
            equal_bi(
                |customer: &Customer| customer.id,
                |assigned: &usize| *assigned,
            ),
        ))
        .penalize_hard()
        .named("all_customers_assigned"),)
}

fn solver_config_for_plan(plan: &PublicationPlan, config: SolverConfig) -> SolverConfig {
    config.with_termination_seconds(plan.time_limit_secs)
}

pub fn build_plan(customer_count: usize, time_limit_secs: u64) -> PublicationPlan {
    let dimension = customer_count + 1;
    let mut distance_matrix = vec![vec![0_i64; dimension]; dimension];
    for (i, row) in distance_matrix.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            let gap = i.abs_diff(j) as i64;
            *cell = if i == j { 0 } else { gap + 1 };
        }
    }
    let travel_times = distance_matrix.clone();

    let shared = Arc::new(ProblemData {
        capacity: customer_count as i64,
        depot: 0,
        demands: std::iter::once(0)
            .chain(std::iter::repeat_n(1, customer_count))
            .collect(),
        distance_matrix,
        time_windows: vec![(0, i64::MAX); dimension],
        service_durations: vec![0; dimension],
        travel_times,
        vehicle_departure_time: 0,
    });
    let data_addr = Arc::as_ptr(&shared) as usize;
    let customer_values: Vec<usize> = (1..=customer_count).collect();

    PublicationPlan {
        customers: customer_values
            .iter()
            .copied()
            .map(|id| Customer { id })
            .collect(),
        customer_values,
        routes: (0..customer_count)
            .map(|id| Route {
                id,
                visits: Vec::new(),
                data_addr,
            })
            .collect(),
        score: None,
        shared,
        time_limit_secs,
    }
}
