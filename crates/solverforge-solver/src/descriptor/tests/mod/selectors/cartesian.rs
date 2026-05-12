
#[test]
fn descriptor_cartesian_builds_composite_moves() {
    let descriptor = descriptor();
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

    let selector = build_descriptor_move_selector::<Plan>(Some(&config), &descriptor, None);
    let mut cursor = selector.open_cursor(&director);
    let indices =
        collect_cursor_indices::<Plan, super::DescriptorMoveUnion<Plan>, _>(&mut cursor);

    assert!(!indices.is_empty());
    assert!(indices.iter().all(|&index| matches!(
        cursor.candidate(index),
        Some(MoveCandidateRef::Sequential(_))
    )));
    let signature = cursor
        .candidate(indices[0])
        .expect("descriptor cartesian candidate must remain valid")
        .tabu_signature(&director);
    assert!(!signature.move_id.is_empty());
    assert!(!signature.entity_tokens.is_empty());
}

fn keep_all_descriptor_cartesian_candidates(
    candidate: MoveCandidateRef<'_, Plan, super::DescriptorMoveUnion<Plan>>,
) -> bool {
    matches!(candidate, MoveCandidateRef::Sequential(_))
}

#[test]
fn descriptor_cartesian_selector_survives_filtering_wrapper() {
    let descriptor = descriptor();
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

    let selector = build_descriptor_move_selector::<Plan>(Some(&config), &descriptor, None);
    let filtered = FilteringMoveSelector::new(selector, keep_all_descriptor_cartesian_candidates);
    let mut cursor = filtered.open_cursor(&director);
    let indices =
        collect_cursor_indices::<Plan, super::DescriptorMoveUnion<Plan>, _>(&mut cursor);

    assert!(!indices.is_empty());
    assert!(indices.iter().all(|&index| matches!(
        cursor.candidate(index),
        Some(MoveCandidateRef::Sequential(_))
    )));
    assert!(cursor
        .candidate(indices[0])
        .is_some_and(|mov| mov.is_doable(&director)));
}

#[test]
#[should_panic(
    expected = "cartesian_product left child cannot contain ruin_recreate_move_selector"
)]
fn descriptor_cartesian_rejects_score_seeking_left_child() {
    let descriptor = descriptor();
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

    let _ = build_descriptor_move_selector::<Plan>(Some(&config), &descriptor, None);
}
