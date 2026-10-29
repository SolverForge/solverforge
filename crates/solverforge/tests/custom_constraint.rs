//! A hand-written constraint, written as `docs/extend-scoring.md` describes,
//! composes with a fluent constraint and keeps the incremental score equal to
//! a from-scratch score through every variable change.

#[path = "custom_constraint/domain/mod.rs"]
mod domain;

use domain::{Plan, Resource, ResourceLoad, Task};
use solverforge::prelude::*;
use solverforge::stream::ConstraintFactory;

const TASKS: usize = 6;

fn plan() -> Plan {
    Plan {
        resources: (0..3).map(|id| Resource { id }).collect(),
        tasks: (0..TASKS)
            .map(|id| Task {
                id,
                size: id as i64 + 1,
                resource_idx: None,
            })
            .collect(),
        score: None,
    }
}

fn constraints() -> impl ConstraintSet<Plan, HardSoftScore> {
    (
        ConstraintFactory::<Plan, HardSoftScore>::new()
            .for_each(Plan::tasks())
            .filter(|task: &Task| task.resource_idx.is_none())
            .penalize(HardSoftScore::ONE_HARD)
            .named("unassigned task"),
        ResourceLoad::new(),
    )
}

fn from_scratch(solution: &Plan) -> HardSoftScore {
    let mut director = ScoreDirector::new(solution.clone(), constraints());
    director.calculate_score()
}

#[test]
fn incremental_score_matches_a_from_scratch_score_after_every_change() {
    let mut director = ScoreDirector::new(plan(), constraints());
    assert_eq!(
        director.calculate_score(),
        HardSoftScore::of(-(TASKS as i64), 0)
    );

    // Assign, move between resources, and unassign, so tasks share and leave
    // resources and the custom constraint's retained loads change each time.
    let changes = [
        (0, Some(0)),
        (1, Some(0)),
        (2, Some(1)),
        (3, Some(0)),
        (1, Some(2)),
        (4, Some(2)),
        (0, None),
        (5, Some(1)),
        (3, Some(1)),
        (0, Some(1)),
        (2, None),
    ];
    for (task, resource) in changes {
        director.before_variable_changed(0, task);
        director.working_solution_mut().tasks[task].resource_idx = resource;
        director.after_variable_changed(0, task);

        let incremental = director.calculate_score();
        assert_eq!(
            incremental,
            from_scratch(director.working_solution()),
            "after assigning task {task} to {resource:?}"
        );
    }
}

#[test]
fn initialize_after_reset_matches_evaluate() {
    let mut solution = plan();
    for (task, resource) in [(0, 0), (1, 0), (2, 1), (4, 2)] {
        solution.tasks[task].resource_idx = Some(resource);
    }
    let mut constraint = ResourceLoad::new();
    let expected = constraint.evaluate(&solution);
    assert_eq!(expected, HardSoftScore::of_soft(-(9 + 9 + 25)));
    assert_eq!(constraint.initialize(&solution), expected);
    constraint.reset();
    assert_eq!(constraint.initialize(&solution), expected);
}
