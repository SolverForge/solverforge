use solverforge::prelude::*;
use solverforge::stream::ConstraintFactory;

use super::{Resource, Task};

#[planning_solution(constraints = "define_constraints", solver_toml = "solver.toml")]
pub struct Plan {
    #[problem_fact_collection]
    pub resources: Vec<Resource>,

    #[planning_entity_collection]
    pub tasks: Vec<Task>,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}

impl Plan {
    pub fn new() -> Self {
        Self {
            resources: (0..3).map(|id| Resource { id }).collect(),
            tasks: (0..4)
                .map(|id| Task {
                    id,
                    resource_idx: None,
                })
                .collect(),
            score: None,
        }
    }
}

/// Every assigned task costs its resource index plus one, so local search
/// always has non-improving moves to try and runs to its step limit.
fn define_constraints() -> impl ConstraintSet<Plan, HardSoftScore> {
    (
        ConstraintFactory::<Plan, HardSoftScore>::new()
            .for_each(Plan::tasks())
            .filter(|task: &Task| task.resource_idx.is_none())
            .penalize(HardSoftScore::ONE_HARD)
            .named("unassigned task"),
        ConstraintFactory::<Plan, HardSoftScore>::new()
            .for_each(Plan::tasks())
            .filter(|task: &Task| task.resource_idx.is_some())
            .penalize(|task: &Task| {
                HardSoftScore::of(0, task.resource_idx.unwrap_or_default() as i64 + 1)
            })
            .named("placement cost"),
    )
}
