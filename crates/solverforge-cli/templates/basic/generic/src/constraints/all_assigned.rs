use crate::domain::{Plan, Task};
use solverforge::prelude::*;
use solverforge::IncrementalConstraint;
use solverforge::stream::vec;

/// HARD: Every task must be assigned to a resource.
///
/// Replace or extend this with constraints that reflect your problem's rules.
/// Common additions: capacity limits, skill matching, conflict avoidance, fairness.
pub fn constraint() -> impl IncrementalConstraint<Plan, HardSoftScore> {
    ConstraintFactory::<Plan, HardSoftScore>::new()
        .for_each(vec(|p: &Plan| &p.tasks))
        .filter(|t: &Task| t.resource_idx.is_none())
        .penalize(HardSoftScore::ONE_HARD)
        .named("All tasks assigned")
}
