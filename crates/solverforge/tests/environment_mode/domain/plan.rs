use solverforge::prelude::*;
use solverforge::stream::{joiner::equal, ConstraintFactory};

use super::{Resource, Task};

#[planning_solution(constraints = "define_constraints")]
pub struct Plan {
    #[problem_fact_collection]
    pub resources: Vec<Resource>,

    #[planning_entity_collection]
    pub tasks: Vec<Task>,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}

impl Plan {
    /// Twelve weighted tasks on four resources: many local optima, so the
    /// random seed decides where a bounded random search ends.
    pub fn new() -> Self {
        Self {
            resources: (0..4).map(|id| Resource { id }).collect(),
            tasks: (0..12)
                .map(|id| Task {
                    id,
                    weight: (id as i64 * 5 + 2) % 9,
                    resource_idx: None,
                })
                .collect(),
            score: None,
        }
    }
}

fn define_constraints() -> impl ConstraintSet<Plan, HardSoftScore> {
    (
        ConstraintFactory::<Plan, HardSoftScore>::new()
            .for_each(Plan::tasks())
            .filter(|task: &Task| task.resource_idx.is_none())
            .penalize(HardSoftScore::ONE_HARD)
            .named("unassigned task"),
        ConstraintFactory::<Plan, HardSoftScore>::new()
            .for_each(Plan::tasks())
            .join(equal(|task: &Task| task.resource_idx))
            .penalize(HardSoftScore::ONE_SOFT)
            .named("shared resource"),
        ConstraintFactory::<Plan, HardSoftScore>::new()
            .for_each(Plan::tasks())
            .filter(|task: &Task| task.resource_idx.is_some())
            .penalize(|task: &Task| {
                let resource = task.resource_idx.unwrap_or_default() as i64;
                HardSoftScore::of(0, (task.weight * (resource + 3)) % 7)
            })
            .named("placement cost"),
    )
}
