fn grouped_worker_candidates(
    _solution: &MixedPlan,
    limits: crate::builder::ScalarGroupLimits,
) -> Vec<crate::builder::ScalarGroupCandidate> {
    assert_eq!(limits.value_candidate_limit, Some(4));
    assert_eq!(limits.group_candidate_limit, None);
    assert_eq!(limits.max_moves_per_step, Some(8));
    vec![crate::builder::ScalarGroupCandidate::new(
        "worker_pair",
        vec![crate::builder::ScalarGroupEdit::set_scalar(
            0,
            0,
            "worker",
            Some(1),
        )],
    )]
}

fn illegal_grouped_worker_candidates(
    _solution: &MixedPlan,
    _limits: crate::builder::ScalarGroupLimits,
) -> Vec<crate::builder::ScalarGroupCandidate> {
    vec![crate::builder::ScalarGroupCandidate::new(
        "illegal",
        vec![crate::builder::ScalarGroupEdit::set_scalar(
            0,
            0,
            "worker",
            Some(99),
        )],
    )]
}

fn duplicate_grouped_worker_candidates(
    _solution: &MixedPlan,
    _limits: crate::builder::ScalarGroupLimits,
) -> Vec<crate::builder::ScalarGroupCandidate> {
    vec![crate::builder::ScalarGroupCandidate::new(
        "duplicate",
        vec![
            crate::builder::ScalarGroupEdit::set_scalar(0, 0, "worker", Some(0)),
            crate::builder::ScalarGroupEdit::set_scalar(0, 0, "worker", Some(1)),
        ],
    )]
}

fn model_with_group(
    provider: crate::builder::context::ScalarGroupCandidateProvider<MixedPlan>,
) -> crate::builder::ModelContext<MixedPlan, usize, NoopMeter, NoopMeter> {
    let ctx = scalar_context();
    scalar_only_model().with_scalar_groups(vec![crate::builder::ScalarGroupContext::new(
        "worker_group",
        vec![crate::builder::ScalarGroupMember::from_scalar_context(ctx)],
        provider,
    )])
}

#[test]
fn grouped_scalar_selector_builds_one_compound_candidate() {
    let descriptor = descriptor(true);
    let director = create_director(
        MixedPlan {
            shifts: vec![Shift { worker: Some(0) }, Shift { worker: Some(0) }],
            vehicles: vec![],
            score: None,
        },
        descriptor,
    );
    let model = model_with_group(grouped_worker_candidates);
    let config = MoveSelectorConfig::GroupedScalarMoveSelector(
        solverforge_config::GroupedScalarMoveSelectorConfig {
            group_name: "worker_group".to_string(),
            value_candidate_limit: Some(4),
            max_moves_per_step: Some(8),
        },
    );

    let selector = build_move_selector(Some(&config), &model, None);
    let mut cursor = selector.open_cursor(&director);
    let first = cursor
        .next_candidate()
        .expect("grouped scalar candidate should be exposed");
    assert!(cursor.next_candidate().is_none());
    let mov = cursor.take_candidate(first);
    assert_eq!(mov.variable_name(), "compound_scalar");
    assert!(mov.is_doable(&director));
}

#[test]
fn grouped_scalar_selector_filters_illegal_and_duplicate_edits() {
    let descriptor = descriptor(true);
    let director = create_director(
        MixedPlan {
            shifts: vec![Shift { worker: Some(0) }, Shift { worker: Some(0) }],
            vehicles: vec![],
            score: None,
        },
        descriptor,
    );

    for provider in [
        illegal_grouped_worker_candidates
            as crate::builder::context::ScalarGroupCandidateProvider<MixedPlan>,
        duplicate_grouped_worker_candidates
            as crate::builder::context::ScalarGroupCandidateProvider<MixedPlan>,
    ] {
        let model = model_with_group(provider);
        let config = MoveSelectorConfig::GroupedScalarMoveSelector(
            solverforge_config::GroupedScalarMoveSelectorConfig {
                group_name: "worker_group".to_string(),
                value_candidate_limit: None,
                max_moves_per_step: Some(8),
            },
        );
        let selector = build_move_selector(Some(&config), &model, None);
        let mut cursor = selector.open_cursor(&director);
        assert!(cursor.next_candidate().is_none());
    }
}
