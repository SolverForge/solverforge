use super::support::*;

#[test]
fn projected_allows_zero_and_multiple_outputs() {
    let constraint = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(source(
            work as fn(&Plan) -> &[Work],
            ChangeSource::Descriptor(0),
        ))
        .project(WorkTwoEntries)
        .penalize(|entry: &Entry| SoftScore::of(entry.delta))
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
        .penalize(|_bucket: &usize, delta: &i64| SoftScore::of((*delta).max(0)))
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
        .penalize(|_left: &Entry, _right: &Entry| SoftScore::of(1))
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
        .penalize(|_left: &Entry, _right: &Entry| SoftScore::of(1))
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
fn projected_self_join_preserves_projection_order_when_reusing_slots() {
    let mut constraint = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(source(
            work as fn(&Plan) -> &[Work],
            ChangeSource::Descriptor(0),
        ))
        .project(OrderedWorkEntries)
        .join(equal(|entry: &Entry| entry.bucket))
        .filter(|left: &Entry, right: &Entry| left.delta < right.delta)
        .penalize(|left: &Entry, right: &Entry| SoftScore::of(left.delta * 10 + right.delta))
        .named("projected ordered duplicate bucket");

    let plan = Plan {
        work: vec![Work {
            bucket: 0,
            demand: 1,
            enabled: true,
        }],
        capacity: Vec::new(),
    };

    let mut total = constraint.initialize(&plan);
    let initial_row_storage_len = constraint.debug_row_storage_len();
    assert_eq!(total, SoftScore::of(-21));
    assert_eq!(total, constraint.evaluate(&plan));

    for _ in 0..4 {
        total = total + constraint.on_retract(&plan, 0, 0);
        total = total + constraint.on_insert(&plan, 0, 0);

        assert_eq!(total, SoftScore::of(-21));
        assert_eq!(total, constraint.evaluate(&plan));
        assert_eq!(constraint.debug_row_storage_len(), initial_row_storage_len);
        assert_eq!(constraint.debug_free_row_count(), 0);
    }
}

#[test]
fn projected_self_join_reuses_slots_across_cardinality_changes() {
    let mut constraint = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(source(
            work as fn(&Plan) -> &[Work],
            ChangeSource::Descriptor(0),
        ))
        .project(OptionalSecondWorkEntry)
        .join(equal(|entry: &Entry| entry.bucket))
        .filter(|left: &Entry, right: &Entry| left.delta < right.delta)
        .penalize(|left: &Entry, right: &Entry| SoftScore::of(left.delta * 10 + right.delta))
        .named("projected cardinality changing duplicate bucket");

    let mut plan = Plan {
        work: vec![Work {
            bucket: 0,
            demand: 2,
            enabled: true,
        }],
        capacity: Vec::new(),
    };

    let mut total = constraint.initialize(&plan);
    assert_eq!(total, SoftScore::of(-32));
    assert_eq!(constraint.debug_row_storage_len(), 2);

    total = total + constraint.on_retract(&plan, 0, 0);
    plan.work[0].enabled = false;
    total = total + constraint.on_insert(&plan, 0, 0);
    assert_eq!(total, SoftScore::ZERO);
    assert_eq!(total, constraint.evaluate(&plan));
    assert_eq!(constraint.debug_row_storage_len(), 2);
    assert_eq!(constraint.debug_free_row_count(), 1);

    total = total + constraint.on_retract(&plan, 0, 0);
    plan.work[0].enabled = true;
    total = total + constraint.on_insert(&plan, 0, 0);
    assert_eq!(total, SoftScore::of(-32));
    assert_eq!(total, constraint.evaluate(&plan));
    assert_eq!(constraint.debug_row_storage_len(), 2);
    assert_eq!(constraint.debug_free_row_count(), 0);
}

#[test]
fn projected_self_join_accepts_non_clone_rows_and_keys() {
    let mut constraint = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(source(
            work as fn(&Plan) -> &[Work],
            ChangeSource::Descriptor(0),
        ))
        .project(NonCloneWorkEntryProjection)
        .join(equal(|entry: &NonCloneEntry| NonCloneBucket(entry.bucket)))
        .filter(|left: &NonCloneEntry, right: &NonCloneEntry| left.delta < right.delta)
        .penalize(|_left: &NonCloneEntry, _right: &NonCloneEntry| SoftScore::of(1))
        .named("projected non-clone duplicate bucket");

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
    assert_eq!(total, SoftScore::of(-1));

    total = total + constraint.on_retract(&plan, 2, 0);
    plan.work[2].bucket = 0;
    total = total + constraint.on_insert(&plan, 2, 0);

    assert_eq!(total, SoftScore::of(-3));
    assert_eq!(total, constraint.evaluate(&plan));
}

#[test]
fn projected_group_by_accepts_non_clone_collector_values() {
    let mut constraint = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(source(
            work as fn(&Plan) -> &[Work],
            ChangeSource::Descriptor(0),
        ))
        .project(WorkEntryProjection)
        .group_by(|entry: &Entry| entry.bucket, NonCloneDeltaCollector)
        .penalize(|_bucket: &usize, delta: &i64| SoftScore::of((*delta).max(0)))
        .named("projected non-clone collector value");

    let mut plan = Plan {
        work: vec![
            Work {
                bucket: 0,
                demand: 5,
                enabled: true,
            },
            Work {
                bucket: 0,
                demand: 7,
                enabled: true,
            },
        ],
        capacity: Vec::new(),
    };

    let mut total = constraint.initialize(&plan);
    assert_eq!(total, SoftScore::of(-12));

    total = total + constraint.on_retract(&plan, 1, 0);
    plan.work[1].demand = -3;
    total = total + constraint.on_insert(&plan, 1, 0);

    assert_eq!(total, SoftScore::of(-2));
    assert_eq!(total, constraint.evaluate(&plan));
}
