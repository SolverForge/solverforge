use solverforge::prelude::*;
use solverforge::stream::ConstraintFactory;

use crate::{Route, Visit};

#[planning_solution(
    constraints = "constraints",
    solver_toml = "../fixtures/mixed_variable_order_runtime_solver.toml"
)]
pub struct Plan {
    #[planning_entity_collection]
    pub routes: Vec<Route>,

    #[problem_fact_collection]
    pub visits: Vec<Visit>,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}

fn constraints() -> impl ConstraintSet<Plan, HardSoftScore> {
    use PlanConstraintStreams;

    (ConstraintFactory::<Plan, HardSoftScore>::new()
        .routes()
        .filter(|route: &Route| route.first_visit.is_none())
        .penalize(HardSoftScore::of(1, 0))
        .named("missing first visit"),)
}
