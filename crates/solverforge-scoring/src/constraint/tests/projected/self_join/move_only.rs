use super::*;

#[test]
fn projected_self_join_accepts_move_only_rows_and_keys() {
    let mut constraint = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(source(
            work as fn(&Plan) -> &[Work],
            ChangeSource::Descriptor(0),
        ))
        .project(MoveOnlyWorkEntryProjection)
        .join(equal(|entry: &MoveOnlyEntry| MoveOnlyBucket(entry.bucket)))
        .filter(|left: &MoveOnlyEntry, right: &MoveOnlyEntry| left.delta < right.delta)
        .penalize(|_left: &MoveOnlyEntry, _right: &MoveOnlyEntry| SoftScore::of(1))
        .named("projected move-only duplicate bucket");

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
fn projected_group_by_accepts_move_only_collector_values() {
    let mut constraint = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(source(
            work as fn(&Plan) -> &[Work],
            ChangeSource::Descriptor(0),
        ))
        .project(WorkEntryProjection)
        .group_by(|entry: &Entry| entry.bucket, MoveOnlyDeltaCollector)
        .penalize(|_bucket: &usize, delta: &i64| SoftScore::of((*delta).max(0)))
        .named("projected move-only collector value");

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

#[test]
fn projected_group_by_accepts_move_only_payloads() {
    let mut constraint = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(source(
            work as fn(&Plan) -> &[Work],
            ChangeSource::Descriptor(0),
        ))
        .project(MoveOnlyWorkEntryProjection)
        .group_by(|entry: &MoveOnlyEntry| entry.bucket, MoveOnlyDeltaCollector)
        .penalize(|_bucket: &usize, delta: &i64| SoftScore::of((*delta).max(0)))
        .named("projected move-only payload");

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
