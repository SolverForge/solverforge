
#[test]
fn descriptor_ruin_recreate_first_fit_reassigns_to_first_improving_value() {
    let descriptor = descriptor();
    let plan = Plan {
        workers: vec![Worker, Worker, Worker],
        tasks: vec![Task {
            worker_idx: Some(1),
        }],
        score: None,
    };
    let mut director = PlanScoreDirector::with_mode(
        plan,
        descriptor.clone(),
        PlanScoreMode::ByWorker {
            unassigned_score: 0,
            assigned_scores: [-5, -1, 7],
        },
    );
    let config = MoveSelectorConfig::RuinRecreateMoveSelector(RuinRecreateMoveSelectorConfig {
        min_ruin_count: 1,
        max_ruin_count: 1,
        moves_per_step: Some(1),
        value_candidate_limit: Some(3),
        recreate_heuristic_type: RecreateHeuristicType::FirstFit,
        target: VariableTargetConfig::default(),
    });

    let selector = build_descriptor_move_selector::<Plan>(Some(&config), &descriptor, None);
    let moves: Vec<_> = selector.iter_moves(&director).collect();
    assert_eq!(moves.len(), 1);

    moves[0].do_move(&mut director);

    assert_eq!(director.working_solution().tasks[0].worker_idx, Some(2));
}

#[test]
fn descriptor_ruin_recreate_cheapest_insertion_picks_best_value() {
    let descriptor = descriptor();
    let plan = Plan {
        workers: vec![Worker, Worker, Worker],
        tasks: vec![Task {
            worker_idx: Some(0),
        }],
        score: None,
    };
    let mut director = PlanScoreDirector::with_mode(
        plan,
        descriptor.clone(),
        PlanScoreMode::ByWorker {
            unassigned_score: 0,
            assigned_scores: [2, 7, 3],
        },
    );
    let config = MoveSelectorConfig::RuinRecreateMoveSelector(RuinRecreateMoveSelectorConfig {
        min_ruin_count: 1,
        max_ruin_count: 1,
        moves_per_step: Some(1),
        value_candidate_limit: Some(3),
        recreate_heuristic_type: RecreateHeuristicType::CheapestInsertion,
        target: VariableTargetConfig::default(),
    });

    let selector = build_descriptor_move_selector::<Plan>(Some(&config), &descriptor, None);
    let moves: Vec<_> = selector.iter_moves(&director).collect();
    assert_eq!(moves.len(), 1);

    moves[0].do_move(&mut director);

    assert_eq!(director.working_solution().tasks[0].worker_idx, Some(1));
}

#[test]
fn descriptor_ruin_recreate_skips_required_entities_without_recreate_values() {
    let descriptor = descriptor_with_allows_unassigned(false);
    let plan = Plan {
        workers: vec![],
        tasks: vec![Task {
            worker_idx: Some(0),
        }],
        score: None,
    };
    let director = PlanScoreDirector::new(plan, descriptor.clone());
    let config = MoveSelectorConfig::RuinRecreateMoveSelector(RuinRecreateMoveSelectorConfig {
        min_ruin_count: 1,
        max_ruin_count: 1,
        moves_per_step: Some(4),
        value_candidate_limit: None,
        recreate_heuristic_type: RecreateHeuristicType::FirstFit,
        target: VariableTargetConfig::default(),
    });

    let selector = build_descriptor_move_selector::<Plan>(Some(&config), &descriptor, None);
    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert!(moves.is_empty());
}

#[test]
fn descriptor_ruin_recreate_honors_configured_random_seed() {
    fn batches(seed: Option<u64>) -> Vec<Vec<usize>> {
        let descriptor = descriptor();
        let plan = Plan {
            workers: vec![Worker, Worker, Worker],
            tasks: (0..8)
                .map(|_| Task {
                    worker_idx: Some(0),
                })
                .collect(),
            score: None,
        };
        let director = PlanScoreDirector::new(plan, descriptor.clone());
        let config = MoveSelectorConfig::RuinRecreateMoveSelector(RuinRecreateMoveSelectorConfig {
            min_ruin_count: 1,
            max_ruin_count: 3,
            moves_per_step: Some(16),
            value_candidate_limit: None,
            recreate_heuristic_type: RecreateHeuristicType::FirstFit,
            target: VariableTargetConfig::default(),
        });
        let selector = build_descriptor_move_selector::<Plan>(Some(&config), &descriptor, seed);

        selector
            .iter_moves(&director)
            .map(|mov| {
                assert!(matches!(
                    mov,
                    super::DescriptorMoveUnion::RuinRecreate(_)
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
fn descriptor_ruin_recreate_do_move_preserves_required_assignment_when_recreate_values_are_empty() {
    let descriptor = descriptor_with_allows_unassigned(false);
    let plan = Plan {
        workers: vec![],
        tasks: vec![Task {
            worker_idx: Some(0),
        }],
        score: None,
    };
    let mut director = PlanScoreDirector::new(plan, descriptor.clone());
    let binding = super::bindings::collect_bindings(&descriptor)
        .into_iter()
        .next()
        .expect("descriptor binding");
    let mov = super::DescriptorRuinRecreateMove::new(
        binding,
        &[0],
        descriptor,
        RecreateHeuristicType::FirstFit,
        None,
    );

    assert!(!mov.is_doable(&director));
    mov.do_move(&mut director);

    assert_eq!(director.working_solution().tasks[0].worker_idx, Some(0));
}

#[test]
fn descriptor_ruin_recreate_zero_candidate_limit_preserves_required_assignment() {
    let descriptor = descriptor_with_allows_unassigned(false);
    let plan = Plan {
        workers: vec![Worker, Worker],
        tasks: vec![Task {
            worker_idx: Some(0),
        }],
        score: None,
    };
    let mut director = PlanScoreDirector::new(plan, descriptor.clone());
    let binding = super::bindings::collect_bindings(&descriptor)
        .into_iter()
        .next()
        .expect("descriptor binding");
    let mov = super::DescriptorRuinRecreateMove::new(
        binding,
        &[0],
        descriptor,
        RecreateHeuristicType::FirstFit,
        Some(0),
    );

    assert!(!mov.is_doable(&director));
    mov.do_move(&mut director);

    assert_eq!(director.working_solution().tasks[0].worker_idx, Some(0));
}
