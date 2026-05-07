
#[test]
fn cartesian_scalar_selector_builds_composite_moves() {
    let descriptor = descriptor(true);
    let director = create_director(
        MixedPlan {
            shifts: vec![
                Shift { worker: Some(0) },
                Shift { worker: Some(1) },
                Shift { worker: Some(2) },
            ],
            vehicles: vec![],
            score: None,
        },
        descriptor,
    );
    let change = MoveSelectorConfig::ChangeMoveSelector(ChangeMoveConfig {

        value_candidate_limit: None,

        target: VariableTargetConfig::default(),
    });
    let swap = MoveSelectorConfig::SwapMoveSelector(SwapMoveConfig {
        target: VariableTargetConfig::default(),
    });
    let config = MoveSelectorConfig::CartesianProductMoveSelector(CartesianProductConfig {
        require_hard_improvement: false,
        selectors: vec![change.clone(), swap.clone()],
    });

    let selector = build_move_selector(Some(&config), &scalar_only_model(), None);
    let neighborhoods = selector.selectors();
    let left = build_move_selector(Some(&change), &scalar_only_model(), None);
    let right = build_move_selector(Some(&swap), &scalar_only_model(), None);

    let mut cursor = selector.open_cursor(&director);
    let indices =
        collect_cursor_indices::<MixedPlan, NeighborhoodMove<MixedPlan, usize>, _>(&mut cursor);

    assert_eq!(neighborhoods.len(), 1);
    assert!(selector.size(&director) <= left.size(&director) * right.size(&director));
    assert!(matches!(&neighborhoods[0], Neighborhood::Cartesian(_)));
    assert!(!indices.is_empty());
    assert!(indices.iter().all(|&index| matches!(
        cursor.candidate(index),
        Some(MoveCandidateRef::Sequential(_))
    )));
    assert!(indices.iter().all(|&index| cursor
        .candidate(index)
        .is_some_and(|mov| mov.is_doable(&director))));
    assert!(matches!(
        cursor.take_candidate(indices[0]),
        NeighborhoodMove::Composite(_)
    ));
}

#[test]
fn cartesian_scalar_selector_can_require_hard_improvement() {
    let descriptor = descriptor(true);
    let director = create_director(
        MixedPlan {
            shifts: vec![Shift { worker: Some(0) }, Shift { worker: Some(1) }],
            vehicles: vec![],
            score: None,
        },
        descriptor,
    );
    let config = MoveSelectorConfig::CartesianProductMoveSelector(CartesianProductConfig {
        require_hard_improvement: true,
        selectors: vec![
            MoveSelectorConfig::ChangeMoveSelector(ChangeMoveConfig {
                value_candidate_limit: None,
                target: VariableTargetConfig::default(),
            }),
            MoveSelectorConfig::SwapMoveSelector(SwapMoveConfig {
                target: VariableTargetConfig::default(),
            }),
        ],
    });

    let selector = build_move_selector(Some(&config), &scalar_only_model(), None);
    let mut cursor = selector.open_cursor(&director);
    let indices =
        collect_cursor_indices::<MixedPlan, NeighborhoodMove<MixedPlan, usize>, _>(&mut cursor);

    assert!(!indices.is_empty());
    assert!(cursor
        .candidate(indices[0])
        .is_some_and(|mov| mov.requires_hard_improvement()));
    assert!(cursor
        .take_candidate(indices[0])
        .requires_hard_improvement());
}

#[test]
fn cartesian_right_child_conflict_repair_uses_preview_constraint_metadata() {
    let descriptor = descriptor(true);
    let director = create_director_with_constraint(
        MixedPlan {
            shifts: vec![Shift { worker: Some(0) }, Shift { worker: Some(1) }],
            vehicles: vec![],
            score: None,
        },
        descriptor,
        "testConstraint",
        true,
    );
    let model = scalar_only_model().with_conflict_repairs(vec![
        crate::builder::ConflictRepair::new("testConstraint", repair_worker_to_one),
    ]);
    let config = MoveSelectorConfig::CartesianProductMoveSelector(CartesianProductConfig {
        require_hard_improvement: false,
        selectors: vec![
            MoveSelectorConfig::ChangeMoveSelector(ChangeMoveConfig {
                value_candidate_limit: None,
                target: VariableTargetConfig::default(),
            }),
            MoveSelectorConfig::ConflictRepairMoveSelector(
                solverforge_config::ConflictRepairMoveSelectorConfig {
                    constraints: vec!["testConstraint".to_string()],
                    max_matches_per_step: 2,
                    max_repairs_per_match: 3,
                    max_moves_per_step: 4,
                    require_hard_improvement: false,
                    include_soft_matches: false,
                },
            ),
        ],
    });

    let selector = build_move_selector(Some(&config), &model, None);
    let mut cursor = selector.open_cursor(&director);
    let indices =
        collect_cursor_indices::<MixedPlan, NeighborhoodMove<MixedPlan, usize>, _>(&mut cursor);

    assert!(
        !indices.is_empty(),
        "right-child conflict repair must open against preview metadata"
    );
    assert!(indices.iter().all(|&index| matches!(
        cursor.candidate(index),
        Some(MoveCandidateRef::Sequential(_))
    )));
}

#[test]
fn cartesian_right_child_conflict_repair_uses_package_qualified_preview_metadata() {
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
    let model = scalar_only_model().with_conflict_repairs(vec![
        crate::builder::ConflictRepair::new("pkg/testConstraint", repair_worker_to_one),
    ]);
    let config = MoveSelectorConfig::CartesianProductMoveSelector(CartesianProductConfig {
        require_hard_improvement: false,
        selectors: vec![
            MoveSelectorConfig::ChangeMoveSelector(ChangeMoveConfig {
                value_candidate_limit: None,
                target: VariableTargetConfig::default(),
            }),
            MoveSelectorConfig::ConflictRepairMoveSelector(
                solverforge_config::ConflictRepairMoveSelectorConfig {
                    constraints: vec!["pkg/testConstraint".to_string()],
                    max_matches_per_step: 2,
                    max_repairs_per_match: 3,
                    max_moves_per_step: 4,
                    require_hard_improvement: false,
                    include_soft_matches: false,
                },
            ),
        ],
    });

    let selector = build_move_selector(Some(&config), &model, None);
    let mut cursor = selector.open_cursor(&director);
    let indices =
        collect_cursor_indices::<MixedPlan, NeighborhoodMove<MixedPlan, usize>, _>(&mut cursor);

    assert!(
        !indices.is_empty(),
        "right-child conflict repair must resolve package-qualified preview metadata"
    );
    assert!(indices.iter().all(|&index| matches!(
        cursor.candidate(index),
        Some(MoveCandidateRef::Sequential(_))
    )));
}

#[test]
#[should_panic(
    expected = "conflict_repair_move_selector configured for non-hard constraint `testConstraint` while include_soft_matches is false"
)]
fn cartesian_right_child_conflict_repair_rejects_soft_metadata_when_not_configured() {
    let descriptor = descriptor(true);
    let director = create_director_with_constraint(
        MixedPlan {
            shifts: vec![Shift { worker: Some(0) }, Shift { worker: Some(1) }],
            vehicles: vec![],
            score: None,
        },
        descriptor,
        "testConstraint",
        false,
    );
    let model = scalar_only_model().with_conflict_repairs(vec![
        crate::builder::ConflictRepair::new("testConstraint", repair_worker_to_one),
    ]);
    let config = MoveSelectorConfig::CartesianProductMoveSelector(CartesianProductConfig {
        require_hard_improvement: false,
        selectors: vec![
            MoveSelectorConfig::ChangeMoveSelector(ChangeMoveConfig {
                value_candidate_limit: None,
                target: VariableTargetConfig::default(),
            }),
            MoveSelectorConfig::ConflictRepairMoveSelector(
                solverforge_config::ConflictRepairMoveSelectorConfig {
                    constraints: vec!["testConstraint".to_string()],
                    max_matches_per_step: 2,
                    max_repairs_per_match: 3,
                    max_moves_per_step: 4,
                    require_hard_improvement: false,
                    include_soft_matches: false,
                },
            ),
        ],
    });
    let selector = build_move_selector(Some(&config), &model, None);

    let _ = selector.open_cursor(&director);
}

#[test]
fn cartesian_right_child_conflict_repair_allows_soft_metadata_when_configured() {
    let descriptor = descriptor(true);
    let director = create_director_with_constraint(
        MixedPlan {
            shifts: vec![Shift { worker: Some(0) }, Shift { worker: Some(1) }],
            vehicles: vec![],
            score: None,
        },
        descriptor,
        "testConstraint",
        false,
    );
    let model = scalar_only_model().with_conflict_repairs(vec![
        crate::builder::ConflictRepair::new("testConstraint", repair_worker_to_one),
    ]);
    let config = MoveSelectorConfig::CartesianProductMoveSelector(CartesianProductConfig {
        require_hard_improvement: false,
        selectors: vec![
            MoveSelectorConfig::ChangeMoveSelector(ChangeMoveConfig {
                value_candidate_limit: None,
                target: VariableTargetConfig::default(),
            }),
            MoveSelectorConfig::ConflictRepairMoveSelector(
                solverforge_config::ConflictRepairMoveSelectorConfig {
                    constraints: vec!["testConstraint".to_string()],
                    max_matches_per_step: 2,
                    max_repairs_per_match: 3,
                    max_moves_per_step: 4,
                    require_hard_improvement: false,
                    include_soft_matches: true,
                },
            ),
        ],
    });

    let selector = build_move_selector(Some(&config), &model, None);
    let mut cursor = selector.open_cursor(&director);
    let indices =
        collect_cursor_indices::<MixedPlan, NeighborhoodMove<MixedPlan, usize>, _>(&mut cursor);

    assert!(!indices.is_empty());
}

#[test]
fn cartesian_list_selector_builds_composite_moves() {
    let descriptor = descriptor(false);
    let director = create_director(
        MixedPlan {
            shifts: vec![],
            vehicles: vec![
                Vehicle {
                    visits: vec![1, 2, 3],
                },
                Vehicle { visits: vec![4, 5] },
            ],
            score: None,
        },
        descriptor,
    );
    let list_change = MoveSelectorConfig::ListChangeMoveSelector(ListChangeMoveConfig {
        target: VariableTargetConfig::default(),
    });
    let list_reverse = MoveSelectorConfig::ListReverseMoveSelector(ListReverseMoveConfig {
        target: VariableTargetConfig::default(),
    });
    let config = MoveSelectorConfig::CartesianProductMoveSelector(CartesianProductConfig {
        require_hard_improvement: false,
        selectors: vec![list_change.clone(), list_reverse.clone()],
    });

    let selector = build_move_selector(Some(&config), &list_only_model(), None);
    let neighborhoods = selector.selectors();
    let mut cursor = selector.open_cursor(&director);
    let indices =
        collect_cursor_indices::<MixedPlan, NeighborhoodMove<MixedPlan, usize>, _>(&mut cursor);

    assert_eq!(neighborhoods.len(), 1);
    assert!(!indices.is_empty());
    assert!(matches!(&neighborhoods[0], Neighborhood::Cartesian(_)));
    assert!(indices.iter().all(|&index| matches!(
        cursor.candidate(index),
        Some(MoveCandidateRef::Sequential(_))
    )));
    assert!(indices.iter().all(|&index| cursor
        .candidate(index)
        .is_some_and(|mov| mov.is_doable(&director))));
    assert!(matches!(
        cursor.take_candidate(indices[0]),
        NeighborhoodMove::Composite(_)
    ));
}

#[test]
fn cartesian_mixed_selector_supports_limited_children() {
    let descriptor = descriptor(true);
    let director = create_director(
        MixedPlan {
            shifts: vec![Shift { worker: Some(0) }, Shift { worker: Some(1) }],
            vehicles: vec![Vehicle {
                visits: vec![1, 2, 3],
            }],
            score: None,
        },
        descriptor,
    );
    let limited_change = MoveSelectorConfig::LimitedNeighborhood(LimitedNeighborhoodConfig {
        selected_count_limit: 2,
        selector: Box::new(MoveSelectorConfig::ChangeMoveSelector(ChangeMoveConfig {

            value_candidate_limit: None,

            target: VariableTargetConfig::default(),
        })),
    });
    let list_reverse = MoveSelectorConfig::ListReverseMoveSelector(ListReverseMoveConfig {
        target: VariableTargetConfig::default(),
    });
    let config = MoveSelectorConfig::CartesianProductMoveSelector(CartesianProductConfig {
        require_hard_improvement: false,
        selectors: vec![limited_change.clone(), list_reverse.clone()],
    });

    let selector = build_move_selector(Some(&config), &mixed_model(), None);
    let left = build_move_selector(Some(&limited_change), &mixed_model(), None);
    let right = build_move_selector(Some(&list_reverse), &mixed_model(), None);
    let mut cursor = selector.open_cursor(&director);
    let indices =
        collect_cursor_indices::<MixedPlan, NeighborhoodMove<MixedPlan, usize>, _>(&mut cursor);

    assert!(selector.size(&director) <= left.size(&director) * right.size(&director));
    assert!(!indices.is_empty());
    assert!(indices.iter().all(|&index| matches!(
        cursor.candidate(index),
        Some(MoveCandidateRef::Sequential(_))
    )));
    assert!(indices.iter().all(|&index| cursor
        .candidate(index)
        .is_some_and(|mov| mov.is_doable(&director))));
    assert!(indices.iter().all(|&index| {
        cursor
            .candidate(index)
            .is_some_and(|mov| mov.variable_name() == "cartesian_product")
    }));
    assert!(matches!(
        cursor.take_candidate(indices[0]),
        NeighborhoodMove::Composite(_)
    ));
}

fn keep_all_mixed_cartesian_candidates(
    candidate: MoveCandidateRef<'_, MixedPlan, NeighborhoodMove<MixedPlan, usize>>,
) -> bool {
    matches!(candidate, MoveCandidateRef::Sequential(_))
}

#[test]
fn mixed_builder_cartesian_selector_survives_filtering_wrapper() {
    let descriptor = descriptor(true);
    let director = create_director(
        MixedPlan {
            shifts: vec![Shift { worker: Some(0) }, Shift { worker: Some(1) }],
            vehicles: vec![Vehicle {
                visits: vec![1, 2, 3],
            }],
            score: None,
        },
        descriptor,
    );
    let config = MoveSelectorConfig::CartesianProductMoveSelector(CartesianProductConfig {
        require_hard_improvement: false,
        selectors: vec![
            MoveSelectorConfig::ChangeMoveSelector(ChangeMoveConfig {

                value_candidate_limit: None,

                target: VariableTargetConfig::default(),
            }),
            MoveSelectorConfig::ListReverseMoveSelector(ListReverseMoveConfig {
                target: VariableTargetConfig::default(),
            }),
        ],
    });

    let selector = build_move_selector(Some(&config), &mixed_model(), None);
    let filtered = FilteringMoveSelector::new(selector, keep_all_mixed_cartesian_candidates);
    let mut cursor = filtered.open_cursor(&director);
    let indices =
        collect_cursor_indices::<MixedPlan, NeighborhoodMove<MixedPlan, usize>, _>(&mut cursor);

    assert!(!indices.is_empty());
    assert!(indices.iter().all(|&index| matches!(
        cursor.candidate(index),
        Some(MoveCandidateRef::Sequential(_))
    )));
    assert!(cursor.take_candidate(indices[0]).is_doable(&director));
}

#[test]
#[should_panic(
    expected = "cartesian_product left child cannot contain ruin_recreate_move_selector or list_ruin_move_selector"
)]
fn cartesian_selector_rejects_score_seeking_scalar_left_child() {
    let config = MoveSelectorConfig::CartesianProductMoveSelector(CartesianProductConfig {
        require_hard_improvement: false,
        selectors: vec![
            MoveSelectorConfig::RuinRecreateMoveSelector(RuinRecreateMoveSelectorConfig::default()),
            MoveSelectorConfig::ChangeMoveSelector(ChangeMoveConfig {

                value_candidate_limit: None,

                target: VariableTargetConfig::default(),
            }),
        ],
    });

    let _ = build_move_selector(Some(&config), &scalar_only_model(), None);
}

#[test]
#[should_panic(
    expected = "cartesian_product left child cannot contain ruin_recreate_move_selector or list_ruin_move_selector"
)]
fn cartesian_selector_rejects_score_seeking_list_left_child() {
    let config = MoveSelectorConfig::CartesianProductMoveSelector(CartesianProductConfig {
        require_hard_improvement: false,
        selectors: vec![
            MoveSelectorConfig::ListRuinMoveSelector(ListRuinMoveSelectorConfig::default()),
            MoveSelectorConfig::ListChangeMoveSelector(ListChangeMoveConfig {
                target: VariableTargetConfig::default(),
            }),
        ],
    });

    let _ = build_move_selector(Some(&config), &list_only_model(), Some(7));
}
