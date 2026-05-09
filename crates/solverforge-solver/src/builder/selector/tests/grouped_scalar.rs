fn grouped_worker_candidates(
    _solution: &MixedPlan,
    limits: crate::builder::ScalarGroupLimits,
) -> Vec<crate::builder::ScalarCandidate<MixedPlan>> {
    assert_eq!(limits.value_candidate_limit, Some(4));
    assert_eq!(limits.group_candidate_limit, None);
    assert_eq!(limits.max_moves_per_step, Some(8));
    vec![crate::builder::ScalarCandidate::new(
        "worker_pair",
        vec![ScalarTarget::from_descriptor_index(0, "worker").set(0, Some(1))],
    )]
}

fn illegal_grouped_worker_candidates(
    _solution: &MixedPlan,
    _limits: crate::builder::ScalarGroupLimits,
) -> Vec<crate::builder::ScalarCandidate<MixedPlan>> {
    vec![crate::builder::ScalarCandidate::new(
        "illegal",
        vec![ScalarTarget::from_descriptor_index(0, "worker").set(0, Some(99))],
    )]
}

fn duplicate_grouped_worker_candidates(
    _solution: &MixedPlan,
    _limits: crate::builder::ScalarGroupLimits,
) -> Vec<crate::builder::ScalarCandidate<MixedPlan>> {
    vec![crate::builder::ScalarCandidate::new(
        "duplicate",
        vec![
            ScalarTarget::from_descriptor_index(0, "worker").set(0, Some(0)),
            ScalarTarget::from_descriptor_index(0, "worker").set(0, Some(1)),
        ],
    )]
}

fn model_limited_grouped_worker_candidates(
    _solution: &MixedPlan,
    limits: crate::builder::ScalarGroupLimits,
) -> Vec<crate::builder::ScalarCandidate<MixedPlan>> {
    assert_eq!(limits.value_candidate_limit, Some(5));
    assert_eq!(limits.group_candidate_limit, None);
    assert_eq!(limits.max_moves_per_step, Some(2));
    vec![crate::builder::ScalarCandidate::new(
        "worker_pair",
        vec![ScalarTarget::from_descriptor_index(0, "worker").set(0, Some(1))],
    )]
}

fn model_with_group(
    provider: crate::builder::context::ScalarCandidateProvider<MixedPlan>,
) -> crate::builder::RuntimeModel<MixedPlan, usize, NoopMeter, NoopMeter> {
    model_with_group_limits(provider, crate::builder::ScalarGroupLimits::new())
}

fn model_with_group_limits(
    provider: crate::builder::context::ScalarCandidateProvider<MixedPlan>,
    limits: crate::builder::ScalarGroupLimits,
) -> crate::builder::RuntimeModel<MixedPlan, usize, NoopMeter, NoopMeter> {
    let model = scalar_only_model();
    let scalar_slots = model.scalar_variables().copied().collect::<Vec<_>>();
    let groups = crate::builder::bind_scalar_groups(
        vec![
            ScalarGroup::candidates(
                "worker_group",
                vec![ScalarTarget::from_descriptor_index(0, "worker")],
                provider,
            )
            .with_limits(limits),
        ],
        &scalar_slots,
    );
    model.with_scalar_groups(groups)
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
            require_hard_improvement: true,
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
    assert!(mov.requires_hard_improvement());
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
            as crate::builder::context::ScalarCandidateProvider<MixedPlan>,
        duplicate_grouped_worker_candidates
            as crate::builder::context::ScalarCandidateProvider<MixedPlan>,
    ] {
        let model = model_with_group(provider);
        let config = MoveSelectorConfig::GroupedScalarMoveSelector(
            solverforge_config::GroupedScalarMoveSelectorConfig {
                group_name: "worker_group".to_string(),
                value_candidate_limit: None,
                max_moves_per_step: Some(8),
                require_hard_improvement: false,
            },
        );
        let selector = build_move_selector(Some(&config), &model, None);
        let mut cursor = selector.open_cursor(&director);
        assert!(cursor.next_candidate().is_none());
    }
}

#[test]
fn grouped_scalar_selector_uses_model_owned_value_limit() {
    let descriptor = descriptor(true);
    let director = create_director(
        MixedPlan {
            shifts: vec![Shift { worker: Some(0) }, Shift { worker: Some(0) }],
            vehicles: vec![],
            score: None,
        },
        descriptor,
    );
    let model = model_with_group_limits(
        model_limited_grouped_worker_candidates,
        crate::builder::ScalarGroupLimits {
            value_candidate_limit: Some(5),
            group_candidate_limit: Some(99),
            max_moves_per_step: Some(2),
            ..crate::builder::ScalarGroupLimits::new()
        },
    );
    let config = MoveSelectorConfig::GroupedScalarMoveSelector(
        solverforge_config::GroupedScalarMoveSelectorConfig {
            group_name: "worker_group".to_string(),
            value_candidate_limit: None,
            max_moves_per_step: None,
            require_hard_improvement: false,
        },
    );

    let selector = build_move_selector(Some(&config), &model, None);
    let mut cursor = selector.open_cursor(&director);
    assert!(cursor.next_candidate().is_some());
}
