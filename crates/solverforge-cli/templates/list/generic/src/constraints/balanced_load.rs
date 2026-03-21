use crate::domain::{Container, Plan};
use solverforge::prelude::*;
use solverforge::IncrementalConstraint;
use solverforge::stream::vec;

/// SOFT: Minimize variance in container load (balanced distribution).
///
/// Penalizes the square of each container's item count — the solver minimizes
/// the sum, which is equivalent to minimizing variance across containers.
///
/// Replace or extend this with constraints that reflect your problem's rules.
/// Common additions: capacity limits, ordering requirements, conflict avoidance.
pub fn constraint() -> impl IncrementalConstraint<Plan, HardSoftScore> {
    ConstraintFactory::<Plan, HardSoftScore>::new()
        .for_each(vec(|p: &Plan| &p.containers))
        .penalize_with(|c: &Container| {
            let load = c.items.len() as i64;
            HardSoftScore::of(0, load * load)
        })
        .named("Balanced load")
}
