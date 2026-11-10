use solverforge::prelude::*;
use solverforge::stream::ConstraintFactory;

use crate::{Route, Visit};

/// A plan with one list-and-scalar entity collection and one fact collection.
#[planning_solution(
    constraints = "constraints",
    solver_toml = "../fixtures/mixed_variable_order_runtime_solver.toml"
)]
pub struct Plan {
    /// Routes the solver plans.
    #[planning_entity_collection]
    pub routes: Vec<Route>,

    /// Visits the routes may take.
    #[problem_fact_collection]
    pub visits: Vec<Visit>,

    /// The plan's score.
    #[planning_score]
    pub score: Option<HardSoftScore>,
}

fn constraints() -> impl ConstraintSet<Plan, HardSoftScore> {
    (ConstraintFactory::<Plan, HardSoftScore>::new()
        .for_each(Plan::routes())
        .filter(|route: &Route| route.first_visit.is_none())
        .penalize(HardSoftScore::of(1, 0))
        .named("missing first visit"),)
}
