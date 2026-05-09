use solverforge::prelude::*;
use solverforge::stream::ConstraintFactory;

use super::{Resource, Task};

#[planning_solution(
    constraints = "define_constraints",
    solver_toml = "../../fixtures/scalar_runtime_selector_assembly_solver.toml"
)]
pub struct Plan {
    #[problem_fact_collection]
    pub resources: Vec<Resource>,

    #[planning_entity_collection]
    pub tasks: Vec<Task>,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}

fn define_constraints() -> impl ConstraintSet<Plan, HardSoftScore> {
    (ConstraintFactory::<Plan, HardSoftScore>::new()
        .for_each(Plan::tasks())
        .penalize(|_: &Task| HardSoftScore::of(0, 0))
        .named("noop"),)
}
