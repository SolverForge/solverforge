use solverforge::prelude::*;

#[path = "scalar_target_source_reuse/domain/mod.rs"]
mod domain;

use domain::Plan;

fn provider(_plan: &Plan, _limits: ScalarGroupLimits) -> Vec<ScalarCandidate<Plan>> {
    Vec::new()
}

#[test]
fn bound_generated_source_can_build_multiple_scalar_targets() {
    let shifts = Plan::shifts();

    let group = ScalarGroup::candidates(
        "paired_assignment",
        vec![shifts.scalar("primary"), shifts.scalar("secondary")],
        provider,
    );

    assert_eq!(group.targets().len(), 2);
}
