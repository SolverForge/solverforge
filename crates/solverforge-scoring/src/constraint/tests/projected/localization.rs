use super::support::*;

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
