use super::*;

#[test]
fn acyclic_graph_route_change_uses_incremental_descendant_refresh() {
    let mut state = graph_state(4, vec![2, 3, 5, 7], &[(0, 1), (1, 2), (0, 3)]);
    assert_eq!(state.cycle_penalty, 0);
    assert_eq!(state.makespan, 10);

    let mut change = RouteChange::default();
    if state.remove_edge((1, 2)) {
        change.removed_edges.push((1, 2));
    }

    assert_eq!(
        state.refresh_graph_after_route_change(&change),
        GraphRefreshKind::Incremental { visited: 1 }
    );
    assert_eq!(state.cycle_penalty, 0);
    assert_eq!(state.earliest[2], 0);
    assert_eq!(state.earliest[3], 2);
    assert_eq!(state.makespan, 9);
}

#[test]
fn cycle_introducing_route_change_marks_cyclic_without_graph_rebuild() {
    let mut state = graph_state(3, vec![1, 1, 1], &[(0, 1), (1, 2)]);
    assert_eq!(state.cycle_penalty, 0);
    assert_eq!(state.makespan, 3);

    let mut change = RouteChange::default();
    if state.add_edge((2, 0)) {
        change.added_edges.push((2, 0));
    }

    assert_eq!(
        state.refresh_graph_after_route_change(&change),
        GraphRefreshKind::CycleDetected
    );
    assert_eq!(state.cycle_penalty, 3);
    assert_eq!(state.makespan, 0);
}

#[test]
fn cycle_retraction_recovers_cached_acyclic_state_without_graph_rebuild() {
    let mut state = graph_state(3, vec![1, 1, 1], &[(0, 1), (1, 2)]);

    let mut cycle_change = RouteChange::default();
    if state.add_edge((2, 0)) {
        cycle_change.added_edges.push((2, 0));
    }
    assert_eq!(
        state.refresh_graph_after_route_change(&cycle_change),
        GraphRefreshKind::CycleDetected
    );
    assert_eq!(state.cycle_penalty, 3);
    assert_eq!(state.makespan, 0);

    let mut undo_change = RouteChange::default();
    if state.remove_edge((2, 0)) {
        undo_change.removed_edges.push((2, 0));
    }
    assert_eq!(
        state.refresh_graph_after_route_change(&undo_change),
        GraphRefreshKind::CycleRecovered
    );
    assert_eq!(state.cycle_penalty, 0);
    assert_eq!(state.earliest, vec![0, 1, 2]);
    assert_eq!(state.makespan, 3);
}

#[test]
fn cycle_introduced_with_removed_edges_recovers_by_full_rebuild() {
    let mut state = graph_state(3, vec![10, 10, 1], &[(0, 1), (1, 2)]);
    assert_eq!(state.makespan, 21);

    let mut cycle_change = RouteChange::default();
    if state.remove_edge((0, 1)) {
        cycle_change.removed_edges.push((0, 1));
    }
    if state.add_edge((2, 1)) {
        cycle_change.added_edges.push((2, 1));
    }
    assert_eq!(
        state.refresh_graph_after_route_change(&cycle_change),
        GraphRefreshKind::CycleDetected
    );
    assert_eq!(state.cycle_penalty, 3);
    assert!(state.cycle_added_edges.is_empty());

    let mut undo_change = RouteChange::default();
    if state.remove_edge((2, 1)) {
        undo_change.removed_edges.push((2, 1));
    }
    assert_eq!(
        state.refresh_graph_after_route_change(&undo_change),
        GraphRefreshKind::Full
    );
    assert_eq!(state.cycle_penalty, 0);
    assert_eq!(state.earliest, vec![0, 0, 10]);
    assert_eq!(state.makespan, 11);
}

#[test]
fn paired_notifications_preserve_unchanged_edges() {
    let mut constraint = constraint();
    let mut solution = four_task_plan(vec![vec![0, 1, 2]]);
    constraint.initialize(&solution);
    constraint.on_retract(&solution, 0, 0);
    solution.routes[0].tasks = vec![0, 1, 3];
    constraint.on_insert(&solution, 0, 0);
    let state = constraint.state.as_ref().unwrap();
    assert_eq!(state.scratch_route_change.removed_edges, vec![(1, 2)]);
    assert_eq!(state.scratch_route_change.added_edges, vec![(1, 3)]);
    assert_eq!(state.score, constraint.evaluate(&solution));
}

#[test]
fn interleaved_retractions_score_only_inserted_owners() {
    let mut constraint = constraint();
    let mut solution = four_task_plan(vec![vec![0, 1], vec![2, 3]]);
    let initial = constraint.initialize(&solution);
    constraint.on_retract(&solution, 0, 0);
    constraint.on_retract(&solution, 1, 0);
    solution.routes[0].tasks = vec![0, 2];
    let first = initial + constraint.on_insert(&solution, 0, 0);
    solution.routes[1].tasks.clear();
    assert_eq!(first, constraint.evaluate(&solution));
    solution.routes[1].tasks = vec![1, 3];
    let second = first + constraint.on_insert(&solution, 1, 0);
    assert_eq!(second, constraint.evaluate(&solution));
}

#[test]
fn paired_trials_and_undo_match_fresh_scores() {
    let original = vec![0, 1, 2, 3];
    let mut director = ScoreDirector::new(
        four_task_plan(vec![original.clone(), Vec::new()]),
        (constraint(),),
    );
    let baseline = director.calculate_score();
    for route in [vec![0, 1, 3], vec![3, 2, 1, 0], vec![0, 0, 1], Vec::new()] {
        director.before_variable_changed(0, 0);
        director.working_solution_mut().routes[0].tasks = route;
        director.after_variable_changed(0, 0);
        assert_eq!(
            Director::fresh_score(&director),
            Some(director.calculate_score())
        );
        director.before_variable_changed(0, 0);
        director.working_solution_mut().routes[0].tasks = original.clone();
        director.after_variable_changed(0, 0);
        assert_eq!(director.calculate_score(), baseline);
        assert_eq!(Director::fresh_score(&director), Some(baseline));
    }
}

#[test]
fn deferred_seed_generations_survive_refresh_and_clear_only_on_wrap() {
    let mut state = graph_state(4, vec![1; 4], &[(0, 1), (1, 2)]);
    let change = RouteChange {
        removed_edges: vec![(0, 1), (0, 1)],
        ..RouteChange::default()
    };
    state.defer_removed_edges(&change);
    assert_eq!(state.deferred_seeds, vec![1]);
    let generation = state.deferred_seed_stamp;
    state.clear_deferred_seeds();
    assert_eq!(state.deferred_seed_seen[1], generation);
    assert_eq!(state.deferred_seed_stamp, generation);
    state.defer_removed_edges(&change);
    assert_eq!(state.deferred_seeds, vec![1]);
    assert_eq!(state.deferred_seed_stamp, generation + 1);
    state.clear_deferred_seeds();
    state.deferred_seed_stamp = usize::MAX;
    state.deferred_seed_seen.fill(1);
    state.defer_removed_edges(&change);
    assert_eq!(state.deferred_seed_stamp, 1);
    assert_eq!(state.deferred_seeds, vec![1]);
    assert_eq!(state.deferred_seed_seen, vec![0, 1, 0, 0]);
}

#[test]
fn owner_route_replacement_diffs_unchanged_prefix_edges() {
    let constraint = constraint();
    let access = constraint.access();
    let mut state = ListPrecedenceState::new(4, 1, vec![1, 1, 1, 1]);
    let mut change = state.take_route_change_scratch();
    state.replace_owner_route_scratch_with(
        &mut change,
        &four_task_plan(vec![vec![0, 1, 2]]),
        0,
        access,
    );
    state.refresh_score_full();

    state.replace_owner_route_scratch_with(
        &mut change,
        &four_task_plan(vec![vec![0, 1, 3]]),
        0,
        access,
    );
    change.added_edges.sort_unstable();
    change.removed_edges.sort_unstable();

    assert_eq!(change.removed_edges, vec![(1, 2)]);
    assert_eq!(change.added_edges, vec![(1, 3)]);
    assert_eq!(state.assigned_counts, vec![1, 1, 0, 1]);
    assert_eq!(state.owner_edges[0], vec![(0, 1), (1, 3)]);
}

#[test]
fn cyclic_state_with_unmatched_change_uses_full_graph_refresh() {
    let mut state = graph_state(4, vec![1, 1, 1, 1], &[(0, 1), (1, 2)]);

    let mut cycle_change = RouteChange::default();
    if state.add_edge((2, 0)) {
        cycle_change.added_edges.push((2, 0));
    }
    assert_eq!(
        state.refresh_graph_after_route_change(&cycle_change),
        GraphRefreshKind::CycleDetected
    );

    let mut unrelated_change = RouteChange::default();
    if state.add_edge((2, 3)) {
        unrelated_change.added_edges.push((2, 3));
    }
    assert_eq!(
        state.refresh_graph_after_route_change(&unrelated_change),
        GraphRefreshKind::Full
    );
    assert_eq!(state.cycle_penalty, 4);
    assert_eq!(state.makespan, 0);
}
