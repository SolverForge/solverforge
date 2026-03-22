use crate::domain::{Plan, Task};
use solverforge::prelude::*;
use solverforge::IncrementalConstraint;
use solverforge::stream::vec;

/// SOFT: Minimize variance in resource load (balanced assignment).
///
/// Penalizes uneven task distribution across resources. This keeps the default
/// standard-variable template generic while giving local search a visible goal.
pub fn constraint() -> impl IncrementalConstraint<Plan, HardSoftScore> {
    ConstraintFactory::<Plan, HardSoftScore>::new()
        .for_each(vec(|p: &Plan| &p.tasks))
        .balance(|t: &Task| t.resource_idx)
        .penalize(HardSoftScore::ONE_SOFT)
        .named("Balanced load")
}
