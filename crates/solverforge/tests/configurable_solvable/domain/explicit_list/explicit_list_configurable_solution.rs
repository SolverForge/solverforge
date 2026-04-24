use solverforge::prelude::*;
use solverforge::SolverConfig;

use super::{DummyRoute, DummyVisit};

#[planning_solution(
    constraints = "define_explicit_list_constraints",
    config = "solver_config_for_explicit_list_solution",
    solver_toml = "../../../fixtures/configurable_solvable_solver.toml"
)]
pub struct ExplicitListConfigurableSolution {
    #[problem_fact_collection]
    pub visits: Vec<DummyVisit>,

    #[planning_entity_collection]
    pub routes: Vec<DummyRoute>,

    #[planning_score]
    pub score: Option<HardSoftScore>,

    pub time_limit_secs: u64,
}

fn define_explicit_list_constraints(
) -> impl ConstraintSet<ExplicitListConfigurableSolution, HardSoftScore> {
}

fn solver_config_for_explicit_list_solution(
    solution: &ExplicitListConfigurableSolution,
    config: SolverConfig,
) -> SolverConfig {
    config.with_termination_seconds(solution.time_limit_secs)
}
