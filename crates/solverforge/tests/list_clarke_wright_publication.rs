use std::sync::Arc;

use solverforge::cvrp::ProblemData;
use solverforge::prelude::*;
use solverforge::stream::joiner::equal_bi;
use solverforge::stream::ConstraintFactory;
use solverforge::{SolverConfig, SolverEvent, SolverManager};

#[problem_fact]
struct Customer {
    #[planning_id]
    id: usize,
}

#[planning_entity]
struct Route {
    #[planning_id]
    id: usize,

    #[planning_list_variable(
        element_collection = "customer_values",
        solution_trait = "solverforge::cvrp::VrpSolution",
        distance_meter = "solverforge::cvrp::MatrixDistanceMeter",
        intra_distance_meter = "solverforge::cvrp::MatrixIntraDistanceMeter",
        cw_depot_fn = "solverforge::cvrp::depot_for_cw",
        cw_distance_fn = "solverforge::cvrp::distance",
        cw_element_load_fn = "solverforge::cvrp::element_load",
        cw_capacity_fn = "solverforge::cvrp::capacity",
        cw_assign_route_fn = "solverforge::cvrp::replace_route",
        k_opt_get_route = "solverforge::cvrp::get_route",
        k_opt_set_route = "solverforge::cvrp::replace_route",
        k_opt_depot_fn = "solverforge::cvrp::depot_for_entity",
        k_opt_distance_fn = "solverforge::cvrp::distance"
    )]
    visits: Vec<usize>,

    data_addr: usize,
}

#[planning_solution(
    constraints = "define_constraints",
    config = "solver_config_for_plan",
    solver_toml = "fixtures/list_clarke_wright_publication_solver.toml"
)]
struct PublicationPlan {
    #[problem_fact_collection]
    customers: Vec<Customer>,

    #[planning_list_element_collection(owner = "routes")]
    customer_values: Vec<usize>,

    #[planning_entity_collection]
    routes: Vec<Route>,

    #[planning_score]
    score: Option<HardSoftScore>,

    shared: Arc<ProblemData>,
    time_limit_secs: u64,
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
    use PublicationPlanConstraintStreams;

    (ConstraintFactory::<PublicationPlan, HardSoftScore>::new()
        .customers()
        .if_not_exists((
            ConstraintFactory::<PublicationPlan, HardSoftScore>::new()
                .routes()
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

fn build_plan(customer_count: usize, time_limit_secs: u64) -> PublicationPlan {
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

#[test]
fn stock_clarke_wright_publishes_constructed_solution_under_solver_manager() {
    static MANAGER: SolverManager<PublicationPlan> = SolverManager::new();

    let plan = build_plan(20, 1);
    let expected_customers = plan.customer_values.len();
    let (job_id, mut receiver) = MANAGER.solve(plan).expect("solve should start");
    let mut saw_non_empty_best = false;

    let completed = loop {
        match receiver
            .blocking_recv()
            .expect("event stream should reach a terminal event")
        {
            SolverEvent::BestSolution { solution, .. } => {
                if solution.routes.iter().any(|route| !route.visits.is_empty()) {
                    saw_non_empty_best = true;
                }
            }
            SolverEvent::Completed { solution, .. } => break solution,
            SolverEvent::Cancelled { metadata } => {
                panic!(
                    "solve was unexpectedly cancelled: {:?}",
                    metadata.terminal_reason
                )
            }
            SolverEvent::Failed { error, .. } => panic!("solve unexpectedly failed: {error}"),
            SolverEvent::Progress { .. }
            | SolverEvent::PauseRequested { .. }
            | SolverEvent::Paused { .. }
            | SolverEvent::Resumed { .. } => {}
        }
    };

    MANAGER
        .delete(job_id)
        .expect("completed job should delete cleanly");

    let assigned_count: usize = completed
        .routes
        .iter()
        .map(|route| route.visits.len())
        .sum();

    assert!(
        saw_non_empty_best,
        "expected a constructed best solution event"
    );
    assert_eq!(assigned_count, expected_customers);
    assert!(
        completed
            .routes
            .iter()
            .any(|route| !route.visits.is_empty()),
        "completed solution should contain constructed routes"
    );
}
