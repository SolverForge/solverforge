use super::support::*;

#[test]
fn projected_self_join_score_director_cached_score_matches_fresh_after_updates() {
    let mut director = ScoreDirector::new(
        projected_asymmetric_self_join_plan(),
        (projected_asymmetric_self_join_constraint(),),
    );
    assert_projected_director_matches_fresh(&mut director);

    for (entity_index, bucket, demand, enabled) in [
        (2, 0, 40, true),
        (1, 0, 20, false),
        (1, 0, 5, true),
        (0, 2, 2, true),
        (2, 0, 30, false),
        (2, 0, 30, true),
    ] {
        director.before_variable_changed(0, entity_index);
        {
            let work = &mut director.working_solution_mut().work[entity_index];
            work.bucket = bucket;
            work.demand = demand;
            work.enabled = enabled;
        }
        director.after_variable_changed(0, entity_index);
        assert_projected_director_matches_fresh(&mut director);
    }
}

#[test]
fn projected_self_join_nested_recording_director_undo_restores_cached_score() {
    let initial_plan = projected_asymmetric_self_join_plan();
    let mut inner = ScoreDirector::new(
        initial_plan.clone(),
        (projected_asymmetric_self_join_constraint(),),
    );
    assert_projected_director_matches_fresh(&mut inner);

    {
        let mut outer = RecordingDirector::new(&mut inner);
        let old_outer_work = outer.working_solution().work[1].clone();
        outer.before_variable_changed(0, 1);
        {
            let work = &mut outer.working_solution_mut().work[1];
            work.bucket = 0;
            work.demand = 5;
            work.enabled = true;
        }
        outer.after_variable_changed(0, 1);
        outer.register_undo(Box::new(move |plan: &mut Plan| {
            plan.work[1] = old_outer_work;
        }));
        assert_eq!(
            outer.calculate_score(),
            fresh_projected_asymmetric_self_join_score(outer.working_solution())
        );

        {
            let mut nested = RecordingDirector::new(&mut outer);
            let old_nested_work = nested.working_solution().work[2].clone();
            nested.before_variable_changed(0, 2);
            {
                let work = &mut nested.working_solution_mut().work[2];
                work.bucket = 0;
                work.demand = 30;
                work.enabled = false;
            }
            nested.after_variable_changed(0, 2);
            nested.register_undo(Box::new(move |plan: &mut Plan| {
                plan.work[2] = old_nested_work;
            }));

            assert_eq!(
                nested.calculate_score(),
                fresh_projected_asymmetric_self_join_score(nested.working_solution())
            );
            nested.undo_changes();
        }

        assert_eq!(
            outer.calculate_score(),
            fresh_projected_asymmetric_self_join_score(outer.working_solution())
        );
        outer.undo_changes();
    }

    assert_eq!(inner.working_solution().work, initial_plan.work);
    assert_projected_director_matches_fresh(&mut inner);
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
        .penalize_with(|_bucket: &usize, delta: &i64| SoftScore::of((*delta).max(0)))
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
