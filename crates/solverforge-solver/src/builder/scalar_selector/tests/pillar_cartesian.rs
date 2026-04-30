
#[test]
fn pillar_change_uses_public_pillar_semantics() {
    let director = create_director(Schedule {
        workers: vec![0, 1, 2],
        shifts: vec![
            Shift {
                worker: Some(0),
                allowed_workers: vec![0, 1, 2],
            },
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
    )];

    let config = MoveSelectorConfig::PillarChangeMoveSelector(PillarChangeMoveConfig {
        minimum_sub_pillar_size: 0,
        maximum_sub_pillar_size: 0,
        value_candidate_limit: None,
        target: VariableTargetConfig {
            entity_class: Some("Shift".to_string()),
            variable_name: Some("worker".to_string()),
        },
    });

    let selector = build_scalar_move_selector(Some(&config), &scalar_variables, None);
    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert_eq!(moves.len(), 2);
    assert!(moves.iter().all(|mov| matches!(
        mov,
        crate::heuristic::r#move::ScalarMoveUnion::PillarChange(_)
    )));
}

#[test]
fn pillar_change_intersects_entity_slice_domains() {
    let director = create_director(Schedule {
        workers: vec![0, 1, 2],
        shifts: vec![
            Shift {
                worker: Some(0),
                allowed_workers: vec![0, 1, 2],
            },
            Shift {
                worker: Some(0),
                allowed_workers: vec![0, 2],
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
        ValueSource::EntitySlice {
            values_for_entity: allowed_workers,
        },
        true,
    )];
    let config = MoveSelectorConfig::PillarChangeMoveSelector(PillarChangeMoveConfig {
        minimum_sub_pillar_size: 0,
        maximum_sub_pillar_size: 0,
        value_candidate_limit: None,
        target: VariableTargetConfig {
            entity_class: Some("Shift".to_string()),
            variable_name: Some("worker".to_string()),
        },
    });

    let selector = build_scalar_move_selector(Some(&config), &scalar_variables, None);
    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert_eq!(moves.len(), 1);
    assert!(moves.iter().all(|mov| mov.is_doable(&director)));
    assert!(matches!(
        &moves[0],
        crate::heuristic::r#move::ScalarMoveUnion::PillarChange(change)
            if change.to_value() == Some(&2)
    ));
}

#[test]
fn pillar_swap_prunes_illegal_entity_slice_partners() {
    let director = create_director(Schedule {
        workers: vec![0, 1, 2],
        shifts: vec![
            Shift {
                worker: Some(0),
                allowed_workers: vec![0, 2],
            },
            Shift {
                worker: Some(0),
                allowed_workers: vec![0, 2],
            },
            Shift {
                worker: Some(1),
                allowed_workers: vec![1, 2],
            },
            Shift {
                worker: Some(1),
                allowed_workers: vec![1, 2],
            },
            Shift {
                worker: Some(2),
                allowed_workers: vec![0, 1, 2],
            },
            Shift {
                worker: Some(2),
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
        ValueSource::EntitySlice {
            values_for_entity: allowed_workers,
        },
        true,
    )];
    let config = MoveSelectorConfig::PillarSwapMoveSelector(PillarSwapMoveConfig {
        minimum_sub_pillar_size: 0,
        maximum_sub_pillar_size: 0,
        target: VariableTargetConfig {
            entity_class: Some("Shift".to_string()),
            variable_name: Some("worker".to_string()),
        },
    });

    let selector = build_scalar_move_selector(Some(&config), &scalar_variables, None);
    let moves: Vec<_> = selector.iter_moves(&director).collect();

    let mut swap_pairs = Vec::new();
    for mov in &moves {
        assert!(mov.is_doable(&director));
        if let crate::heuristic::r#move::ScalarMoveUnion::PillarSwap(swap) = mov {
            let left_value =
                get_worker(director.working_solution(), swap.left_indices()[0], 0).unwrap();
            let right_value =
                get_worker(director.working_solution(), swap.right_indices()[0], 0).unwrap();
            swap_pairs.push((left_value, right_value));
        }
    }
    swap_pairs.sort_unstable();

    assert_eq!(swap_pairs, vec![(0, 2), (1, 2)]);
}

fn keep_all_cartesian_scalar_candidates(
    candidate: MoveCandidateRef<
        '_,
        Schedule,
        crate::heuristic::r#move::ScalarMoveUnion<Schedule, usize>,
    >,
) -> bool {
    matches!(candidate, MoveCandidateRef::Sequential(_))
}

#[test]
fn scalar_builder_cartesian_selector_survives_filtering_wrapper() {
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
        ValueSource::EntitySlice {
            values_for_entity: allowed_workers,
        },
        true,
    )];
    let config = MoveSelectorConfig::CartesianProductMoveSelector(CartesianProductConfig {
        require_hard_improvement: false,
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

    let selector = build_scalar_move_selector(Some(&config), &scalar_variables, None);
    let filtered = FilteringMoveSelector::new(selector, keep_all_cartesian_scalar_candidates);
    let mut cursor = filtered.open_cursor(&director);
    let indices = collect_cursor_indices::<
        Schedule,
        crate::heuristic::r#move::ScalarMoveUnion<Schedule, usize>,
        _,
    >(&mut cursor);

    assert!(!indices.is_empty());
    assert!(indices.iter().all(|&index| matches!(
        cursor.candidate(index),
        Some(MoveCandidateRef::Sequential(_))
    )));
    assert!(cursor.take_candidate(indices[0]).is_doable(&director));
}
