
#[test]
fn builds_solution_count_scalar_selectors_without_descriptor_bindings() {
    let director = create_director(Schedule {
        workers: vec![0, 1, 2],
        shifts: vec![
            Shift {
                worker: Some(0),
                allowed_workers: vec![0, 2],
            },
            Shift {
                worker: Some(1),
                allowed_workers: vec![1],
            },
        ],
        score: None,
    });

    let scalar_variables = vec![ScalarVariableSlot::new(
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
    )];

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

    let selector = build_scalar_move_selector::<Schedule>(Some(&config), &scalar_variables, None);
    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert_eq!(selector.size(&director), 9);
    assert_eq!(moves.len(), 9);
    assert_eq!(
        moves.iter()
            .filter(|mov| matches!(mov, crate::heuristic::r#move::ScalarMoveUnion::Change(change) if change.to_value().is_none()))
            .count(),
        2
    );
}

#[test]
fn filters_change_moves_against_entity_slice_candidates() {
    let director = create_director(Schedule {
        workers: vec![0, 1, 2],
        shifts: vec![
            Shift {
                worker: Some(0),
                allowed_workers: vec![0, 2],
            },
            Shift {
                worker: Some(1),
                allowed_workers: vec![1],
            },
        ],
        score: None,
    });

    let scalar_variables = vec![ScalarVariableSlot::new(
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
        true,
    )];

    let config = MoveSelectorConfig::ChangeMoveSelector(ChangeMoveConfig {
        value_candidate_limit: None,
        target: VariableTargetConfig {
            entity_class: Some("Shift".to_string()),
            variable_name: Some("worker".to_string()),
        },
    });

    let selector = build_scalar_move_selector(Some(&config), &scalar_variables, None);
    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert_eq!(selector.size(&director), 5);
    assert_eq!(moves.len(), 5);
    assert_eq!(
        moves.iter()
            .filter(|mov| matches!(mov, crate::heuristic::r#move::ScalarMoveUnion::Change(change) if change.to_value().is_none()))
            .count(),
        2
    );
}

#[test]
fn change_selector_ignores_construction_value_order_key() {
    fn live_worker_order(
        solution: &Schedule,
        entity_index: usize,
        _variable_index: usize,
        value: usize,
    ) -> Option<i64> {
        let preferred = solution.shifts[entity_index].worker.unwrap_or(usize::MAX);
        Some(match value {
            value if value == preferred => 0,
            1 => 1,
            2 => 2,
            _ => 3,
        })
    }

    let director = create_director(Schedule {
        workers: vec![0, 1, 2],
        shifts: vec![Shift {
            worker: Some(2),
            allowed_workers: vec![0, 1, 2],
        }],
        score: None,
    });

    let scalar_variables = vec![ScalarVariableSlot::new(
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
        true,
    )
    .with_construction_value_order_key(live_worker_order)];

    let config = MoveSelectorConfig::ChangeMoveSelector(ChangeMoveConfig {
        value_candidate_limit: None,
        target: VariableTargetConfig {
            entity_class: Some("Shift".to_string()),
            variable_name: Some("worker".to_string()),
        },
    });

    let selector = build_scalar_move_selector(Some(&config), &scalar_variables, None);
    let values = selector
        .iter_moves(&director)
        .map(|mov| match mov {
            crate::heuristic::r#move::ScalarMoveUnion::Change(change) => change.to_value().copied(),
            other => panic!("expected change move, got {other:?}"),
        })
        .collect::<Vec<_>>();

    assert_eq!(values, vec![Some(0), Some(1), Some(2), None]);
}

#[test]
fn filters_swap_moves_against_entity_slice_candidates_before_evaluation() {
    let director = create_director(Schedule {
        workers: vec![0, 1, 2],
        shifts: vec![
            Shift {
                worker: Some(0),
                allowed_workers: vec![0, 1],
            },
            Shift {
                worker: Some(1),
                allowed_workers: vec![0, 1],
            },
            Shift {
                worker: Some(2),
                allowed_workers: vec![2],
            },
        ],
        score: None,
    });

    let scalar_variables = vec![ScalarVariableSlot::new(
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
        true,
    )];
    let config = MoveSelectorConfig::SwapMoveSelector(SwapMoveConfig {
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
            other => panic!("expected swap move, got {other:?}"),
        })
        .collect();

    assert_eq!(swap_pairs, vec![(0, 1)]);
    assert!(moves.iter().all(|mov| mov.is_doable(&director)));
}

#[test]
fn swap_selector_emits_complete_assignment_swaps_without_domain() {
    let director = create_director(Schedule {
        workers: vec![],
        shifts: vec![
            Shift {
                worker: Some(0),
                allowed_workers: vec![],
            },
            Shift {
                worker: Some(1),
                allowed_workers: vec![],
            },
        ],
        score: None,
    });

    let scalar_variables = vec![ScalarVariableSlot::new(
        0,
        0,
        "Shift",
        shift_count,
        "worker",
        get_worker,
        set_worker,
        ValueSource::Empty,
        false,
    )];
    let config = MoveSelectorConfig::SwapMoveSelector(SwapMoveConfig {
        target: VariableTargetConfig {
            entity_class: Some("Shift".to_string()),
            variable_name: Some("worker".to_string()),
        },
    });

    let selector = build_scalar_move_selector(Some(&config), &scalar_variables, None);
    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert_eq!(selector.size(&director), 1);
    assert_eq!(moves.len(), 1);
    assert!(matches!(
        &moves[0],
        crate::heuristic::r#move::ScalarMoveUnion::Swap(swap)
            if (swap.left_entity_index(), swap.right_entity_index()) == (0, 1)
    ));
    assert!(moves[0].is_doable(&director));
}

#[test]
fn swap_selector_rejects_explicit_empty_entity_slice_domain() {
    let director = create_director(Schedule {
        workers: vec![],
        shifts: vec![
            Shift {
                worker: Some(0),
                allowed_workers: vec![],
            },
            Shift {
                worker: Some(1),
                allowed_workers: vec![],
            },
        ],
        score: None,
    });

    let scalar_variables = vec![ScalarVariableSlot::new(
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
    let config = MoveSelectorConfig::SwapMoveSelector(SwapMoveConfig {
        target: VariableTargetConfig {
            entity_class: Some("Shift".to_string()),
            variable_name: Some("worker".to_string()),
        },
    });

    let selector = build_scalar_move_selector(Some(&config), &scalar_variables, None);
    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert_eq!(selector.size(&director), 0);
    assert!(moves.is_empty());
}
