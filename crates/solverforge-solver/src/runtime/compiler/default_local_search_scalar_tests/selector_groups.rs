use super::*;

#[test]
fn omitted_selector_provenance_keeps_the_explicit_local_search_policy() {
    let local_search = LocalSearchConfig {
        local_search_type: LocalSearchType::AcceptorForager,
        acceptor: Some(AcceptorConfig::HillClimbing {}),
        forager: Some(ForagerConfig::FirstAccepted {}),
        termination: Some(TerminationConfig {
            step_count_limit: Some(17),
            ..TerminationConfig::default()
        }),
        ..LocalSearchConfig::default()
    };
    let config = SolverConfig {
        phases: vec![PhaseConfig::LocalSearch(local_search)],
        ..SolverConfig::default()
    };
    let context = SearchContext::new(descriptor(), dynamic_scalar_model(), config.random_seed)
        .expect("search context should bind");
    let graph = compile_runtime_graph(
        &config,
        RuntimeGraphInput::new(context, NoDynamicExtensions),
    )
    .expect("explicit policy with an omitted selector compiles");
    let expected_selector_count = graph
        .default_bindings()
        .local_search_plan
        .as_ref()
        .expect("the omitted selector has a default declaration")
        .selectors
        .len();
    let executor = super::super::executor::CompiledRuntimeExecutor::new(graph);
    let runner = super::super::executor::CompiledRuntimePhaseRunner::try_new(executor)
        .expect("the explicit phase lowers against the default selector tree");
    let phase = runner.phase_plan_for_test(0);
    let declaration = &phase.children[0];

    assert_eq!(
        declaration.kind,
        "solverforge.runtime.local_search.acceptor_forager"
    );
    assert_eq!(
        phase_trace_value(declaration, "acceptor").as_deref(),
        Some("configured:HillClimbing")
    );
    assert_eq!(
        phase_trace_value(declaration, "forager").as_deref(),
        Some("configured:FirstAccepted")
    );
    assert!(phase_trace_value(declaration, "config")
        .expect("explicit configuration is retained")
        .contains("step_count_limit: Some(17)"));
    assert_eq!(declaration.children.len(), expected_selector_count);
    assert!(declaration
        .children
        .iter()
        .all(|child| { child.kind == "solverforge.runtime.default_local_search.selector" }));
}

#[test]
fn typed_and_dynamic_assignment_groups_hide_scalar_leaves_and_keep_group_policy() {
    for plan in [
        default_plan(static_assignment_model()),
        default_plan(dynamic_assignment_model()),
    ] {
        assert_eq!(
            signature(&plan),
            vec![(
                DefaultLocalSearchSelectorFamily::GroupedScalar,
                DefaultSelectorCapabilityPolicy::DeclaredCapabilities,
            )],
        );
        assert_eq!(
            plan.components.acceptor,
            DefaultLocalSearchAcceptorPolicy::DiversifiedLateAcceptance { history_size: 400 }
        );
        assert_eq!(
            plan.components.forager,
            DefaultLocalSearchForagerPolicy::AcceptedCount { limit: 256 }
        );
        assert!(plan.candidate_trace_plan().is_complete());
    }
}

#[test]
fn assignment_and_candidate_groups_reject_sharing_one_name() {
    let slot = scalar_slot();
    let groups = bind_scalar_groups(
        vec![
            ScalarGroup::candidates(
                "worker_assignment",
                vec![ScalarTarget::from_descriptor_index(0, "worker")],
                |_, limits| {
                    vec![crate::planning::ScalarCandidate::new(
                        "duplicate_probe",
                        vec![ScalarTarget::from_descriptor_index(0, "worker").set(0, Some(1))],
                    )]
                    .into_iter()
                    .take(limits.value_candidate_limit.unwrap_or(usize::MAX))
                    .collect()
                },
            ),
            ScalarGroup::assignment(
                "worker_assignment",
                ScalarTarget::from_descriptor_index(0, "worker"),
            ),
        ],
        &[scalar_slot()],
    );
    let model: Model =
        RuntimeModel::new(vec![VariableSlot::Scalar(slot)]).with_scalar_groups(groups);

    let error = match SearchContext::new(descriptor(), model, None) {
        Err(error) => error,
        Ok(_) => panic!("assignment and callback provider must not share one group name"),
    };
    let message = match error {
        crate::RuntimeBuildError::Declaration { message } => message,
        other => panic!("expected a declaration error, got: {other}"),
    };
    assert!(
        message.contains("scalar group `worker_assignment` is registered more than once"),
        "duplicate group names must be rejected at model resolution: {message}"
    );
}
