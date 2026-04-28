#[test]
fn descriptor_nearby_change_uses_value_distance_meter() {
    let descriptor = descriptor_with_nearby_value_meter();
    let plan = Plan {
        workers: vec![Worker, Worker, Worker],
        tasks: vec![
            Task {
                worker_idx: Some(0),
            },
            Task {
                worker_idx: Some(1),
            },
        ],
        score: None,
    };
    let director = ScoreDirector::simple(plan, descriptor.clone(), |s, _| s.tasks.len());
    let config = MoveSelectorConfig::NearbyChangeMoveSelector(NearbyChangeMoveConfig {
        max_nearby: 1,
        value_candidate_limit: None,
        target: VariableTargetConfig::default(),
    });

    let selector = build_descriptor_move_selector::<Plan>(Some(&config), &descriptor, None);
    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert_eq!(selector.size(&director), 4);
    assert_eq!(moves.len(), 4);
    assert!(moves
        .iter()
        .all(|mov| matches!(mov, super::DescriptorScalarMoveUnion::Change(_))));
    assert!(moves.iter().all(|mov| mov.is_doable(&director)));
}

#[test]
fn descriptor_nearby_change_applies_value_candidate_limit_before_ranking() {
    let descriptor = descriptor_with_nearby_value_meter();
    let plan = Plan {
        workers: vec![Worker, Worker, Worker],
        tasks: vec![Task {
            worker_idx: Some(0),
        }],
        score: None,
    };
    let director = ScoreDirector::simple(plan, descriptor.clone(), |s, _| s.tasks.len());
    let config = MoveSelectorConfig::NearbyChangeMoveSelector(NearbyChangeMoveConfig {
        max_nearby: 3,
        value_candidate_limit: Some(1),
        target: VariableTargetConfig::default(),
    });

    let selector = build_descriptor_move_selector::<Plan>(Some(&config), &descriptor, None);
    let moves: Vec<_> = selector.iter_moves(&director).collect();
    let targets: Vec<_> = moves
        .iter()
        .map(|mov| {
            assert!(matches!(mov, super::DescriptorScalarMoveUnion::Change(_)));
            mov.entity_indices().to_vec()
        })
        .collect();

    assert_eq!(targets, vec![vec![0], vec![0]]);
    assert_eq!(moves.len(), 2);
    assert!(moves.iter().all(|mov| mov.is_doable(&director)));
}

#[test]
#[should_panic(expected = "nearby_change_move selector requires nearby_value_candidates")]
fn descriptor_nearby_change_rejects_distance_meter_without_candidate_hook() {
    let descriptor = descriptor_with_nearby_value_meter_only();
    let config = MoveSelectorConfig::NearbyChangeMoveSelector(NearbyChangeMoveConfig {
        max_nearby: 1,
        value_candidate_limit: None,
        target: VariableTargetConfig::default(),
    });

    let _ = build_descriptor_move_selector::<Plan>(Some(&config), &descriptor, None);
}

#[test]
fn descriptor_nearby_swap_filters_same_value_candidates_before_limiting() {
    let descriptor = descriptor_with_nearby_entity_meter();
    let plan = Plan {
        workers: vec![Worker, Worker],
        tasks: vec![
            Task {
                worker_idx: Some(0),
            },
            Task {
                worker_idx: Some(0),
            },
            Task {
                worker_idx: Some(1),
            },
        ],
        score: None,
    };
    let director = ScoreDirector::simple(plan, descriptor.clone(), |s, _| s.tasks.len());
    let config = MoveSelectorConfig::NearbySwapMoveSelector(NearbySwapMoveConfig {
        max_nearby: 1,
        target: VariableTargetConfig::default(),
    });

    let selector = build_descriptor_move_selector::<Plan>(Some(&config), &descriptor, None);
    let moves: Vec<_> = selector.iter_moves(&director).collect();
    let swap_pairs: Vec<Vec<_>> = moves
        .iter()
        .map(|mov| {
            assert!(matches!(mov, super::DescriptorScalarMoveUnion::Swap(_)));
            mov.entity_indices().to_vec()
        })
        .collect();

    assert_eq!(swap_pairs, vec![vec![0, 2], vec![1, 2]]);
    assert!(moves.iter().all(|mov| mov.is_doable(&director)));
}

#[test]
fn descriptor_swap_move_uses_indexed_legality_after_generation() {
    let descriptor = restricted_descriptor_with_variable(restricted_panic_after_index_variable());
    let plan = RestrictedPlan {
        workers: vec![Worker, Worker, Worker],
        tasks: vec![
            RestrictedTask {
                worker_idx: Some(0),
                allowed_workers: vec![0, 2],
            },
            RestrictedTask {
                worker_idx: Some(1),
                allowed_workers: vec![1, 2],
            },
            RestrictedTask {
                worker_idx: Some(2),
                allowed_workers: vec![0, 1, 2],
            },
        ],
        score: None,
    };
    let director = ScoreDirector::simple(plan, descriptor.clone(), |s, _| s.tasks.len());
    let config = MoveSelectorConfig::SwapMoveSelector(SwapMoveConfig {
        target: VariableTargetConfig::default(),
    });
    let selector =
        build_descriptor_move_selector::<RestrictedPlan>(Some(&config), &descriptor, None);
    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert_eq!(moves.len(), 2);
    let _panic_guard = RestrictedAllowedWorkersPanicGuard::enable();
    assert!(moves.iter().all(|mov| mov.is_doable(&director)));
}

#[test]
fn descriptor_swap_selector_prunes_illegal_entity_ranges() {
    let descriptor = restricted_descriptor();
    let plan = RestrictedPlan {
        workers: vec![Worker, Worker, Worker],
        tasks: vec![
            RestrictedTask {
                worker_idx: Some(0),
                allowed_workers: vec![0, 2],
            },
            RestrictedTask {
                worker_idx: Some(1),
                allowed_workers: vec![1, 2],
            },
            RestrictedTask {
                worker_idx: Some(2),
                allowed_workers: vec![0, 1, 2],
            },
        ],
        score: None,
    };
    let director = ScoreDirector::simple(plan, descriptor.clone(), |s, _| s.tasks.len());
    let config = MoveSelectorConfig::SwapMoveSelector(SwapMoveConfig {
        target: VariableTargetConfig::default(),
    });

    let selector =
        build_descriptor_move_selector::<RestrictedPlan>(Some(&config), &descriptor, None);
    let moves: Vec<_> = selector.iter_moves(&director).collect();
    let swap_pairs: Vec<_> = moves
        .iter()
        .map(|mov| {
            assert!(matches!(mov, super::DescriptorScalarMoveUnion::Swap(_)));
            mov.entity_indices().to_vec()
        })
        .collect();

    assert_eq!(selector.size(&director), 2);
    assert_eq!(swap_pairs, vec![vec![0, 2], vec![1, 2]]);
    assert!(moves.iter().all(|mov| mov.is_doable(&director)));
}

#[test]
fn descriptor_swap_selector_emits_complete_assignment_swaps_without_domain() {
    let descriptor = descriptor_without_value_range();
    let plan = Plan {
        workers: Vec::new(),
        tasks: vec![
            Task {
                worker_idx: Some(0),
            },
            Task {
                worker_idx: Some(1),
            },
        ],
        score: None,
    };
    let director = ScoreDirector::simple(plan, descriptor.clone(), |s, _| s.tasks.len());
    let config = MoveSelectorConfig::SwapMoveSelector(SwapMoveConfig {
        target: VariableTargetConfig::default(),
    });

    let selector = build_descriptor_move_selector::<Plan>(Some(&config), &descriptor, None);
    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert_eq!(selector.size(&director), 1);
    assert_eq!(moves.len(), 1);
    assert!(matches!(
        moves[0],
        super::DescriptorScalarMoveUnion::Swap(_)
    ));
    assert_eq!(moves[0].entity_indices(), [0, 1]);
    assert!(moves[0].is_doable(&director));
}

#[test]
fn descriptor_swap_selector_rejects_explicit_empty_domain() {
    let descriptor = descriptor_with_empty_countable_range();
    let plan = Plan {
        workers: Vec::new(),
        tasks: vec![
            Task {
                worker_idx: Some(0),
            },
            Task {
                worker_idx: Some(1),
            },
        ],
        score: None,
    };
    let director = ScoreDirector::simple(plan, descriptor.clone(), |s, _| s.tasks.len());
    let config = MoveSelectorConfig::SwapMoveSelector(SwapMoveConfig {
        target: VariableTargetConfig::default(),
    });

    let selector = build_descriptor_move_selector::<Plan>(Some(&config), &descriptor, None);
    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert_eq!(selector.size(&director), 0);
    assert!(moves.is_empty());
}

#[test]
fn descriptor_nearby_swap_prunes_illegal_entity_ranges_before_limiting() {
    let descriptor = restricted_descriptor_with_nearby_entity_meter();
    let plan = RestrictedPlan {
        workers: vec![Worker, Worker, Worker],
        tasks: vec![
            RestrictedTask {
                worker_idx: Some(0),
                allowed_workers: vec![0, 2],
            },
            RestrictedTask {
                worker_idx: Some(1),
                allowed_workers: vec![1, 2],
            },
            RestrictedTask {
                worker_idx: Some(2),
                allowed_workers: vec![0, 1, 2],
            },
        ],
        score: None,
    };
    let director = ScoreDirector::simple(plan, descriptor.clone(), |s, _| s.tasks.len());
    let config = MoveSelectorConfig::NearbySwapMoveSelector(NearbySwapMoveConfig {
        max_nearby: 1,
        target: VariableTargetConfig::default(),
    });

    let selector =
        build_descriptor_move_selector::<RestrictedPlan>(Some(&config), &descriptor, None);
    let moves: Vec<_> = selector.iter_moves(&director).collect();
    let swap_pairs: Vec<_> = moves
        .iter()
        .map(|mov| {
            assert!(matches!(mov, super::DescriptorScalarMoveUnion::Swap(_)));
            mov.entity_indices().to_vec()
        })
        .collect();

    assert_eq!(selector.size(&director), 2);
    assert_eq!(swap_pairs, vec![vec![0, 2], vec![1, 2]]);
    assert!(moves.iter().all(|mov| mov.is_doable(&director)));
}

#[test]
fn descriptor_swap_tabu_identity_is_direction_stable() {
    let descriptor = descriptor();
    let plan = Plan {
        workers: vec![Worker, Worker],
        tasks: vec![
            Task {
                worker_idx: Some(0),
            },
            Task {
                worker_idx: Some(1),
            },
        ],
        score: None,
    };
    let mut director = ScoreDirector::simple(plan, descriptor.clone(), |s, _| s.tasks.len());
    let config = MoveSelectorConfig::SwapMoveSelector(SwapMoveConfig {
        target: VariableTargetConfig::default(),
    });
    let selector = build_descriptor_move_selector::<Plan>(Some(&config), &descriptor, None);
    let moves: Vec<_> = selector.iter_moves(&director).collect();
    let forward = moves
        .iter()
        .find(|mov| mov.entity_indices() == [0, 1])
        .expect("forward descriptor swap should be generated");
    let forward_signature = forward.tabu_signature(&director);

    forward.do_move(&mut director);

    let reverse_selector = build_descriptor_move_selector::<Plan>(Some(&config), &descriptor, None);
    let reverse_moves: Vec<_> = reverse_selector.iter_moves(&director).collect();
    let reverse = reverse_moves
        .iter()
        .find(|mov| mov.entity_indices() == [0, 1])
        .expect("reverse descriptor swap should be generated");
    let reverse_signature = reverse.tabu_signature(&director);

    assert_eq!(forward_signature.move_id, forward_signature.undo_move_id);
    assert_eq!(forward_signature.move_id, reverse_signature.move_id);
}
