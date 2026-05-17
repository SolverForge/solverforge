#[test]
fn default_scalar_assignment_construction_batches_required_fill() {
    let solver_scope = solve_default_assignment(coverage_plan(
        2,
        vec![
            coverage_slot(true, 0, None, &[0, 1]),
            coverage_slot(true, 1, None, &[0, 1]),
        ],
    ));

    let slots = &solver_scope.working_solution().slots;
    assert!(slots.iter().all(|slot| slot.assigned.is_some()));
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(HardSoftScore::of(0, 0))
    );
    assert_eq!(solver_scope.stats().moves_generated, 1);
    assert_eq!(solver_scope.stats().moves_accepted, 1);
    assert_eq!(solver_scope.stats().moves_applied, 1);
    assert_eq!(solver_scope.stats().step_count, 1);
}

#[test]
fn required_assignment_construction_completes_under_expired_time_limit() {
    let solver_scope = solve_default_assignment_with_expired_time_limit(coverage_plan(
        2,
        vec![
            coverage_slot(true, 0, None, &[0, 1]),
            coverage_slot(true, 1, None, &[0, 1]),
            coverage_slot(true, 2, None, &[0, 1]),
        ],
    ));

    assert!(solver_scope
        .working_solution()
        .slots
        .iter()
        .all(|slot| slot.assigned.is_some()));
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(HardSoftScore::of(0, 0))
    );
    assert_eq!(solver_scope.stats().moves_applied, 1);
}

#[test]
fn required_assignment_construction_batches_under_expired_time_limit() {
    let values = (0..80).collect::<Vec<_>>();
    let solver_scope = solve_default_assignment_with_expired_time_limit(coverage_plan(
        values.len(),
        vec![coverage_slot(true, 0, None, &values)],
    ));

    assert!(solver_scope.working_solution().slots[0].assigned.is_some());
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(HardSoftScore::of(0, 0))
    );
    assert_eq!(solver_scope.stats().moves_generated, 1);
    assert_eq!(solver_scope.stats().moves_evaluated, 1);
}

#[test]
fn scalar_assignment_construction_cursor_batches_required_entity_values() {
    let model = assignment_model_with_limits(ScalarGroupLimits {
        group_candidate_limit: Some(4),
        max_moves_per_step: Some(1),
        max_augmenting_depth: Some(3),
        ..ScalarGroupLimits::new()
    });
    let plan = coverage_plan(
        2,
        vec![
            coverage_slot(true, 0, None, &[0, 1]),
            coverage_slot(true, 1, None, &[0, 1]),
        ],
    );
    let crate::builder::ScalarGroupBindingKind::Assignment(assignment) =
        model.scalar_groups()[0].kind
    else {
        panic!("test model should contain an assignment-backed scalar group");
    };
    let options =
        crate::phase::construction::grouped_scalar::ScalarAssignmentMoveOptions::for_construction(
            model.scalar_groups()[0].limits,
        );
    let mut cursor =
        crate::phase::construction::grouped_scalar::ScalarAssignmentMoveCursor::required_construction(
            assignment, plan, options,
        );
    let mut moves = Vec::new();
    while let Some(mov) = cursor.next_move() {
        moves.push(mov);
    }

    assert_eq!(moves.len(), 1);
    for mov in moves {
        assert_eq!(mov.reason(), "scalar_assignment_required");
        assert_eq!(
            mov.edits().len(),
            2,
            "required construction cursor must preserve the fast whole-phase fill"
        );
    }
}

#[test]
fn scalar_assignment_construction_ignores_repair_move_cap() {
    let model = assignment_model_with_limits(ScalarGroupLimits {
        group_candidate_limit: Some(4),
        max_moves_per_step: Some(1),
        max_augmenting_depth: Some(3),
        ..ScalarGroupLimits::new()
    });
    let plan = coverage_plan(
        2,
        vec![
            coverage_slot(true, 0, None, &[0, 1]),
            coverage_slot(true, 1, None, &[0, 1]),
        ],
    );
    let crate::builder::ScalarGroupBindingKind::Assignment(assignment) =
        model.scalar_groups()[0].kind
    else {
        panic!("test model should contain an assignment-backed scalar group");
    };
    let options =
        crate::phase::construction::grouped_scalar::ScalarAssignmentMoveOptions::for_construction(
            model.scalar_groups()[0].limits,
        );
    let mut cursor =
        crate::phase::construction::grouped_scalar::ScalarAssignmentMoveCursor::required_construction(
            assignment, plan, options,
        );
    let mut edited_slots = 0;
    while let Some(mov) = cursor.next_move() {
        edited_slots += mov.edits().len();
    }

    assert_eq!(edited_slots, 2);
}
