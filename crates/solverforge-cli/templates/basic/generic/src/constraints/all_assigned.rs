use crate::domain::{Plan, Task};
use solverforge::prelude::*;
use solverforge::IncrementalConstraint;

/// HARD: Every task must be assigned to a resource.
///
/// Replace or extend this with constraints that reflect your problem's rules.
/// Common additions: capacity limits, skill matching, conflict avoidance, fairness.
pub fn constraint() -> impl IncrementalConstraint<Plan, HardSoftScore> {
    ConstraintFactory::<Plan, HardSoftScore>::new()
        .for_each(|p: &Plan| p.tasks.as_slice())
        .filter(|t: &Task| t.resource_idx.is_none())
        .penalize(HardSoftScore::ONE_HARD)
        .as_constraint("All tasks assigned")
}
