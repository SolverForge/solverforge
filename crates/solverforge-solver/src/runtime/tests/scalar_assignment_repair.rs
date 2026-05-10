use crate::builder::selector::GroupedScalarSelector;
use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::{MoveCursor, MoveSelector};

fn scalar_assignment_selector(
    value_candidate_limit: Option<usize>,
    max_moves_per_step: Option<usize>,
    require_hard_improvement: bool,
) -> GroupedScalarSelector<CoveragePlan> {
    scalar_assignment_selector_with_model(
        assignment_model(),
        value_candidate_limit,
        max_moves_per_step,
        require_hard_improvement,
    )
}

fn scalar_assignment_selector_with_model(
    model: RuntimeModel<CoveragePlan, usize, DefaultMeter, DefaultMeter>,
    value_candidate_limit: Option<usize>,
    max_moves_per_step: Option<usize>,
    require_hard_improvement: bool,
) -> GroupedScalarSelector<CoveragePlan> {
    GroupedScalarSelector::new(
        model.scalar_groups()[0].clone(),
        value_candidate_limit,
        max_moves_per_step,
        require_hard_improvement,
    )
}

fn repair_move_results(
    plan: &CoveragePlan,
    selector: &GroupedScalarSelector<CoveragePlan>,
) -> Vec<CoveragePlan> {
    let director = CoverageDirector {
        working_solution: plan.clone(),
        descriptor: coverage_plan_descriptor(),
    };
    let mut cursor = selector.open_cursor(&director);
    let mut results = Vec::new();
    while let Some(id) = cursor.next_candidate() {
        let mov = cursor.take_candidate(id);
        let mut trial = CoverageDirector {
            working_solution: plan.clone(),
            descriptor: coverage_plan_descriptor(),
        };
        assert!(mov.is_doable(&trial));
        mov.do_move(&mut trial);
        trial.calculate_score();
        results.push(trial.working_solution);
    }
    results
}

#[test]
fn scalar_assignment_selector_emits_only_hard_improving_required_moves() {
    let plan = coverage_plan(
        2,
        vec![
            coverage_slot(true, 0, None, &[0]),
            coverage_slot(true, 0, Some(0), &[0, 1]),
        ],
    );
    let selector = scalar_assignment_selector(None, Some(8), true);
    let director = CoverageDirector {
        working_solution: plan.clone(),
        descriptor: coverage_plan_descriptor(),
    };
    let mut cursor = selector.open_cursor(&director);
    let mut emitted = 0;

    while let Some(id) = cursor.next_candidate() {
        let mov = cursor.take_candidate(id);
        let mut trial = CoverageDirector {
            working_solution: plan.clone(),
            descriptor: coverage_plan_descriptor(),
        };
        let current = trial.calculate_score();
        assert!(mov.requires_hard_improvement());
        assert!(mov.is_doable(&trial));
        mov.do_move(&mut trial);
        let next = trial.calculate_score();
        assert!(next.hard() >= current.hard());
        emitted += 1;
    }

    assert!(emitted > 0);
}

#[test]
fn scalar_assignment_repair_falls_back_to_moving_preferred_keeper() {
    let plan = coverage_plan(
        2,
        vec![
            coverage_slot(true, 0, Some(0), &[0, 1]),
            coverage_slot(true, 0, Some(0), &[0]),
        ],
    );
    let selector = scalar_assignment_selector(None, Some(8), true);
    let results = repair_move_results(&plan, &selector);

    assert!(results.iter().any(|result| {
        result.slots[0].assigned == Some(1)
            && result.slots[1].assigned == Some(0)
            && result.score == Some(HardSoftScore::of(0, 0))
    }));
}

#[test]
fn scalar_assignment_repair_orders_capacity_conflict_groups_by_coverage_order() {
    let plan = coverage_plan(
        2,
        vec![
            coverage_slot(true, 0, Some(1), &[1]),
            coverage_slot(false, 0, Some(1), &[1]),
            coverage_slot(true, 0, Some(0), &[0]),
            coverage_slot(false, 0, Some(0), &[0]),
        ],
    );
    let selector = scalar_assignment_selector(None, Some(1), false);
    let results = repair_move_results(&plan, &selector);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].slots[1].assigned, None);
    assert_eq!(results[0].slots[3].assigned, Some(0));
}

#[test]
fn scalar_assignment_repair_honors_value_candidate_limit_when_relocating_blocker() {
    let plan = coverage_plan(
        2,
        vec![
            coverage_slot(true, 0, None, &[0]),
            coverage_slot(true, 0, Some(0), &[0, 1]),
        ],
    );
    let limited_selector = scalar_assignment_selector(Some(1), Some(8), false);
    let unlimited_selector = scalar_assignment_selector(None, Some(8), false);

    assert!(repair_move_results(&plan, &limited_selector).is_empty());

    let unlimited_results = repair_move_results(&plan, &unlimited_selector);
    assert!(unlimited_results.iter().any(|result| {
        result.slots[0].assigned == Some(0) && result.slots[1].assigned == Some(1)
    }));
}

#[test]
fn scalar_assignment_repair_uses_group_cap_when_selector_cap_is_omitted() {
    let model = assignment_model_with_limits(ScalarGroupLimits {
        max_moves_per_step: Some(1),
        max_augmenting_depth: Some(3),
        ..ScalarGroupLimits::new()
    });
    let plan = coverage_plan(
        3,
        vec![
            coverage_slot(true, 0, None, &[0, 1, 2]),
            coverage_slot(true, 0, None, &[0, 1, 2]),
            coverage_slot(true, 0, None, &[0, 1, 2]),
        ],
    );
    let selector = scalar_assignment_selector_with_model(model, None, None, false);

    assert_eq!(repair_move_results(&plan, &selector).len(), 1);
}

#[test]
fn scalar_assignment_selector_cap_overrides_group_cap() {
    let model = assignment_model_with_limits(ScalarGroupLimits {
        max_moves_per_step: Some(1),
        max_augmenting_depth: Some(3),
        ..ScalarGroupLimits::new()
    });
    let plan = coverage_plan(
        3,
        vec![
            coverage_slot(true, 0, None, &[0, 1, 2]),
            coverage_slot(true, 0, None, &[0, 1, 2]),
            coverage_slot(true, 0, None, &[0, 1, 2]),
        ],
    );
    let selector = scalar_assignment_selector_with_model(model, None, Some(2), false);

    assert_eq!(repair_move_results(&plan, &selector).len(), 2);
}

#[test]
fn scalar_assignment_selector_cap_rotates_across_cursor_openings() {
    let model = assignment_model_with_limits(ScalarGroupLimits {
        max_moves_per_step: Some(1),
        max_augmenting_depth: Some(3),
        ..ScalarGroupLimits::new()
    });
    let plan = coverage_plan(
        3,
        vec![
            coverage_slot(true, 0, None, &[0, 1, 2]),
            coverage_slot(true, 1, None, &[0, 1, 2]),
            coverage_slot(true, 2, None, &[0, 1, 2]),
        ],
    );
    let selector = scalar_assignment_selector_with_model(model, None, None, false);

    let first_results = repair_move_results(&plan, &selector);
    let second_results = repair_move_results(&plan, &selector);
    let third_results = repair_move_results(&plan, &selector);

    assert_eq!(first_results.len(), 1);
    assert_eq!(second_results.len(), 1);
    assert_eq!(third_results.len(), 1);
    assert_eq!(first_results[0].slots[0].assigned, Some(0));
    assert_eq!(second_results[0].slots[1].assigned, Some(0));
    assert_eq!(third_results[0].slots[2].assigned, Some(0));
}

#[test]
fn scalar_assignment_rematch_emits_bounded_sequence_swap() {
    let model = assignment_model_with_limits(ScalarGroupLimits {
        max_rematch_size: Some(2),
        ..ScalarGroupLimits::new()
    });
    let plan = coverage_plan(
        2,
        vec![
            coverage_slot(true, 0, Some(0), &[0, 1]),
            coverage_slot(true, 0, Some(1), &[0, 1]),
        ],
    );
    let selector = scalar_assignment_selector_with_model(model, None, Some(1), false);
    let results = repair_move_results(&plan, &selector);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].slots[0].assigned, Some(1));
    assert_eq!(results[0].slots[1].assigned, Some(0));
}

#[test]
fn scalar_assignment_sequence_window_exchanges_across_sequences() {
    let model = assignment_model_with_limits(ScalarGroupLimits {
        max_rematch_size: Some(2),
        ..ScalarGroupLimits::new()
    });
    let plan = coverage_plan(
        2,
        vec![
            coverage_slot(true, 0, Some(0), &[0, 1]),
            coverage_slot(true, 1, Some(1), &[0, 1]),
        ],
    );
    let selector = scalar_assignment_selector_with_model(model, None, Some(8), false);
    let director = CoverageDirector {
        working_solution: plan.clone(),
        descriptor: coverage_plan_descriptor(),
    };
    let mut cursor = selector.open_cursor(&director);
    let mut sequence_move = None;
    while let Some(id) = cursor.next_candidate() {
        let mov = cursor.take_candidate(id);
        if format!("{mov:?}").contains("scalar_assignment_sequence_window") {
            sequence_move = Some(mov);
            break;
        }
    }
    let mov = sequence_move.expect("position metadata should expose a sequence-window exchange");

    let mut trial = CoverageDirector {
        working_solution: plan,
        descriptor: coverage_plan_descriptor(),
    };
    assert!(mov.is_doable(&trial));
    mov.do_move(&mut trial);

    assert_eq!(trial.working_solution.slots[0].assigned, Some(1));
    assert_eq!(trial.working_solution.slots[1].assigned, Some(0));
}

#[test]
fn scalar_assignment_augmenting_rematch_emits_bounded_rotation() {
    let model = assignment_model_with_limits(ScalarGroupLimits {
        max_rematch_size: Some(3),
        ..ScalarGroupLimits::new()
    });
    let plan = coverage_plan(
        3,
        vec![
            coverage_slot(true, 0, Some(0), &[0, 1, 2]),
            coverage_slot(true, 1, Some(1), &[0, 1, 2]),
            coverage_slot(true, 2, Some(2), &[0, 1, 2]),
        ],
    );
    let selector = scalar_assignment_selector_with_model(model, None, Some(16), false);
    let director = CoverageDirector {
        working_solution: plan.clone(),
        descriptor: coverage_plan_descriptor(),
    };
    let mut cursor = selector.open_cursor(&director);
    let mut rematch_move = None;
    while let Some(id) = cursor.next_candidate() {
        let mov = cursor.take_candidate(id);
        if format!("{mov:?}").contains("scalar_assignment_augmenting_rematch") {
            rematch_move = Some(mov);
            break;
        }
    }
    let mov = rematch_move.expect("bounded augmenting rematch should expose a rotation");

    let mut trial = CoverageDirector {
        working_solution: plan,
        descriptor: coverage_plan_descriptor(),
    };
    assert!(mov.is_doable(&trial));
    mov.do_move(&mut trial);
    assert_eq!(trial.working_solution.slots[0].assigned, Some(1));
    assert_eq!(trial.working_solution.slots[1].assigned, Some(2));
    assert_eq!(trial.working_solution.slots[2].assigned, Some(0));
}

#[test]
fn scalar_assignment_ejection_reinsert_emits_bounded_multi_slot_rebuild() {
    let model = assignment_model_with_limits(ScalarGroupLimits {
        max_augmenting_depth: Some(3),
        max_rematch_size: Some(3),
        ..ScalarGroupLimits::new()
    });
    let plan = coverage_plan(
        3,
        vec![
            coverage_slot(true, 0, Some(0), &[0, 1, 2]),
            coverage_slot(true, 1, Some(1), &[0, 1, 2]),
            coverage_slot(true, 2, Some(2), &[0, 1, 2]),
        ],
    );
    let selector = scalar_assignment_selector_with_model(model, None, Some(32), false);
    let director = CoverageDirector {
        working_solution: plan.clone(),
        descriptor: coverage_plan_descriptor(),
    };
    let mut cursor = selector.open_cursor(&director);
    let mut ejection_move = None;
    while let Some(id) = cursor.next_candidate() {
        let mov = cursor.take_candidate(id);
        if format!("{mov:?}").contains("scalar_assignment_ejection_reinsert") {
            ejection_move = Some(mov);
            break;
        }
    }
    let mov = ejection_move.expect("grouped assignment should expose bounded ejection/reinsert");

    let mut trial = CoverageDirector {
        working_solution: plan.clone(),
        descriptor: coverage_plan_descriptor(),
    };
    assert!(mov.is_doable(&trial));
    mov.do_move(&mut trial);
    let changed = trial
        .working_solution
        .slots
        .iter()
        .zip(plan.slots.iter())
        .filter(|(left, right)| left.assigned != right.assigned)
        .count();

    assert!(changed >= 2);
    assert!(
        trial
            .working_solution
            .slots
            .iter()
            .all(|slot| slot.assigned.is_some())
    );
}

#[test]
fn scalar_assignment_selector_emits_independent_pair_reassignment() {
    let plan = coverage_plan(
        2,
        vec![
            coverage_slot(true, 0, Some(0), &[0, 1]),
            coverage_slot(true, 1, Some(0), &[0, 1]),
        ],
    );
    let selector = scalar_assignment_selector(None, Some(1), false);
    let director = CoverageDirector {
        working_solution: plan.clone(),
        descriptor: coverage_plan_descriptor(),
    };
    let mut cursor = selector.open_cursor(&director);
    let first = cursor
        .next_candidate()
        .expect("independent pair reassignment should be exposed");
    assert!(cursor.next_candidate().is_none());
    let mov = cursor.take_candidate(first);
    assert!(format!("{mov:?}").contains("scalar_assignment_pair_reassignment"));

    let mut trial = CoverageDirector {
        working_solution: plan,
        descriptor: coverage_plan_descriptor(),
    };
    assert!(mov.is_doable(&trial));
    mov.do_move(&mut trial);

    assert_eq!(trial.working_solution.slots[0].assigned, Some(1));
    assert_eq!(trial.working_solution.slots[1].assigned, Some(1));
}

#[test]
fn scalar_assignment_rematch_orders_sequence_groups_deterministically() {
    let model = assignment_model_with_limits(ScalarGroupLimits {
        max_rematch_size: Some(2),
        ..ScalarGroupLimits::new()
    });
    let crate::builder::ScalarGroupBindingKind::Assignment(assignment) =
        model.scalar_groups()[0].kind
    else {
        panic!("test model should contain an assignment-backed scalar group");
    };
    let plan = coverage_plan(
        2,
        vec![
            coverage_slot(true, 1, Some(0), &[0, 1]),
            coverage_slot(true, 1, Some(1), &[0, 1]),
            coverage_slot(true, 0, Some(0), &[0, 1]),
            coverage_slot(true, 0, Some(1), &[0, 1]),
        ],
    );
    let options = crate::phase::construction::grouped_scalar::ScalarAssignmentMoveOptions::for_selector(
        model.scalar_groups()[0].limits,
        None,
        1,
        0,
    );
    let moves = crate::phase::construction::grouped_scalar::rematch_assignment_moves(
        &assignment,
        &plan,
        options,
    );
    assert_eq!(moves.len(), 1);

    let mut trial = CoverageDirector {
        working_solution: plan,
        descriptor: coverage_plan_descriptor(),
    };
    moves[0].do_move(&mut trial);

    assert_eq!(trial.working_solution.slots[0].assigned, Some(0));
    assert_eq!(trial.working_solution.slots[1].assigned, Some(1));
    assert_eq!(trial.working_solution.slots[2].assigned, Some(1));
    assert_eq!(trial.working_solution.slots[3].assigned, Some(0));
}

#[test]
fn scalar_assignment_reassignment_emits_bounded_direct_moves() {
    let plan = coverage_plan(
        3,
        vec![coverage_slot(true, 0, Some(0), &[0, 1, 2])],
    );
    let selector = scalar_assignment_selector(None, Some(1), false);
    let results = repair_move_results(&plan, &selector);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].slots[0].assigned, Some(1));
}
