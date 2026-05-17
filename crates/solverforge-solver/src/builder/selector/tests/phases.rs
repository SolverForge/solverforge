
#[test]
#[should_panic(expected = "move selector configuration produced no neighborhoods")]
fn empty_model_does_not_synthesize_scalar_neighborhoods() {
    let _ =
        build_move_selector::<MixedPlan, usize, NoopMeter, NoopMeter>(None, &empty_model(), None);
}

#[test]
fn default_scalar_local_search_uses_scalar_streaming_defaults() {
    let phase = build_local_search::<MixedPlan, usize, NoopMeter, NoopMeter>(
        None,
        &scalar_only_model(),
        Some(7),
    );
    let debug = format!("{phase:?}");

    assert!(debug.contains("SimulatedAnnealing"));
    assert!(debug.contains("accepted_count_limit: 1"));
}

#[test]
fn default_nearby_scalar_local_search_uses_stream_horizon() {
    let phase = build_local_search::<MixedPlan, usize, NoopMeter, NoopMeter>(
        None,
        &nearby_scalar_only_model(),
        Some(7),
    );
    let debug = format!("{phase:?}");

    assert!(debug.contains("SimulatedAnnealing"));
    assert!(debug.contains("accepted_count_limit: 256"));
}

#[test]
fn default_search_profile_uses_one_streaming_phase_for_assignment_groups() {
    let phases = crate::builder::search::defaults::default_local_search_phases(
        &assignment_scalar_model(),
        Some(7),
    );

    assert_eq!(phases.len(), 1);
    let debug = format!("{:?}", phases[0]);
    assert!(debug.contains("AcceptorForager"));
    assert!(!debug.contains("VariableNeighborhoodDescent"));
    assert!(debug.contains("DiversifiedLateAcceptance"));
    assert!(debug.contains("LastStepScoreImproving"));
    assert!(!debug.contains("AcceptedCount"));
}

#[test]
fn default_search_profile_keeps_plain_scalar_to_one_streaming_phase() {
    let phases = crate::builder::search::defaults::default_local_search_phases(
        &scalar_only_model(),
        Some(7),
    );

    assert_eq!(phases.len(), 1);
    assert!(format!("{:?}", phases[0]).contains("AcceptorForager"));
}

#[test]
fn default_list_and_mixed_local_search_use_list_streaming_defaults() {
    let list_phase = build_local_search::<MixedPlan, usize, NoopMeter, NoopMeter>(
        None,
        &list_only_model(),
        None,
    );
    let list_debug = format!("{list_phase:?}");
    assert!(list_debug.contains("LateAcceptance"));
    assert!(list_debug.contains("accepted_count_limit: 256"));

    let mixed_phase =
        build_local_search::<MixedPlan, usize, NoopMeter, NoopMeter>(None, &mixed_model(), None);
    let mixed_debug = format!("{mixed_phase:?}");
    assert!(mixed_debug.contains("LateAcceptance"));
    assert!(mixed_debug.contains("accepted_count_limit: 256"));
}

#[test]
fn explicit_acceptor_and_forager_configs_override_defaults() {
    let config = LocalSearchConfig {
        local_search_type: LocalSearchType::AcceptorForager,
        acceptor: Some(AcceptorConfig::LateAcceptance(LateAcceptanceConfig {
            late_acceptance_size: Some(17),
        })),
        forager: Some(ForagerConfig::FirstBestScoreImproving),
        move_selector: None,
        neighborhoods: Vec::new(),
        termination: None,
    };

    let phase = build_local_search::<MixedPlan, usize, NoopMeter, NoopMeter>(
        Some(&config),
        &scalar_only_model(),
        None,
    );
    let debug = format!("{phase:?}");

    assert!(debug.contains("LateAcceptance"));
    assert!(debug.contains("size: 17"));
    assert!(debug.contains("BestScoreImproving"));
}

#[test]
fn local_search_phase_uses_configured_step_count_limit() {
    let config = LocalSearchConfig {
        local_search_type: LocalSearchType::AcceptorForager,
        acceptor: None,
        forager: None,
        move_selector: None,
        neighborhoods: Vec::new(),
        termination: Some(TerminationConfig {
            step_count_limit: Some(3),
            ..TerminationConfig::default()
        }),
    };

    let phase = build_local_search::<MixedPlan, usize, NoopMeter, NoopMeter>(
        Some(&config),
        &scalar_only_model(),
        None,
    );
    let debug = format!("{phase:?}");

    assert!(debug.contains("step_limit: Some(3)"));
}

#[test]
fn local_search_type_defaults_to_acceptor_forager() {
    let config = LocalSearchConfig::default();
    let phase = build_local_search::<MixedPlan, usize, NoopMeter, NoopMeter>(
        Some(&config),
        &scalar_only_model(),
        Some(7),
    );
    let debug = format!("{phase:?}");

    assert!(debug.contains("AcceptorForager"));
    assert!(debug.contains("SimulatedAnnealing"));
}

#[test]
fn omitted_and_empty_local_search_configs_share_defaults() {
    let config = LocalSearchConfig::default();
    let omitted = build_local_search::<MixedPlan, usize, NoopMeter, NoopMeter>(
        None,
        &nearby_scalar_only_model(),
        Some(7),
    );
    let empty = build_local_search::<MixedPlan, usize, NoopMeter, NoopMeter>(
        Some(&config),
        &nearby_scalar_only_model(),
        Some(7),
    );

    assert_eq!(format!("{omitted:?}"), format!("{empty:?}"));
}

#[test]
fn variable_neighborhood_descent_type_dispatches_under_local_search() {
    let config = LocalSearchConfig {
        local_search_type: LocalSearchType::VariableNeighborhoodDescent,
        neighborhoods: vec![MoveSelectorConfig::ChangeMoveSelector(ChangeMoveConfig {
            value_candidate_limit: None,
            target: VariableTargetConfig::default(),
        })],
        termination: Some(TerminationConfig {
            step_count_limit: Some(4),
            ..TerminationConfig::default()
        }),
        ..LocalSearchConfig::default()
    };

    let phase = build_local_search::<MixedPlan, usize, NoopMeter, NoopMeter>(
        Some(&config),
        &scalar_only_model(),
        None,
    );
    let debug = format!("{phase:?}");

    assert!(debug.contains("VariableNeighborhoodDescent"));
    assert!(debug.contains("step_limit: Some(4)"));
}

#[test]
#[should_panic(expected = "acceptor_forager local_search uses move_selector")]
fn acceptor_forager_local_search_rejects_neighborhoods() {
    let config = LocalSearchConfig {
        neighborhoods: vec![MoveSelectorConfig::ChangeMoveSelector(ChangeMoveConfig {
            value_candidate_limit: None,
            target: VariableTargetConfig::default(),
        })],
        ..LocalSearchConfig::default()
    };

    let _ = build_local_search::<MixedPlan, usize, NoopMeter, NoopMeter>(
        Some(&config),
        &scalar_only_model(),
        None,
    );
}

#[test]
#[should_panic(expected = "variable_neighborhood_descent local_search uses neighborhoods")]
fn variable_neighborhood_descent_rejects_acceptor_forager_fields() {
    let config = LocalSearchConfig {
        local_search_type: LocalSearchType::VariableNeighborhoodDescent,
        acceptor: Some(AcceptorConfig::LateAcceptance(LateAcceptanceConfig {
            late_acceptance_size: Some(17),
        })),
        neighborhoods: vec![MoveSelectorConfig::ChangeMoveSelector(ChangeMoveConfig {
            value_candidate_limit: None,
            target: VariableTargetConfig::default(),
        })],
        ..LocalSearchConfig::default()
    };

    let _ = build_local_search::<MixedPlan, usize, NoopMeter, NoopMeter>(
        Some(&config),
        &scalar_only_model(),
        None,
    );
}

#[test]
#[should_panic(
    expected = "variable_neighborhood_descent local_search requires at least one [[phases.neighborhoods]] block"
)]
fn variable_neighborhood_descent_requires_neighborhoods() {
    let config = LocalSearchConfig {
        local_search_type: LocalSearchType::VariableNeighborhoodDescent,
        ..LocalSearchConfig::default()
    };

    let _ = build_local_search::<MixedPlan, usize, NoopMeter, NoopMeter>(
        Some(&config),
        &scalar_only_model(),
        None,
    );
}
