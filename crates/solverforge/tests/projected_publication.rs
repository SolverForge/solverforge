//! Projected and cross-joined streams are public through the facade and infer
//! their output types.

use solverforge::prelude::*;
use solverforge::stream::ConstraintFactory;
use solverforge::IncrementalConstraint;

#[path = "projected_publication/domain.rs"]
mod domain;

pub use domain::{Assignment, AssignmentCapacity, Capacity, CapacityEntry, GroupRow, Plan};
use domain::{AssignmentEntries, CapacityEntries, CustomCapacityExtract, GroupRows};

#[test]
fn projected_stream_is_public_and_infers_output_type() {
    let constraint = ConstraintFactory::<Plan, HardSoftScore>::new()
        .for_each(Plan::assignments())
        .project(AssignmentEntries)
        .merge(
            ConstraintFactory::<Plan, HardSoftScore>::new()
                .for_each(Plan::capacities())
                .project(CapacityEntries),
        )
        .group_by(
            |entry: &CapacityEntry| entry.bucket,
            sum(|entry: &CapacityEntry| entry.delta),
        )
        .penalize(|_bucket: &usize, delta: &i64| HardSoftScore::of_hard((*delta).max(0)))
        .named("capacity shortage");

    let plan = Plan {
        capacities: vec![Capacity {
            id: 0,
            bucket: 0,
            amount: 3,
        }],
        assignments: vec![Assignment {
            id: 0,
            bucket: 0,
            demand: 5,
        }],
        score: None,
    };

    assert_eq!(constraint.evaluate(&plan), HardSoftScore::of(-2, 0));
}

#[test]
fn cross_join_project_is_public_and_infers_output_type() {
    let constraint = ConstraintFactory::<Plan, HardSoftScore>::new()
        .for_each(Plan::assignments())
        .join((
            ConstraintFactory::<Plan, HardSoftScore>::new().for_each(Plan::capacities()),
            joiner::equal_bi(
                |assignment: &Assignment| assignment.bucket,
                |capacity: &Capacity| capacity.bucket,
            ),
        ))
        .project(
            |assignment: &Assignment, capacity: &Capacity| AssignmentCapacity {
                bucket: assignment.bucket,
                demand: assignment.demand,
                capacity: capacity.amount,
            },
        )
        .penalize(|row: &AssignmentCapacity| {
            HardSoftScore::of_hard((row.demand - row.capacity).max(0))
        })
        .named("assignment capacity shortage");

    let plan = Plan {
        capacities: vec![Capacity {
            id: 0,
            bucket: 0,
            amount: 3,
        }],
        assignments: vec![Assignment {
            id: 0,
            bucket: 0,
            demand: 5,
        }],
        score: None,
    };

    assert_eq!(constraint.evaluate(&plan), HardSoftScore::of(-2, 0));
}

#[test]
fn projected_self_join_accepts_directed_distinct_keys() {
    let constraint = ConstraintFactory::<Plan, HardSoftScore>::new()
        .for_each(Plan::assignments())
        .project(GroupRows)
        .join(joiner::equal_bi(
            |left: &GroupRow| Some(left.group),
            |right: &GroupRow| right.parent,
        ))
        .penalize(|left: &GroupRow, right: &GroupRow| {
            HardSoftScore::of_hard((left.group * 10 + right.group) as i64)
        })
        .named("projected parent child");

    let plan = Plan {
        capacities: Vec::new(),
        assignments: vec![
            Assignment {
                id: 0,
                bucket: 1,
                demand: -1,
            },
            Assignment {
                id: 1,
                bucket: 2,
                demand: 1,
            },
            Assignment {
                id: 2,
                bucket: 3,
                demand: 1,
            },
        ],
        score: None,
    };

    assert_eq!(constraint.evaluate(&plan), HardSoftScore::of(-25, 0));
}

#[test]
fn cross_join_accepts_custom_collection_extract_target() {
    let constraint = ConstraintFactory::<Plan, HardSoftScore>::new()
        .for_each(Plan::assignments())
        .join((
            CustomCapacityExtract,
            joiner::equal_bi(
                |assignment: &Assignment| assignment.bucket,
                |capacity: &Capacity| capacity.bucket,
            ),
        ))
        .penalize(|_assignment: &Assignment, capacity: &Capacity| {
            HardSoftScore::of_hard(capacity.amount)
        })
        .named("custom capacity target");

    let plan = Plan {
        capacities: vec![Capacity {
            id: 0,
            bucket: 0,
            amount: 3,
        }],
        assignments: vec![Assignment {
            id: 0,
            bucket: 0,
            demand: 5,
        }],
        score: None,
    };

    assert_eq!(constraint.evaluate(&plan), HardSoftScore::of(-3, 0));
}

#[test]
fn cross_join_accepts_filtered_source_stream_target() {
    let constraint = ConstraintFactory::<Plan, HardSoftScore>::new()
        .for_each(Plan::assignments())
        .join((
            ConstraintFactory::<Plan, HardSoftScore>::new()
                .for_each(Plan::capacities())
                .filter(|capacity: &Capacity| capacity.amount > 5),
            joiner::equal_bi(
                |assignment: &Assignment| assignment.bucket,
                |capacity: &Capacity| capacity.bucket,
            ),
        ))
        .penalize(|_assignment: &Assignment, capacity: &Capacity| {
            HardSoftScore::of_hard(capacity.amount)
        })
        .named("filtered capacity target");

    let plan = Plan {
        capacities: vec![
            Capacity {
                id: 0,
                bucket: 0,
                amount: 3,
            },
            Capacity {
                id: 1,
                bucket: 0,
                amount: 8,
            },
        ],
        assignments: vec![Assignment {
            id: 0,
            bucket: 0,
            demand: 5,
        }],
        score: None,
    };

    assert_eq!(constraint.evaluate(&plan), HardSoftScore::of(-8, 0));
}

#[test]
fn cross_join_accepts_generated_source_target() {
    let constraint = ConstraintFactory::<Plan, HardSoftScore>::new()
        .for_each(Plan::assignments())
        .join((
            Plan::capacities(),
            joiner::equal_bi(
                |assignment: &Assignment| assignment.bucket,
                |capacity: &Capacity| capacity.bucket,
            ),
        ))
        .penalize(|_assignment: &Assignment, capacity: &Capacity| {
            HardSoftScore::of_hard(capacity.amount)
        })
        .named("generated capacity target");

    let plan = Plan {
        capacities: vec![Capacity {
            id: 0,
            bucket: 0,
            amount: 3,
        }],
        assignments: vec![Assignment {
            id: 0,
            bucket: 0,
            demand: 5,
        }],
        score: None,
    };

    assert_eq!(constraint.evaluate(&plan), HardSoftScore::of(-3, 0));
}

#[test]
fn cross_join_group_by_is_public_without_projecting_first() {
    let constraint = ConstraintFactory::<Plan, HardSoftScore>::new()
        .for_each(Plan::assignments())
        .join((
            ConstraintFactory::<Plan, HardSoftScore>::new().for_each(Plan::capacities()),
            joiner::equal_bi(
                |assignment: &Assignment| assignment.bucket,
                |capacity: &Capacity| capacity.bucket,
            ),
        ))
        .group_by(
            |_assignment: &Assignment, capacity: &Capacity| capacity.bucket,
            sum(|(_assignment, _capacity): (&Assignment, &Capacity)| 1i64),
        )
        .penalize(|_bucket: &usize, count: &i64| HardSoftScore::of_hard(*count))
        .named("assignments per capacity");

    let plan = Plan {
        capacities: vec![Capacity {
            id: 0,
            bucket: 0,
            amount: 3,
        }],
        assignments: vec![Assignment {
            id: 0,
            bucket: 0,
            demand: 5,
        }],
        score: None,
    };

    assert_eq!(constraint.evaluate(&plan), HardSoftScore::of(-1, 0));
}

#[test]
fn cross_join_group_by_complement_is_public_without_projecting_first() {
    let constraint = ConstraintFactory::<Plan, HardSoftScore>::new()
        .for_each(Plan::assignments())
        .join((
            ConstraintFactory::<Plan, HardSoftScore>::new().for_each(Plan::capacities()),
            joiner::equal_bi(
                |assignment: &Assignment| assignment.bucket,
                |capacity: &Capacity| capacity.bucket,
            ),
        ))
        .group_by(
            |_assignment: &Assignment, capacity: &Capacity| capacity.bucket,
            sum(|(assignment, capacity): (&Assignment, &Capacity)| {
                assignment.demand - capacity.amount
            }),
        )
        .complement(
            Plan::capacities(),
            |capacity: &Capacity| capacity.bucket,
            |_| 0i64,
        )
        .penalize(|bucket: &usize, delta: &i64| {
            HardSoftScore::of_hard((*bucket as i64 * 10) + *delta)
        })
        .named("direct grouped assignment capacity shortage including empty capacity");

    let plan = Plan {
        capacities: vec![
            Capacity {
                id: 0,
                bucket: 0,
                amount: 3,
            },
            Capacity {
                id: 1,
                bucket: 1,
                amount: 4,
            },
        ],
        assignments: vec![Assignment {
            id: 0,
            bucket: 0,
            demand: 5,
        }],
        score: None,
    };

    assert_eq!(constraint.evaluate(&plan), HardSoftScore::of(-12, 0));
}

#[test]
fn cross_join_project_group_by_complement_is_public() {
    let constraint = ConstraintFactory::<Plan, HardSoftScore>::new()
        .for_each(Plan::assignments())
        .join((
            ConstraintFactory::<Plan, HardSoftScore>::new().for_each(Plan::capacities()),
            joiner::equal_bi(
                |assignment: &Assignment| assignment.bucket,
                |capacity: &Capacity| capacity.bucket,
            ),
        ))
        .project(
            |assignment: &Assignment, capacity: &Capacity| CapacityEntry {
                bucket: assignment.bucket,
                delta: assignment.demand - capacity.amount,
            },
        )
        .group_by(
            |entry: &CapacityEntry| entry.bucket,
            sum(|entry: &CapacityEntry| entry.delta),
        )
        .complement(
            Plan::capacities(),
            |capacity: &Capacity| capacity.bucket,
            |_| 0i64,
        )
        .penalize(|bucket: &usize, delta: &i64| {
            HardSoftScore::of_hard((*bucket as i64 * 10) + *delta)
        })
        .named("assignment capacity shortage including empty capacity");

    let plan = Plan {
        capacities: vec![
            Capacity {
                id: 0,
                bucket: 0,
                amount: 3,
            },
            Capacity {
                id: 1,
                bucket: 1,
                amount: 4,
            },
        ],
        assignments: vec![Assignment {
            id: 0,
            bucket: 0,
            demand: 5,
        }],
        score: None,
    };

    assert_eq!(constraint.evaluate(&plan), HardSoftScore::of(-12, 0));
}
