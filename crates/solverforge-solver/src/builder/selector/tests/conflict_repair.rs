#[test]
fn conflict_repair_accepts_package_qualified_constraint_metadata() {
    let descriptor = descriptor(true);
    let director = create_director_with_constraint_ref(
        MixedPlan {
            shifts: vec![Shift { worker: Some(0) }, Shift { worker: Some(1) }],
            vehicles: vec![],
            score: None,
        },
        descriptor,
        "pkg",
        "testConstraint",
        true,
    );
    let model = scalar_only_model().with_conflict_repair_providers(vec![
        crate::builder::ConflictRepairProviderEntry::new("pkg/testConstraint", repair_worker_to_one),
    ]);
    let config = MoveSelectorConfig::ConflictRepairMoveSelector(
        solverforge_config::ConflictRepairMoveSelectorConfig {
            constraints: vec!["pkg/testConstraint".to_string()],
            max_matches_per_step: 2,
            max_repairs_per_match: 3,
            max_moves_per_step: 4,
            require_hard_improvement: false,
            include_soft_matches: false,
        },
    );
    let selector = build_move_selector(Some(&config), &model, None);
    let mut cursor = selector.open_cursor(&director);

    assert!(cursor.next_candidate().is_some());
}

#[test]
#[should_panic(
    expected = "conflict_repair_move_selector configured for non-hard constraint `pkg/testConstraint` while include_soft_matches is false"
)]
fn conflict_repair_rejects_soft_package_qualified_metadata() {
    let descriptor = descriptor(true);
    let director = create_director_with_constraint_ref(
        MixedPlan {
            shifts: vec![Shift { worker: Some(0) }],
            vehicles: vec![],
            score: None,
        },
        descriptor,
        "pkg",
        "testConstraint",
        false,
    );
    let model = scalar_only_model().with_conflict_repair_providers(vec![
        crate::builder::ConflictRepairProviderEntry::new(
            "pkg/testConstraint",
            repair_provider_must_not_run,
        ),
    ]);
    let config = MoveSelectorConfig::ConflictRepairMoveSelector(
        solverforge_config::ConflictRepairMoveSelectorConfig {
            constraints: vec!["pkg/testConstraint".to_string()],
            max_matches_per_step: 2,
            max_repairs_per_match: 3,
            max_moves_per_step: 4,
            require_hard_improvement: false,
            include_soft_matches: false,
        },
    );
    let selector = build_move_selector(Some(&config), &model, None);

    let _ = selector.open_cursor(&director);
}

#[test]
#[should_panic(
    expected = "conflict_repair_move_selector configured for `testConstraint`, but no matching scoring constraint was found"
)]
fn conflict_repair_rejects_short_name_for_package_qualified_metadata() {
    let descriptor = descriptor(true);
    let director = create_director_with_constraint_ref(
        MixedPlan {
            shifts: vec![Shift { worker: Some(0) }],
            vehicles: vec![],
            score: None,
        },
        descriptor,
        "pkg",
        "testConstraint",
        true,
    );
    let model = scalar_only_model().with_conflict_repair_providers(vec![
        crate::builder::ConflictRepairProviderEntry::new(
            "testConstraint",
            repair_provider_must_not_run,
        ),
    ]);
    let config = MoveSelectorConfig::ConflictRepairMoveSelector(
        solverforge_config::ConflictRepairMoveSelectorConfig {
            constraints: vec!["testConstraint".to_string()],
            max_matches_per_step: 2,
            max_repairs_per_match: 3,
            max_moves_per_step: 4,
            require_hard_improvement: false,
            include_soft_matches: false,
        },
    );
    let selector = build_move_selector(Some(&config), &model, None);

    let _ = selector.open_cursor(&director);
}
