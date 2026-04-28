
#[test]
fn builds_nearby_change_selectors_when_meter_is_present() {
    let director = create_director(Schedule {
        workers: vec![0, 1, 2],
        shifts: vec![
            Shift {
                worker: Some(0),
                allowed_workers: vec![0, 1, 2],
            },
            Shift {
                worker: Some(1),
                allowed_workers: vec![0, 1, 2],
            },
        ],
        score: None,
    });

    let scalar_variables = vec![ScalarVariableContext::new(
        0,
        0,
        "Shift",
        shift_count,
        "worker",
        get_worker,
        set_worker,
        ValueSource::SolutionCount {
            count_fn: worker_count,
            provider_index: 0,
        },
        true,
    )
    .with_nearby_value_candidates(nearby_worker_candidates)
    .with_nearby_value_distance_meter(nearby_worker_value_distance)];

    let config = MoveSelectorConfig::NearbyChangeMoveSelector(NearbyChangeMoveConfig {
        max_nearby: 1,
        value_candidate_limit: None,
        target: VariableTargetConfig {
            entity_class: Some("Shift".to_string()),
            variable_name: Some("worker".to_string()),
        },
    });

    let selector = build_scalar_move_selector(Some(&config), &scalar_variables, None);
    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert_eq!(moves.len(), 4);
    let change_targets: Vec<_> = moves
        .iter()
        .map(|mov| match mov {
            crate::heuristic::r#move::ScalarMoveUnion::Change(change) => {
                (change.entity_index(), change.to_value().copied())
            }
            other => panic!("expected nearby change move, got {other:?}"),
        })
        .collect();
    assert_eq!(
        change_targets,
        vec![(0, Some(1)), (0, None), (1, Some(0)), (1, None)]
    );
    assert!(moves.iter().all(|mov| mov.is_doable(&director)));
}

#[test]
#[should_panic(expected = "nearby_change_move selector requires nearby_value_candidates")]
fn nearby_change_rejects_distance_meter_without_candidate_hook() {
    let scalar_variables = vec![ScalarVariableContext::new(
        0,
        0,
        "Shift",
        shift_count,
        "worker",
        get_worker,
        set_worker,
        ValueSource::SolutionCount {
            count_fn: worker_count,
            provider_index: 0,
        },
        true,
    )
    .with_nearby_value_distance_meter(nearby_worker_value_distance)];

    let config = MoveSelectorConfig::NearbyChangeMoveSelector(NearbyChangeMoveConfig {
        max_nearby: 1,
        value_candidate_limit: None,
        target: VariableTargetConfig::default(),
    });

    let _ = build_scalar_move_selector(Some(&config), &scalar_variables, None);
}

#[test]
fn scalar_change_applies_value_candidate_limit_before_generation() {
    let director = create_director(Schedule {
        workers: (0..100).collect(),
        shifts: vec![Shift {
            worker: Some(0),
            allowed_workers: Vec::new(),
        }],
        score: None,
    });
    let scalar_variables = vec![ScalarVariableContext::new(
        0,
        0,
        "Shift",
        shift_count,
        "worker",
        get_worker,
        set_worker,
        ValueSource::SolutionCount {
            count_fn: worker_count,
            provider_index: 0,
        },
        false,
    )];
    let config = MoveSelectorConfig::ChangeMoveSelector(ChangeMoveConfig {
        value_candidate_limit: Some(3),
        target: VariableTargetConfig::default(),
    });

    let selector = build_scalar_move_selector(Some(&config), &scalar_variables, None);
    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert_eq!(moves.len(), 3);
    let values: Vec<_> = moves
        .iter()
        .map(|mov| match mov {
            crate::heuristic::r#move::ScalarMoveUnion::Change(change) => {
                change.to_value().copied()
            }
            other => panic!("expected scalar change, got {other:?}"),
        })
        .collect();
    assert_eq!(values, vec![Some(0), Some(1), Some(2)]);
}

#[test]
fn nearby_change_applies_value_candidate_limit_before_ranking() {
    let director = create_director(Schedule {
        workers: vec![0, 1, 2],
        shifts: vec![Shift {
            worker: Some(0),
            allowed_workers: vec![2, 1, 0],
        }],
        score: None,
    });
    let scalar_variables = vec![ScalarVariableContext::new(
        0,
        0,
        "Shift",
        shift_count,
        "worker",
        get_worker,
        set_worker,
        ValueSource::SolutionCount {
            count_fn: worker_count,
            provider_index: 0,
        },
        false,
    )
    .with_nearby_value_candidates(nearby_worker_candidates)
    .with_nearby_value_distance_meter(nearby_worker_value_distance)];
    let config = MoveSelectorConfig::NearbyChangeMoveSelector(NearbyChangeMoveConfig {
        max_nearby: 3,
        value_candidate_limit: Some(1),
        target: VariableTargetConfig::default(),
    });

    let selector = build_scalar_move_selector(Some(&config), &scalar_variables, None);
    let moves: Vec<_> = selector.iter_moves(&director).collect();
    let values: Vec<_> = moves
        .iter()
        .map(|mov| match mov {
            crate::heuristic::r#move::ScalarMoveUnion::Change(change) => {
                change.to_value().copied()
            }
            other => panic!("expected scalar change, got {other:?}"),
        })
        .collect();

    assert_eq!(values, vec![Some(2)]);
}

#[test]
fn nearby_swap_filters_same_value_candidates_before_limiting() {
    let director = create_director(Schedule {
        workers: vec![0, 1],
        shifts: vec![
            Shift {
                worker: Some(0),
                allowed_workers: vec![1, 2],
            },
            Shift {
                worker: Some(0),
                allowed_workers: vec![2],
            },
            Shift {
                worker: Some(1),
                allowed_workers: vec![],
            },
        ],
        score: None,
    });

    let scalar_variables = vec![ScalarVariableContext::new(
        0,
        0,
        "Shift",
        shift_count,
        "worker",
        get_worker,
        set_worker,
        ValueSource::SolutionCount {
            count_fn: worker_count,
            provider_index: 0,
        },
        true,
    )
    .with_nearby_entity_candidates(nearby_shift_candidates)
    .with_nearby_entity_distance_meter(nearby_worker_entity_distance)];

    let config = MoveSelectorConfig::NearbySwapMoveSelector(NearbySwapMoveConfig {
        max_nearby: 1,
        target: VariableTargetConfig {
            entity_class: Some("Shift".to_string()),
            variable_name: Some("worker".to_string()),
        },
    });

    let selector = build_scalar_move_selector(Some(&config), &scalar_variables, None);
    let moves: Vec<_> = selector.iter_moves(&director).collect();

    let swap_pairs: Vec<_> = moves
        .iter()
        .map(|mov| match mov {
            crate::heuristic::r#move::ScalarMoveUnion::Swap(swap) => {
                (swap.left_entity_index(), swap.right_entity_index())
            }
            other => panic!("expected nearby swap move, got {other:?}"),
        })
        .collect();

    assert_eq!(swap_pairs, vec![(0, 2), (1, 2)]);
    assert!(moves.iter().all(|mov| mov.is_doable(&director)));
}

#[test]
fn ruin_recreate_skips_required_entities_without_recreate_values() {
    let director = create_director(Schedule {
        workers: vec![],
        shifts: vec![Shift {
            worker: Some(0),
            allowed_workers: vec![],
        }],
        score: None,
    });
    let scalar_variables = vec![ScalarVariableContext::new(
        0,
        0,
        "Shift",
        shift_count,
        "worker",
        get_worker,
        set_worker,
        ValueSource::EntitySlice {
            values_for_entity: allowed_workers,
        },
        false,
    )];
    let config = MoveSelectorConfig::RuinRecreateMoveSelector(RuinRecreateMoveSelectorConfig {
        min_ruin_count: 1,
        max_ruin_count: 1,
        moves_per_step: Some(4),
        value_candidate_limit: None,
        recreate_heuristic_type: RecreateHeuristicType::FirstFit,
        target: VariableTargetConfig {
            entity_class: Some("Shift".to_string()),
            variable_name: Some("worker".to_string()),
        },
    });

    let selector = build_scalar_move_selector(Some(&config), &scalar_variables, None);
    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert!(moves.is_empty());
}

#[test]
fn ruin_recreate_honors_configured_random_seed() {
    fn batches(seed: Option<u64>) -> Vec<Vec<usize>> {
        let director = create_director(Schedule {
            workers: vec![0, 1, 2],
            shifts: (0..8)
                .map(|_| Shift {
                    worker: Some(0),
                    allowed_workers: vec![0, 1, 2],
                })
                .collect(),
            score: None,
        });
        let scalar_variables = vec![ScalarVariableContext::new(
            0,
            0,
            "Shift",
            shift_count,
            "worker",
            get_worker,
            set_worker,
            ValueSource::SolutionCount {
                count_fn: worker_count,
                provider_index: 0,
            },
            false,
        )];
        let config = MoveSelectorConfig::RuinRecreateMoveSelector(RuinRecreateMoveSelectorConfig {
            min_ruin_count: 1,
            max_ruin_count: 3,
            moves_per_step: Some(16),
            value_candidate_limit: None,
            recreate_heuristic_type: RecreateHeuristicType::FirstFit,
            target: VariableTargetConfig {
                entity_class: Some("Shift".to_string()),
                variable_name: Some("worker".to_string()),
            },
        });
        let selector = build_scalar_move_selector(Some(&config), &scalar_variables, seed);

        selector
            .iter_moves(&director)
            .map(|mov| {
                assert!(matches!(
                    mov,
                    crate::heuristic::r#move::ScalarMoveUnion::RuinRecreate(_)
                ));
                mov.entity_indices().to_vec()
            })
            .collect()
    }

    let first = batches(Some(17));
    let repeat = batches(Some(17));
    let changed = batches(Some(18));

    assert_eq!(first, repeat);
    assert_ne!(first, changed);
}

#[test]
fn ruin_recreate_do_move_preserves_required_assignment_when_recreate_values_are_empty() {
    let mut director = create_director(Schedule {
        workers: vec![],
        shifts: vec![Shift {
            worker: Some(0),
            allowed_workers: vec![],
        }],
        score: None,
    });
    let mov = crate::heuristic::r#move::RuinRecreateMove::new(
        &[0],
        get_worker,
        set_worker,
        0,
        0,
        "worker",
        crate::heuristic::r#move::ScalarRecreateValueSource::EntitySlice {
            values_for_entity: allowed_workers,
            variable_index: 0,
            value_candidate_limit: None,
        },
        RecreateHeuristicType::FirstFit,
        false,
    );

    assert!(!mov.is_doable(&director));
    mov.do_move(&mut director);

    assert_eq!(director.working_solution().shifts[0].worker, Some(0));
}

#[test]
fn ruin_recreate_zero_candidate_limit_preserves_required_assignment() {
    let mut director = create_director(Schedule {
        workers: vec![0, 1],
        shifts: vec![Shift {
            worker: Some(0),
            allowed_workers: vec![0, 1],
        }],
        score: None,
    });
    let mov = crate::heuristic::r#move::RuinRecreateMove::new(
        &[0],
        get_worker,
        set_worker,
        0,
        0,
        "worker",
        crate::heuristic::r#move::ScalarRecreateValueSource::EntitySlice {
            values_for_entity: allowed_workers,
            variable_index: 0,
            value_candidate_limit: Some(0),
        },
        RecreateHeuristicType::FirstFit,
        false,
    );

    assert!(!mov.is_doable(&director));
    mov.do_move(&mut director);

    assert_eq!(director.working_solution().shifts[0].worker, Some(0));
}
