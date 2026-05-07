use solverforge::prelude::*;
use solverforge::stream::ConstraintFactory;

use super::{Resource, Task};

#[planning_solution(
    constraints = "define_constraints",
    solver_toml = "../../fixtures/scalar_runtime_publication_solver.toml"
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
        .filter(|task: &Task| task.resource_idx.is_none())
        .penalize(HardSoftScore::of(0, 1))
        .named("unassigned task"),)
}
