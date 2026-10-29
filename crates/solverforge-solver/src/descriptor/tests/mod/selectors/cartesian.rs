fn director_cursor_indices<C>(
    cursor: &mut C,
    director: &mut ScoreDirector<Plan, ()>,
) -> Vec<crate::heuristic::selector::move_selector::CandidateId>
where
    C: MoveCursor<Plan, super::DescriptorMoveUnion<Plan>>,
{
    let mut indices = Vec::new();
    while let Some(id) = cursor.next_candidate_with_director(director) {
        indices.push(id);
    }
    indices
}

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
    let mut director = ScoreDirector::simple(plan, descriptor.clone(), |s, _| s.tasks.len());
    let config = MoveSelectorConfig::CartesianProductMoveSelector(CartesianProductConfig {
        require_hard_improvement: false,
        selectors: vec![
            MoveSelectorConfig::ChangeMoveSelector(ChangeMoveConfig {
                selection_order: None,
                selection_metric: None,
                value_candidate_limit: None,
                target: VariableTargetConfig::default(),
            }),
            MoveSelectorConfig::SwapMoveSelector(SwapMoveConfig {
                selection_order: None,
                selection_metric: None,
                target: VariableTargetConfig::default(),
            }),
        ],
    });

    let selector = build_descriptor_move_selector::<Plan>(Some(&config), &descriptor, None)
        .expect("descriptor selector should build");
    let mut cursor = selector.open_cursor(&director);
    let indices = director_cursor_indices(&mut cursor, &mut director);

    assert!(!indices.is_empty());
    assert!(indices.iter().all(|&index| matches!(
        cursor.candidate(index),
        Some(MoveCandidateRef::Sequential(_))
    )));
    let signature = cursor
        .candidate(indices[0])
        .expect("descriptor cartesian candidate must remain valid")
        .tabu_signature(&director)
        .into_owned();
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
    let mut director = ScoreDirector::simple(plan, descriptor.clone(), |s, _| s.tasks.len());
    let config = MoveSelectorConfig::CartesianProductMoveSelector(CartesianProductConfig {
        require_hard_improvement: false,
        selectors: vec![
            MoveSelectorConfig::ChangeMoveSelector(ChangeMoveConfig {
                selection_order: None,
                selection_metric: None,
                value_candidate_limit: None,
                target: VariableTargetConfig::default(),
            }),
            MoveSelectorConfig::SwapMoveSelector(SwapMoveConfig {
                selection_order: None,
                selection_metric: None,
                target: VariableTargetConfig::default(),
            }),
        ],
    });

    let selector = build_descriptor_move_selector::<Plan>(Some(&config), &descriptor, None)
        .expect("descriptor selector should build");
    let filtered = FilteringMoveSelector::new(selector, keep_all_descriptor_cartesian_candidates);
    let mut cursor = filtered.open_cursor(&director);
    let indices = director_cursor_indices(&mut cursor, &mut director);

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
fn descriptor_cartesian_rejects_score_seeking_left_child() {
    let descriptor = descriptor();
    let config = MoveSelectorConfig::CartesianProductMoveSelector(CartesianProductConfig {
        require_hard_improvement: false,
        selectors: vec![
            MoveSelectorConfig::RuinRecreateMoveSelector(RuinRecreateMoveSelectorConfig::default()),
            MoveSelectorConfig::ChangeMoveSelector(ChangeMoveConfig {
                selection_order: None,
                selection_metric: None,
                value_candidate_limit: None,
                target: VariableTargetConfig::default(),
            }),
        ],
    });

    let error = match build_descriptor_move_selector::<Plan>(Some(&config), &descriptor, None) {
        Ok(_) => panic!("selector must be rejected"),
        Err(error) => error,
    };
    assert!(
        error
            .to_string()
            .contains("cartesian_product left child cannot contain ruin_recreate_move_selector"),
        "{error}"
    );
}

#[test]
fn descriptor_selector_rejects_selectors_owned_by_the_canonical_runtime() {
    let descriptor = descriptor();
    for config in [
        MoveSelectorConfig::ListChangeMoveSelector(
            solverforge_config::ListChangeMoveConfig::default(),
        ),
        MoveSelectorConfig::CartesianProductMoveSelector(CartesianProductConfig {
            require_hard_improvement: false,
            selectors: Vec::new(),
        }),
    ] {
        let error = match build_descriptor_move_selector::<Plan>(Some(&config), &descriptor, None) {
            Ok(_) => panic!("selector must be rejected"),
            Err(error) => error,
        };
        assert!(
            matches!(error, crate::RuntimeBuildError::Declaration { .. }),
            "{error}"
        );
    }
}
