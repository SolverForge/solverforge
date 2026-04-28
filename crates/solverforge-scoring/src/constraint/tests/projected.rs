use std::sync::atomic::{AtomicUsize, Ordering};

use solverforge_core::score::SoftScore;

use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::collection_extract::{source, ChangeSource};
use crate::stream::collector::sum;
use crate::stream::joiner::equal;
use crate::stream::{ConstraintFactory, Projection, ProjectionSink};

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

struct WorkTwoEntries;

impl Projection<Work> for WorkTwoEntries {
    type Out = Entry;
    const MAX_EMITS: usize = 2;

    fn project<Sink>(&self, work: &Work, out: &mut Sink)
    where
        Sink: ProjectionSink<Self::Out>,
    {
        if !work.enabled {
            return;
        }
        out.emit(Entry {
            bucket: work.bucket,
            delta: work.demand,
        });
        out.emit(Entry {
            bucket: work.bucket + 1,
            delta: work.demand,
        });
    }
}

struct WorkEntryProjection;

impl Projection<Work> for WorkEntryProjection {
    type Out = Entry;
    const MAX_EMITS: usize = 1;

    fn project<Sink>(&self, work: &Work, out: &mut Sink)
    where
        Sink: ProjectionSink<Self::Out>,
    {
        out.emit(Entry {
            bucket: work.bucket,
            delta: work.demand,
        });
    }
}

struct CapacityEntryProjection;

impl Projection<Capacity> for CapacityEntryProjection {
    type Out = Entry;
    const MAX_EMITS: usize = 1;

    fn project<Sink>(&self, capacity: &Capacity, out: &mut Sink)
    where
        Sink: ProjectionSink<Self::Out>,
    {
        out.emit(Entry {
            bucket: capacity.bucket,
            delta: -capacity.capacity,
        });
    }
}

struct CountedProjection;

impl Projection<Work> for CountedProjection {
    type Out = Entry;
    const MAX_EMITS: usize = 1;

    fn project<Sink>(&self, work: &Work, out: &mut Sink)
    where
        Sink: ProjectionSink<Self::Out>,
    {
        PROJECTION_CALLS.fetch_add(1, Ordering::SeqCst);
        out.emit(Entry {
            bucket: work.bucket,
            delta: work.demand,
        });
    }
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
        .project(WorkTwoEntries)
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
        .project(WorkEntryProjection)
        .merge(
            ConstraintFactory::<Plan, SoftScore>::new()
                .for_each(source(
                    capacity as fn(&Plan) -> &[Capacity],
                    ChangeSource::Descriptor(1),
                ))
                .project(CapacityEntryProjection),
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
fn projected_rows_can_self_join_by_key() {
    let mut constraint = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(source(
            work as fn(&Plan) -> &[Work],
            ChangeSource::Descriptor(0),
        ))
        .project(WorkEntryProjection)
        .join(equal(|entry: &Entry| entry.bucket))
        .filter(|left: &Entry, right: &Entry| left.delta < right.delta)
        .penalize_with(|_left: &Entry, _right: &Entry| SoftScore::of(1))
        .named("projected duplicate bucket");

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
                bucket: 1,
                demand: 3,
                enabled: true,
            },
        ],
        capacity: Vec::new(),
    };

    let mut total = constraint.initialize(&plan);
    assert_eq!(constraint.match_count(&plan), 1);
    assert_eq!(total, SoftScore::of(-1));

    total = total + constraint.on_retract(&plan, 2, 0);
    plan.work[2].bucket = 0;
    total = total + constraint.on_insert(&plan, 2, 0);

    assert_eq!(constraint.match_count(&plan), 3);
    assert_eq!(total, SoftScore::of(-3));
    assert_eq!(total, constraint.evaluate(&plan));
}

#[test]
fn projected_self_join_reuses_row_slots_after_repeated_updates() {
    let mut constraint = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(source(
            work as fn(&Plan) -> &[Work],
            ChangeSource::Descriptor(0),
        ))
        .project(WorkEntryProjection)
        .join(equal(|entry: &Entry| entry.bucket))
        .filter(|left: &Entry, right: &Entry| left.delta < right.delta)
        .penalize_with(|_left: &Entry, _right: &Entry| SoftScore::of(1))
        .named("projected duplicate bucket");

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
                bucket: 1,
                demand: 3,
                enabled: true,
            },
        ],
        capacity: Vec::new(),
    };

    let mut total = constraint.initialize(&plan);
    let initial_row_storage_len = constraint.debug_row_storage_len();
    assert_eq!(initial_row_storage_len, 3);

    for bucket in [0, 1, 2, 0, 3, 0] {
        total = total + constraint.on_retract(&plan, 2, 0);
        plan.work[2].bucket = bucket;
        total = total + constraint.on_insert(&plan, 2, 0);

        assert_eq!(total, constraint.evaluate(&plan));
        assert_eq!(constraint.debug_row_storage_len(), initial_row_storage_len);
        assert_eq!(constraint.debug_free_row_count(), 0);
    }
}

#[test]
fn projected_merged_descriptor_sources_update_only_owning_slot() {
    let mut constraint = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(source(
            work as fn(&Plan) -> &[Work],
            ChangeSource::Descriptor(0),
        ))
        .project(WorkEntryProjection)
        .merge(
            ConstraintFactory::<Plan, SoftScore>::new()
                .for_each(source(
                    capacity as fn(&Plan) -> &[Capacity],
                    ChangeSource::Descriptor(1),
                ))
                .project(CapacityEntryProjection),
        )
        .group_by(
            |entry: &Entry| entry.bucket,
            sum(|entry: &Entry| entry.delta),
        )
        .penalize_with(|delta: &i64| SoftScore::of((*delta).max(0)))
        .named("capacity shortage");

    let mut plan = Plan {
        work: vec![Work {
            bucket: 0,
            demand: 5,
            enabled: true,
        }],
        capacity: vec![Capacity {
            bucket: 0,
            capacity: 3,
        }],
    };

    let mut total = constraint.initialize(&plan);
    assert_eq!(total, SoftScore::of(-2));

    total = total + constraint.on_retract(&plan, 0, 1);
    plan.capacity[0].capacity = 8;
    total = total + constraint.on_insert(&plan, 0, 1);

    assert_eq!(total, SoftScore::of(0));
    assert_eq!(total, constraint.evaluate(&plan));
}

#[test]
fn projected_merged_descriptor_sources_keep_same_entity_index_slots_distinct() {
    let mut constraint = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(source(
            work as fn(&Plan) -> &[Work],
            ChangeSource::Descriptor(0),
        ))
        .project(WorkEntryProjection)
        .merge(
            ConstraintFactory::<Plan, SoftScore>::new()
                .for_each(source(
                    capacity as fn(&Plan) -> &[Capacity],
                    ChangeSource::Descriptor(1),
                ))
                .project(CapacityEntryProjection),
        )
        .penalize_with(|entry: &Entry| SoftScore::of(entry.delta))
        .named("merged projected rows");

    let mut plan = Plan {
        work: vec![Work {
            bucket: 0,
            demand: 5,
            enabled: true,
        }],
        capacity: vec![Capacity {
            bucket: 0,
            capacity: 3,
        }],
    };

    let mut total = constraint.initialize(&plan);
    assert_eq!(total, SoftScore::of(-2));

    total = total + constraint.on_retract(&plan, 0, 0);
    plan.work[0].demand = 8;
    total = total + constraint.on_insert(&plan, 0, 0);

    assert_eq!(total, SoftScore::of(-5));
    assert_eq!(total, constraint.evaluate(&plan));
}

#[test]
#[should_panic(expected = "cannot localize entity indexes")]
fn projected_unknown_source_panics_on_localized_callback() {
    let mut constraint = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(work as fn(&Plan) -> &[Work])
        .project(WorkEntryProjection)
        .penalize_with(|entry: &Entry| SoftScore::of(entry.delta))
        .named("unknown projected");

    let plan = Plan {
        work: vec![Work {
            bucket: 0,
            demand: 5,
            enabled: true,
        }],
        capacity: Vec::new(),
    };

    constraint.initialize(&plan);
    constraint.on_retract(&plan, 0, 0);
}

#[test]
fn projected_retracts_previous_outputs_before_update() {
    let mut constraint = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(source(
            work as fn(&Plan) -> &[Work],
            ChangeSource::Descriptor(0),
        ))
        .project(WorkEntryProjection)
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

#[test]
fn projected_updates_only_the_changed_descriptor_entity() {
    PROJECTION_CALLS.store(0, Ordering::SeqCst);
    let mut constraint = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(source(
            work as fn(&Plan) -> &[Work],
            ChangeSource::Descriptor(0),
        ))
        .project(CountedProjection)
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
