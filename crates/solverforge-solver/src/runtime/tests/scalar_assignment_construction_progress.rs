#[test]
fn scalar_assignment_selector_releases_optional_assigned_entities() {
    let plan = coverage_plan(1, vec![coverage_slot(false, 0, Some(0), &[0])]);
    let moves = selector_assignment_moves_for_plan(&plan, ScalarGroupLimits::new(), 16);

    assert!(moves.iter().any(|mov| {
        mov.reason() == "scalar_assignment_optional_release"
            && mov.edits().len() == 1
            && mov.edits()[0].entity_index == 0
            && mov.edits()[0].to_value.is_none()
    }));
}

#[test]
fn scalar_assignment_selector_assigns_optional_unassigned_entities() {
    let plan = coverage_plan(1, vec![coverage_slot(false, 0, None, &[0])]);
    let moves = selector_assignment_moves_for_plan(&plan, ScalarGroupLimits::new(), 16);

    assert!(moves.iter().any(|mov| {
        mov.reason() == "scalar_assignment_optional"
            && mov.edits().len() == 1
            && mov.edits()[0].entity_index == 0
            && mov.edits()[0].to_value == Some(0)
    }));
}

#[test]
fn scalar_assignment_selector_transfers_optional_assignments_to_open_slots() {
    let plan = coverage_plan(
        1,
        vec![
            coverage_slot(false, 0, Some(0), &[0]),
            coverage_slot(false, 1, None, &[0]),
        ],
    );
    let moves = selector_assignment_moves_for_plan(&plan, ScalarGroupLimits::new(), 16);

    assert!(moves.iter().any(|mov| {
        mov.reason() == "scalar_assignment_optional_transfer"
            && mov.edits().len() == 2
            && mov
                .edits()
                .iter()
                .any(|edit| edit.entity_index == 0 && edit.to_value.is_none())
            && mov
                .edits()
                .iter()
                .any(|edit| edit.entity_index == 1 && edit.to_value == Some(0))
    }));
}

#[test]
fn scalar_assignment_selector_prioritizes_required_work() {
    let plan = coverage_plan(
        2,
        vec![
            coverage_slot(false, 0, Some(0), &[0, 1]),
            coverage_slot(true, 0, None, &[0, 1]),
        ],
    );
    let moves = selector_assignment_moves_for_plan(&plan, ScalarGroupLimits::new(), 1);

    assert_eq!(moves.len(), 1);
    assert_eq!(moves[0].reason(), "scalar_assignment_required");
    assert!(moves[0]
        .edits()
        .iter()
        .any(|edit| edit.entity_index == 1 && edit.to_value.is_some()));
}

#[test]
fn scalar_assignment_selector_interleaves_later_assignment_families() {
    let plan = coverage_plan(
        3,
        vec![
            coverage_slot(true, 0, None, &[0, 1, 2]),
            coverage_slot(true, 1, None, &[0, 1, 2]),
            coverage_slot(true, 2, None, &[0, 1, 2]),
            coverage_slot(true, 3, Some(0), &[0, 1, 2]),
            coverage_slot(true, 4, Some(1), &[0, 1, 2]),
        ],
    );
    let moves = selector_assignment_moves_for_plan(&plan, ScalarGroupLimits::new(), 2);

    assert_eq!(moves.len(), 2);
    assert_eq!(moves[0].reason(), "scalar_assignment_required");
    assert_ne!(moves[1].reason(), "scalar_assignment_required");
}

#[test]
fn scalar_assignment_selector_repairs_capacity_conflicts_without_losing_required() {
    let plan = coverage_plan(
        1,
        vec![
            coverage_slot(true, 0, Some(0), &[0]),
            coverage_slot(false, 0, Some(0), &[0]),
        ],
    );
    let moves = selector_assignment_moves_for_plan(&plan, ScalarGroupLimits::new(), 16);

    assert!(moves.iter().any(|mov| {
        mov.reason() == "scalar_assignment_capacity_repair"
            && mov.edits().len() == 1
            && mov.edits()[0].entity_index == 1
            && mov.edits()[0].to_value.is_none()
    }));
}

#[test]
fn scalar_assignment_selector_deduplicates_equivalent_compound_edits() {
    let plan = coverage_plan(
        2,
        vec![
            coverage_slot(true, 0, Some(0), &[0, 1]),
            coverage_slot(true, 0, Some(1), &[0, 1]),
        ],
    );
    let moves = selector_assignment_moves_for_plan(&plan, ScalarGroupLimits::new(), 32);
    let mut seen = std::collections::HashSet::new();

    for mov in &moves {
        let mut key = mov
            .edits()
            .iter()
            .map(|edit| (edit.entity_index, edit.to_value))
            .collect::<Vec<_>>();
        key.sort_unstable();
        assert!(seen.insert(key), "duplicate compound assignment move exposed");
    }
}

#[test]
fn scalar_assignment_construction_assigns_optional_only_after_required_complete() {
    let solver_scope = solve_assignment(coverage_plan(
        1,
        vec![
            coverage_slot(false, 0, None, &[0]),
            coverage_slot(true, 0, None, &[0]),
        ],
    ));

    let slots = &solver_scope.working_solution().slots;
    assert_eq!(slots[0].assigned, None);
    assert_eq!(slots[1].assigned, Some(0));
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(HardSoftScore::of(0, -1))
    );
}

#[test]
fn default_scalar_assignment_construction_does_not_fill_optional_slots_in_required_pass() {
    let solver_scope = solve_default_assignment(coverage_plan(
        1,
        vec![
            coverage_slot(false, 0, None, &[0]),
            coverage_slot(true, 0, None, &[0]),
        ],
    ));

    let slots = &solver_scope.working_solution().slots;
    assert_eq!(slots[0].assigned, None);
    assert_eq!(slots[1].assigned, Some(0));
}

#[test]
fn scalar_assignment_construction_displaces_optional_occupant_for_required_slot() {
    let solver_scope = solve_assignment(coverage_plan(
        1,
        vec![
            coverage_slot(false, 0, Some(0), &[0]),
            coverage_slot(true, 0, None, &[0]),
        ],
    ));

    let slots = &solver_scope.working_solution().slots;
    assert_eq!(slots[0].assigned, None);
    assert_eq!(slots[1].assigned, Some(0));
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(HardSoftScore::of(0, -1))
    );
}

#[test]
fn scalar_assignment_construction_moves_required_blocker_through_augmenting_path() {
    let solver_scope = solve_assignment(coverage_plan(
        2,
        vec![
            coverage_slot(true, 0, None, &[0]),
            coverage_slot(true, 0, Some(0), &[0, 1]),
        ],
    ));

    let slots = &solver_scope.working_solution().slots;
    assert_eq!(slots[0].assigned, Some(0));
    assert_eq!(slots[1].assigned, Some(1));
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(HardSoftScore::of(0, 0))
    );
}

#[test]
fn scalar_assignment_construction_forces_required_assignment_when_hard_neutral_soft_worse() {
    let solver_scope = solve_assignment(soft_preferred_coverage_plan(
        1,
        vec![coverage_slot_with_penalty(true, 0, None, &[0], 5)],
    ));

    assert_eq!(solver_scope.working_solution().slots[0].assigned, Some(0));
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(HardSoftScore::of(0, -5))
    );
    assert_eq!(solver_scope.stats().scalar_assignment_required_remaining, 0);
}

#[test]
fn scalar_assignment_construction_honors_explicit_optional_assignment_obligation() {
    let solver_scope = solve_assignment(soft_preferred_coverage_plan(
        1,
        vec![coverage_slot_with_penalty(false, 0, None, &[0], 5)],
    ));

    assert_eq!(solver_scope.working_solution().slots[0].assigned, Some(0));
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(HardSoftScore::of(0, -5))
    );
}

#[test]
fn scalar_assignment_construction_reports_remaining_required_slots_without_panic() {
    let solver_scope = solve_assignment(coverage_plan(1, vec![coverage_slot(true, 0, None, &[])]));

    assert_eq!(solver_scope.working_solution().slots[0].assigned, None);
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(HardSoftScore::of(-1, 0))
    );
    assert_eq!(solver_scope.stats().scalar_assignment_required_remaining, 1);
}

#[test]
fn scalar_assignment_best_solution_callback_reports_current_remaining_required_slots() {
    let best_remaining = std::sync::Mutex::new(Vec::new());
    {
        let descriptor = coverage_plan_descriptor();
        let director = CoverageDirector {
            working_solution: soft_preferred_coverage_plan(
                1,
                vec![
                    coverage_slot(false, 0, Some(0), &[0]),
                    coverage_slot(true, 0, None, &[0]),
                ],
            ),
            descriptor: descriptor.clone(),
        };
        let callback = |progress: crate::scope::SolverProgressRef<'_, CoveragePlan>| {
            if progress.kind == crate::scope::SolverProgressKind::BestSolution {
                best_remaining
                    .lock()
                    .expect("best-solution telemetry capture should not be poisoned")
                    .push(progress.telemetry.scalar_assignment_required_remaining);
            }
        };
        let mut solver_scope = SolverScope::new(director).with_progress_callback(callback);
        solver_scope.start_solving();
        let mut config = assignment_config();
        config.construction_obligation = ConstructionObligation::PreserveUnassigned;
        let mut phase = Construction::new(Some(config), descriptor, assignment_model());

        phase.solve(&mut solver_scope);

        assert_eq!(solver_scope.working_solution().slots[1].assigned, None);
        assert_eq!(solver_scope.stats().scalar_assignment_required_remaining, 1);
    }

    let best_remaining = best_remaining
        .into_inner()
        .expect("best-solution telemetry capture should not be poisoned");
    assert_eq!(best_remaining.as_slice(), &[1]);
}
