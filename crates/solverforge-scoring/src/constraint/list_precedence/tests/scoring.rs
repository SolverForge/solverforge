use super::*;

#[test]
fn evaluates_fixed_and_list_precedence_makespan() {
    let constraint = constraint();

    assert_eq!(
        constraint.evaluate(&plan(vec![vec![0], vec![1]])),
        HardSoftScore::of(0, -5)
    );
}

#[test]
fn partial_cycle_penalizes_whole_precedence_schedule() {
    let state = graph_state(4, vec![1, 1, 1, 10], &[(0, 1), (1, 0)]);

    assert_eq!(state.cycle_penalty, 4);
    assert_eq!(state.makespan, 0);
    assert_eq!(state.score, HardSoftScore::of(-8, 0));
}

#[test]
fn penalizes_cycles_incrementally() {
    let mut director = ScoreDirector::new(plan(vec![vec![0, 1]]), (constraint(),));

    assert_eq!(director.calculate_score(), HardSoftScore::of(0, -5));
    director.before_variable_changed(0, 0);
    director.working_solution_mut().routes[0].tasks = vec![1, 0];
    director.after_variable_changed(0, 0);

    let score = director.calculate_score();
    assert_eq!(score, HardSoftScore::of(-2, 0));
    assert_eq!(Director::fresh_score(&director), Some(score));
}

#[test]
fn repeated_route_updates_keep_incremental_score_fresh() {
    let mut director = ScoreDirector::new(plan(vec![vec![0], vec![1]]), (constraint(),));

    let score = director.calculate_score();
    assert_eq!(Director::fresh_score(&director), Some(score));

    for (owner, tasks) in [
        (0, vec![0, 1]),
        (1, Vec::new()),
        (0, vec![1, 0]),
        (1, vec![0, 1]),
    ] {
        director.before_variable_changed(0, owner);
        director.working_solution_mut().routes[owner].tasks = tasks;
        director.after_variable_changed(0, owner);

        let score = director.calculate_score();
        assert_eq!(Director::fresh_score(&director), Some(score));
    }
}

#[test]
fn retract_removes_cached_owner_route_before_insert() {
    let mut constraint = constraint();
    let mut solution = plan(vec![vec![0], vec![1]]);
    let mut score = constraint.initialize(&solution);
    assert_eq!(score, HardSoftScore::of(0, -5));

    // Retraction defers the re-timing to the matching insert: the
    // event itself contributes no delta, the insert re-times the union
    // of pending removals plus its own additions and carries the whole
    // after-before delta.
    assert_eq!(constraint.on_retract(&solution, 0, 0), HardSoftScore::ZERO);

    solution.routes[0].tasks = vec![0, 1];
    score = score + constraint.on_insert(&solution, 0, 0);

    assert_eq!(score, HardSoftScore::of(-1, -5));
    assert_eq!(constraint.evaluate(&solution), score);
}

#[test]
fn incremental_route_update_only_reads_changed_owner_route() {
    reset_access_counts();
    let mut director = ScoreDirector::new(plan(vec![vec![0], vec![1]]), (counted_constraint(),));

    assert_eq!(director.calculate_score(), HardSoftScore::of(0, -5));
    assert_eq!(NODE_COUNT_CALLS.load(Ordering::Relaxed), 1);
    assert_eq!(NODE_DURATION_CALLS.load(Ordering::Relaxed), 2);
    assert_eq!(FIXED_SUCCESSOR_CALLS.load(Ordering::Relaxed), 2);
    assert_eq!(OWNER_COUNT_CALLS.load(Ordering::Relaxed), 1);
    assert_eq!(LIST_LEN_CALLS[0].load(Ordering::Relaxed), 1);
    assert_eq!(LIST_LEN_CALLS[1].load(Ordering::Relaxed), 1);
    assert_eq!(LIST_GET_CALLS[0].load(Ordering::Relaxed), 1);
    assert_eq!(LIST_GET_CALLS[1].load(Ordering::Relaxed), 1);

    reset_access_counts();
    director.before_variable_changed(0, 0);
    director.working_solution_mut().routes[0].tasks = vec![0, 1];
    director.after_variable_changed(0, 0);

    assert_eq!(director.calculate_score(), HardSoftScore::of(-1, -5));
    assert_eq!(NODE_COUNT_CALLS.load(Ordering::Relaxed), 0);
    assert_eq!(NODE_DURATION_CALLS.load(Ordering::Relaxed), 0);
    assert_eq!(FIXED_SUCCESSOR_CALLS.load(Ordering::Relaxed), 0);
    assert_eq!(OWNER_COUNT_CALLS.load(Ordering::Relaxed), 0);
    assert_eq!(LIST_LEN_CALLS[0].load(Ordering::Relaxed), 1);
    assert_eq!(LIST_LEN_CALLS[1].load(Ordering::Relaxed), 0);
    assert_eq!(LIST_GET_CALLS[0].load(Ordering::Relaxed), 2);
    assert_eq!(LIST_GET_CALLS[1].load(Ordering::Relaxed), 0);
    assert_eq!(
        Director::fresh_score(&director),
        Some(director.calculate_score())
    );
}

#[test]
fn penalizes_missing_duplicate_and_wrong_owner_assignments() {
    let constraint = owner_constraint();

    assert_eq!(
        constraint.evaluate(&plan(vec![vec![0, 1], vec![1]])),
        HardSoftScore::of(-2, -5)
    );
}

#[test]
fn ignores_unrelated_descriptor_changes() {
    let mut director = ScoreDirector::new(plan(vec![vec![0], vec![1]]), (constraint(),));

    let score = director.calculate_score();
    director.before_variable_changed(1, 0);
    director.working_solution_mut().routes[0].tasks = vec![1, 0];
    director.after_variable_changed(1, 0);

    assert_eq!(director.calculate_score(), score);
}
