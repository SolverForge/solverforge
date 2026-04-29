
#[test]
fn default_scalar_selector_uses_change_and_swap() {
    let descriptor = descriptor(true);
    let director = create_director(
        MixedPlan {
            shifts: vec![Shift { worker: Some(0) }, Shift { worker: Some(1) }],
            vehicles: vec![],
            score: None,
        },
        descriptor.clone(),
    );
    let selector = build_move_selector(None, &scalar_only_model(), None);
    let neighborhoods = selector.selectors();

    assert_eq!(neighborhoods.len(), 2);
    match &neighborhoods[0] {
        Neighborhood::Flat(leafs) => {
            assert_eq!(leafs.selectors().len(), 1);
            assert!(matches!(
                &leafs.selectors()[0],
                NeighborhoodLeaf::Scalar(ScalarLeafSelector::Change(_))
            ));
        }
        Neighborhood::Limited { .. } => panic!("default scalar selector must not wrap a limit"),
        Neighborhood::Cartesian(_) => {
            panic!("default scalar selector must not wrap a cartesian neighborhood")
        }
    }
    assert!(matches!(
        &neighborhoods[1],
        Neighborhood::Flat(leafs)
            if matches!(&leafs.selectors()[0], NeighborhoodLeaf::Scalar(ScalarLeafSelector::Swap(_)))
    ));
    assert_eq!(selector.size(&director), 7);
}

#[test]
fn default_list_selector_uses_three_explicit_neighborhoods() {
    let selector = build_move_selector(None, &list_only_model(), None);
    let neighborhoods = selector.selectors();

    assert_eq!(neighborhoods.len(), 3);
    assert!(matches!(
        &neighborhoods[0],
        Neighborhood::Flat(leafs)
            if matches!(&leafs.selectors()[0], NeighborhoodLeaf::List(ListLeafSelector::NearbyListChange(_)))
    ));
    assert!(matches!(
        &neighborhoods[1],
        Neighborhood::Flat(leafs)
            if matches!(&leafs.selectors()[0], NeighborhoodLeaf::List(ListLeafSelector::NearbyListSwap(_)))
    ));
    assert!(matches!(
        &neighborhoods[2],
        Neighborhood::Flat(leafs)
            if matches!(&leafs.selectors()[0], NeighborhoodLeaf::List(ListLeafSelector::ListReverse(_)))
    ));
}

#[test]
fn mixed_default_selector_puts_list_neighborhoods_before_scalar_defaults() {
    let selector = build_move_selector(None, &mixed_model(), None);
    let neighborhoods = selector.selectors();

    assert_eq!(neighborhoods.len(), 5);
    assert!(matches!(
        &neighborhoods[0],
        Neighborhood::Flat(leafs)
            if matches!(&leafs.selectors()[0], NeighborhoodLeaf::List(ListLeafSelector::NearbyListChange(_)))
    ));
    assert!(matches!(
        &neighborhoods[1],
        Neighborhood::Flat(leafs)
            if matches!(&leafs.selectors()[0], NeighborhoodLeaf::List(ListLeafSelector::NearbyListSwap(_)))
    ));
    assert!(matches!(
        &neighborhoods[2],
        Neighborhood::Flat(leafs)
            if matches!(&leafs.selectors()[0], NeighborhoodLeaf::List(ListLeafSelector::ListReverse(_)))
    ));
    assert!(matches!(
        &neighborhoods[3],
        Neighborhood::Flat(leafs)
            if matches!(&leafs.selectors()[0], NeighborhoodLeaf::Scalar(ScalarLeafSelector::Change(_)))
    ));
    assert!(matches!(
        &neighborhoods[4],
        Neighborhood::Flat(leafs)
            if matches!(&leafs.selectors()[0], NeighborhoodLeaf::Scalar(ScalarLeafSelector::Swap(_)))
    ));
}

#[test]
fn explicit_limited_neighborhood_remains_supported() {
    let descriptor = descriptor(true);
    let director = create_director(
        MixedPlan {
            shifts: vec![Shift { worker: Some(0) }, Shift { worker: Some(1) }],
            vehicles: vec![],
            score: None,
        },
        descriptor.clone(),
    );
    let config = MoveSelectorConfig::LimitedNeighborhood(LimitedNeighborhoodConfig {
        selected_count_limit: 2,
        selector: Box::new(MoveSelectorConfig::ChangeMoveSelector(ChangeMoveConfig {
            value_candidate_limit: None,
            target: VariableTargetConfig::default(),
        })),
    });

    let selector = build_move_selector(Some(&config), &scalar_only_model(), None);
    let neighborhoods = selector.selectors();

    assert_eq!(neighborhoods.len(), 1);
    match &neighborhoods[0] {
        Neighborhood::Limited {
            selected_count_limit,
            ..
        } => {
            assert_eq!(*selected_count_limit, 2);
            assert_eq!(selector.size(&director), 2);
        }
        Neighborhood::Flat(_) => panic!("limited_neighborhood must remain a neighborhood wrapper"),
        Neighborhood::Cartesian(_) => {
            panic!("limited_neighborhood must not become a cartesian neighborhood")
        }
    }
}

#[test]
fn union_child_limited_neighborhood_keeps_scalar_change_context() {
    let descriptor = descriptor(true);
    let director = create_director(
        MixedPlan {
            shifts: vec![Shift { worker: Some(0) }, Shift { worker: Some(1) }],
            vehicles: vec![],
            score: None,
        },
        descriptor.clone(),
    );
    let config = MoveSelectorConfig::UnionMoveSelector(UnionMoveSelectorConfig {
        selection_order: solverforge_config::UnionSelectionOrder::Sequential,
        selectors: vec![MoveSelectorConfig::LimitedNeighborhood(
            LimitedNeighborhoodConfig {
                selected_count_limit: 2,
                selector: Box::new(MoveSelectorConfig::ChangeMoveSelector(ChangeMoveConfig {
                    value_candidate_limit: None,
                    target: VariableTargetConfig::default(),
                })),
            },
        )],
    });

    let selector = build_move_selector(Some(&config), &scalar_only_model(), None);
    let neighborhoods = selector.selectors();

    assert_eq!(neighborhoods.len(), 1);
    match &neighborhoods[0] {
        Neighborhood::Limited {
            selector: leaves,
            selected_count_limit,
        } => {
            assert_eq!(*selected_count_limit, 2);
            assert!(matches!(
                &leaves.selectors()[0],
                NeighborhoodLeaf::Scalar(ScalarLeafSelector::Change(_))
            ));
            assert_eq!(selector.size(&director), 2);
        }
        Neighborhood::Flat(_) => panic!("limited union child must remain a neighborhood wrapper"),
        Neighborhood::Cartesian(_) => panic!("limited union child must not become cartesian"),
    }
}

#[test]
fn explicit_scalar_union_selector_remains_supported() {
    let config = MoveSelectorConfig::UnionMoveSelector(UnionMoveSelectorConfig {
        selection_order: solverforge_config::UnionSelectionOrder::Sequential,
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
    let neighborhoods = selector.selectors();

    assert_eq!(neighborhoods.len(), 2);
    assert!(matches!(
        &neighborhoods[0],
        Neighborhood::Flat(leafs)
            if matches!(&leafs.selectors()[0], NeighborhoodLeaf::Scalar(ScalarLeafSelector::Change(_)))
    ));
    assert!(matches!(
        &neighborhoods[1],
        Neighborhood::Flat(leafs)
            if matches!(&leafs.selectors()[0], NeighborhoodLeaf::Scalar(ScalarLeafSelector::Swap(_)))
    ));
}

#[test]
fn explicit_scalar_union_selector_can_be_round_robin() {
    let config = MoveSelectorConfig::UnionMoveSelector(UnionMoveSelectorConfig {
        selection_order: solverforge_config::UnionSelectionOrder::RoundRobin,
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

    assert_eq!(
        selector.selection_order(),
        solverforge_config::UnionSelectionOrder::RoundRobin
    );
}

fn repair_worker_to_one(
    _solution: &MixedPlan,
    limits: crate::builder::ConflictRepairLimits,
) -> Vec<crate::builder::ConflictRepairSpec> {
    assert_eq!(limits.max_matches_per_step, 2);
    assert_eq!(limits.max_repairs_per_match, 3);
    assert_eq!(limits.max_moves_per_step, 4);
    vec![
        crate::builder::ConflictRepairSpec::new(
            "testConstraint",
            vec![crate::builder::ConflictRepairEdit::set_scalar(
                0,
                0,
                "worker",
                Some(1),
            )],
        ),
        crate::builder::ConflictRepairSpec::new(
            "testConstraint",
            vec![crate::builder::ConflictRepairEdit::set_scalar(
                0,
                1,
                "worker",
                Some(99),
            )],
        ),
    ]
}

#[test]
fn conflict_repair_selector_builds_executable_registered_repairs() {
    let descriptor = descriptor(true);
    let mut director = create_director(
        MixedPlan {
            shifts: vec![Shift { worker: Some(0) }, Shift { worker: Some(1) }],
            vehicles: vec![],
            score: None,
        },
        descriptor,
    );
    let model = scalar_only_model().with_conflict_repair_providers(vec![
        crate::builder::ConflictRepairProviderEntry::new("testConstraint", repair_worker_to_one),
    ]);
    let config = MoveSelectorConfig::ConflictRepairMoveSelector(
        solverforge_config::ConflictRepairMoveSelectorConfig {
            constraints: vec!["testConstraint".to_string()],
            max_matches_per_step: 2,
            max_repairs_per_match: 3,
            max_moves_per_step: 4,
            include_soft_matches: false,
        },
    );

    let selector = build_move_selector(Some(&config), &model, None);
    let mut cursor = selector.open_cursor(&director);
    let first = cursor
        .next_candidate()
        .expect("registered legal repair should produce a candidate");
    assert!(
        cursor.next_candidate().is_none(),
        "illegal provider edits must be filtered before candidate exposure"
    );

    let repair = cursor.take_candidate(first);
    assert!(repair.is_doable(&director));
    repair.do_move(&mut director);

    assert_eq!(director.working_solution().shifts[0].worker, Some(1));
    assert_eq!(director.working_solution().shifts[1].worker, Some(1));
}
