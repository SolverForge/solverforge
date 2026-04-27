use std::sync::atomic::{AtomicUsize, Ordering};

use solverforge_core::score::SoftScore;

use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::collection_extract::{source, ChangeSource};
use crate::stream::collector::sum;
use crate::stream::ConstraintFactory;

#[derive(Clone)]
struct Work {
    bucket: usize,
    demand: i64,
    enabled: bool,
}

#[derive(Clone)]
struct Capacity {
    bucket: usize,
    capacity: i64,
}

#[derive(Clone)]
struct Plan {
    work: Vec<Work>,
    capacity: Vec<Capacity>,
}

#[derive(Clone)]
struct Entry {
    bucket: usize,
    delta: i64,
}

fn work(plan: &Plan) -> &[Work] {
    plan.work.as_slice()
}

fn capacity(plan: &Plan) -> &[Capacity] {
    plan.capacity.as_slice()
}

#[test]
fn projected_allows_zero_and_multiple_outputs() {
    let constraint = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(source(
            work as fn(&Plan) -> &[Work],
            ChangeSource::Descriptor(0),
        ))
        .project(|work: &Work| {
            if !work.enabled {
                Vec::new()
            } else {
                vec![
                    Entry {
                        bucket: work.bucket,
                        delta: work.demand,
                    },
                    Entry {
                        bucket: work.bucket + 1,
                        delta: work.demand,
                    },
                ]
            }
        })
        .penalize_with(|entry: &Entry| SoftScore::of(entry.delta))
        .named("projected work");

    let plan = Plan {
        work: vec![
            Work {
                bucket: 0,
                demand: 3,
                enabled: true,
            },
            Work {
                bucket: 0,
                demand: 100,
                enabled: false,
            },
        ],
        capacity: Vec::new(),
    };

    assert_eq!(constraint.match_count(&plan), 2);
    assert_eq!(constraint.evaluate(&plan), SoftScore::of(-6));
}

#[test]
fn projected_grouping_merges_multiple_sources() {
    let constraint = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(source(
            work as fn(&Plan) -> &[Work],
            ChangeSource::Descriptor(0),
        ))
        .project(|work: &Work| {
            vec![Entry {
                bucket: work.bucket,
                delta: work.demand,
            }]
        })
        .merge(
            ConstraintFactory::<Plan, SoftScore>::new()
                .for_each(source(
                    capacity as fn(&Plan) -> &[Capacity],
                    ChangeSource::Descriptor(1),
                ))
                .project(|capacity: &Capacity| {
                    vec![Entry {
                        bucket: capacity.bucket,
                        delta: -capacity.capacity,
                    }]
                }),
        )
        .group_by(
            |entry: &Entry| entry.bucket,
            sum(|entry: &Entry| entry.delta),
        )
        .penalize_with(|delta: &i64| SoftScore::of((*delta).max(0)))
        .named("capacity shortage");

    let plan = Plan {
        work: vec![
            Work {
                bucket: 0,
                demand: 5,
                enabled: true,
            },
            Work {
                bucket: 1,
                demand: 7,
                enabled: true,
            },
        ],
        capacity: vec![
            Capacity {
                bucket: 0,
                capacity: 3,
            },
            Capacity {
                bucket: 1,
                capacity: 10,
            },
        ],
    };

    assert_eq!(constraint.match_count(&plan), 2);
    assert_eq!(constraint.evaluate(&plan), SoftScore::of(-2));
}

#[test]
fn projected_retracts_previous_outputs_before_update() {
    let mut constraint = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(source(
            work as fn(&Plan) -> &[Work],
            ChangeSource::Descriptor(0),
        ))
        .project(|work: &Work| {
            vec![Entry {
                bucket: work.bucket,
                delta: work.demand,
            }]
        })
        .group_by(
            |entry: &Entry| entry.bucket,
            sum(|entry: &Entry| entry.delta),
        )
        .penalize_with(|delta: &i64| SoftScore::of((*delta).max(0)))
        .named("demand");

    let mut plan = Plan {
        work: vec![Work {
            bucket: 0,
            demand: 5,
            enabled: true,
        }],
        capacity: Vec::new(),
    };

    let mut total = constraint.initialize(&plan);
    assert_eq!(total, SoftScore::of(-5));
    total = total + constraint.on_retract(&plan, 0, 0);
    plan.work[0].demand = 2;
    total = total + constraint.on_insert(&plan, 0, 0);

    assert_eq!(total, SoftScore::of(-2));
    assert_eq!(total, constraint.evaluate(&plan));
}

static PROJECTION_CALLS: AtomicUsize = AtomicUsize::new(0);

fn counted_projection(work: &Work) -> Vec<Entry> {
    PROJECTION_CALLS.fetch_add(1, Ordering::SeqCst);
    vec![Entry {
        bucket: work.bucket,
        delta: work.demand,
    }]
}

#[test]
fn projected_updates_only_the_changed_descriptor_entity() {
    PROJECTION_CALLS.store(0, Ordering::SeqCst);
    let mut constraint = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(source(
            work as fn(&Plan) -> &[Work],
            ChangeSource::Descriptor(0),
        ))
        .project(counted_projection as fn(&Work) -> Vec<Entry>)
        .penalize_with(|entry: &Entry| SoftScore::of(entry.delta))
        .named("counted");

    let mut plan = Plan {
        work: vec![
            Work {
                bucket: 0,
                demand: 1,
                enabled: true,
            },
            Work {
                bucket: 0,
                demand: 2,
                enabled: true,
            },
            Work {
                bucket: 0,
                demand: 3,
                enabled: true,
            },
        ],
        capacity: vec![Capacity {
            bucket: 0,
            capacity: 1,
        }],
    };

    let mut total = constraint.initialize(&plan);
    assert_eq!(PROJECTION_CALLS.load(Ordering::SeqCst), 3);

    total = total + constraint.on_retract(&plan, 0, 1);
    plan.capacity[0].capacity = 2;
    total = total + constraint.on_insert(&plan, 0, 1);
    assert_eq!(PROJECTION_CALLS.load(Ordering::SeqCst), 3);
    assert_eq!(total, constraint.evaluate(&plan));
    PROJECTION_CALLS.store(3, Ordering::SeqCst);

    total = total + constraint.on_retract(&plan, 1, 0);
    plan.work[1].demand = 20;
    total = total + constraint.on_insert(&plan, 1, 0);
    assert_eq!(PROJECTION_CALLS.load(Ordering::SeqCst), 4);
    assert_eq!(total, constraint.evaluate(&plan));
}
