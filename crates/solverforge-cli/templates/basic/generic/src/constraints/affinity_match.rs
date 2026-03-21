use crate::domain::{Plan, Task};
use solverforge::prelude::*;
use solverforge::stream::joiner::equal_bi;
use solverforge::stream::vec;
use solverforge::IncrementalConstraint;

/// SOFT: Prefer assignments whose affinity group matches the task preference.
pub fn constraint() -> impl IncrementalConstraint<Plan, HardSoftScore> {
    ConstraintFactory::<Plan, HardSoftScore>::new()
        .for_each(vec(|p: &Plan| &p.tasks))
        .join((
            vec(|p: &Plan| &p.resources),
            equal_bi(
                |task: &Task| task.resource_idx,
                |resource: &crate::domain::Resource| Some(resource.index),
            ),
        ))
        .penalize_with(|task: &Task, resource: &crate::domain::Resource| {
            if task.preferred_group == resource.affinity_group {
                HardSoftScore::ZERO
            } else {
                HardSoftScore::of(0, task.demand)
            }
        })
        .named("Affinity match")
}
