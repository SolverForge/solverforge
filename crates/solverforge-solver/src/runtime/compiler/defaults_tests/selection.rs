use super::*;

#[test]
fn dynamic_and_native_default_construction_have_equal_cvrp_and_jssp_profiles() {
    for (scenario, expected) in [
        (Scenario::Cvrp, ConstructionHeuristicType::ListClarkeWright),
        (
            Scenario::Jssp,
            ConstructionHeuristicType::ListRegretInsertion,
        ),
        (
            Scenario::Cheapest,
            ConstructionHeuristicType::ListCheapestInsertion,
        ),
    ] {
        let plan = initial_plan();
        let native = resolved_preconstruction_kinds(
            static_model(scenario),
            SolverConfig::default(),
            DefaultPreconstructionStage::ListConstruction,
            &plan,
        );
        let dynamic = resolved_preconstruction_kinds(
            dynamic_model(scenario),
            SolverConfig::default(),
            DefaultPreconstructionStage::ListConstruction,
            &plan,
        );
        let expected = vec![DefaultConstructionStepKind::ListConstruction(expected)];
        assert_eq!(
            native,
            (
                DefaultConstructionStage::Preconstruction(
                    DefaultPreconstructionStage::ListConstruction,
                ),
                expected.clone(),
            ),
            "native profile"
        );
        assert_eq!(dynamic, native, "dynamic profile");
    }
}

#[test]
fn default_list_construction_records_the_declaration_that_chose_it() {
    for (scenario, expected) in [
        (
            Scenario::Cvrp,
            (
                ConstructionHeuristicType::ListClarkeWright,
                DefaultConstructionReason::SavingsDeclared,
            ),
        ),
        (
            Scenario::Jssp,
            (
                ConstructionHeuristicType::ListRegretInsertion,
                DefaultConstructionReason::PrecedenceOrderAndOwnership,
            ),
        ),
        (
            Scenario::Cheapest,
            (
                ConstructionHeuristicType::ListCheapestInsertion,
                DefaultConstructionReason::NoSpecializedListPolicy,
            ),
        ),
    ] {
        for model in [static_model(scenario), dynamic_model(scenario)] {
            let (_, steps) = resolved_preconstruction_steps(
                model,
                SolverConfig::default(),
                DefaultPreconstructionStage::ListConstruction,
                &initial_plan(),
            );
            assert_eq!(
                steps,
                vec![(
                    DefaultConstructionStepKind::ListConstruction(expected.0),
                    expected.1
                )]
            );
        }
    }
}

#[test]
fn empty_savings_routes_stage_clarke_wright_before_postconstruction_kopt() {
    let empty = initial_plan();
    let expected_pre = (
        DefaultConstructionStage::Preconstruction(DefaultPreconstructionStage::ListConstruction),
        vec![DefaultConstructionStepKind::ListConstruction(
            ConstructionHeuristicType::ListClarkeWright,
        )],
    );
    let expected_post = (
        DefaultConstructionStage::PostConstructionKOpt,
        vec![DefaultConstructionStepKind::ListKOpt],
    );

    for model in [static_model(Scenario::Cvrp), dynamic_model(Scenario::Cvrp)] {
        assert_eq!(
            resolved_preconstruction_kinds(
                model.clone(),
                SolverConfig::default(),
                DefaultPreconstructionStage::ListConstruction,
                &empty,
            ),
            expected_pre,
        );
        assert_eq!(
            resolved_postconstruction_kinds(model.clone(), SolverConfig::default(), &empty),
            (DefaultConstructionStage::PostConstructionKOpt, Vec::new(),),
            "K-opt must not be selected from the empty initial route",
        );

        let mut after_construction = empty.clone();
        after_construction.routes[0] = vec![0, 1, 2];
        assert_eq!(
            resolved_postconstruction_kinds(model, SolverConfig::default(), &after_construction,),
            expected_post,
        );
    }
}

#[test]
fn metadata_free_lists_use_cheapest_without_postconstruction_kopt() {
    let plan = initial_plan();
    let expected_pre = (
        DefaultConstructionStage::Preconstruction(DefaultPreconstructionStage::ListConstruction),
        vec![DefaultConstructionStepKind::ListConstruction(
            ConstructionHeuristicType::ListCheapestInsertion,
        )],
    );
    let expected_post = (DefaultConstructionStage::PostConstructionKOpt, Vec::new());

    for model in [
        static_model(Scenario::Cheapest),
        dynamic_model(Scenario::Cheapest),
    ] {
        assert_eq!(
            resolved_preconstruction_kinds(
                model.clone(),
                SolverConfig::default(),
                DefaultPreconstructionStage::ListConstruction,
                &plan,
            ),
            expected_pre,
        );
        assert_eq!(
            resolved_postconstruction_kinds(model, SolverConfig::default(), &plan),
            expected_post,
        );
    }
}

#[test]
fn omitted_local_search_requires_an_effective_solver_termination() {
    let bounded = SolverConfig::default().with_termination_seconds(1);

    for model in [static_model(Scenario::Cvrp), dynamic_model(Scenario::Cvrp)] {
        assert_eq!(
            default_local_search_eligibility(model.clone(), SolverConfig::default()),
            DefaultLocalSearchEligibility::IneligibleWithoutEffectiveTermination,
        );
        assert_eq!(
            default_local_search_eligibility(model, bounded.clone()),
            DefaultLocalSearchEligibility::Eligible,
        );
    }
}

#[test]
fn equivalent_typed_and_dynamic_cvrp_capabilities_freeze_the_same_default_local_search_order() {
    let typed = default_local_search_plan(static_model(Scenario::Cvrp));
    let dynamic = default_local_search_plan(dynamic_model_with_profile(
        Scenario::Cvrp,
        DynamicListProfile::Equivalent,
    ));
    let expected = vec![
        (
            DefaultLocalSearchSelectorFamily::NearbyListChange,
            DefaultSelectorCapabilityPolicy::DeclaredCapabilities,
        ),
        (
            DefaultLocalSearchSelectorFamily::NearbyListSwap,
            DefaultSelectorCapabilityPolicy::DeclaredCapabilities,
        ),
        (
            DefaultLocalSearchSelectorFamily::SublistChange,
            DefaultSelectorCapabilityPolicy::DeclaredCapabilities,
        ),
        (
            DefaultLocalSearchSelectorFamily::SublistSwap,
            DefaultSelectorCapabilityPolicy::DeclaredCapabilities,
        ),
        (
            DefaultLocalSearchSelectorFamily::ListReverse,
            DefaultSelectorCapabilityPolicy::DeclaredCapabilities,
        ),
        (
            DefaultLocalSearchSelectorFamily::NearbyKOpt,
            DefaultSelectorCapabilityPolicy::DeclaredCapabilities,
        ),
        (
            DefaultLocalSearchSelectorFamily::ListRuin,
            DefaultSelectorCapabilityPolicy::DeclaredCapabilities,
        ),
    ];

    assert_eq!(selector_signature(&typed), expected, "typed CVRP order");
    assert_eq!(selector_signature(&dynamic), expected, "dynamic CVRP order");
    assert_eq!(typed.components, dynamic.components);
    assert_eq!(
        typed.components.acceptor,
        DefaultLocalSearchAcceptorPolicy::LateAcceptance { history_size: 400 }
    );
    assert_eq!(
        typed.components.forager,
        DefaultLocalSearchForagerPolicy::AcceptedCount { limit: 256 }
    );
    let typed_excluded = typed
        .excluded
        .iter()
        .map(|excluded| excluded.family)
        .collect::<Vec<_>>();
    assert_eq!(typed_excluded, ["list_precedence_and_permute"]);
    assert_eq!(typed.excluded[0].reason, "missing=precedence");
    assert!(typed.candidate_trace_plan().is_complete());
    assert!(dynamic.candidate_trace_plan().is_complete());
}

#[test]
fn reduced_dynamic_list_capabilities_select_named_policy_rows() {
    let cvrp_without_position_metrics = default_local_search_plan(dynamic_model(Scenario::Cvrp));
    assert_eq!(
        selector_signature(&cvrp_without_position_metrics),
        vec![
            (
                DefaultLocalSearchSelectorFamily::PlainListChange,
                DefaultSelectorCapabilityPolicy::PlainListChangeWithoutCrossPositionDistance,
            ),
            (
                DefaultLocalSearchSelectorFamily::ListRuin,
                DefaultSelectorCapabilityPolicy::DeclaredCapabilities,
            ),
        ],
    );

    let set_without_metric = default_local_search_plan(dynamic_model_with_profile(
        Scenario::Cheapest,
        DynamicListProfile::SetWithoutMetric,
    ));
    assert_eq!(
        selector_signature(&set_without_metric),
        vec![
            (
                DefaultLocalSearchSelectorFamily::PlainListChange,
                DefaultSelectorCapabilityPolicy::PlainListChangeWithoutCrossPositionDistance,
            ),
            (
                DefaultLocalSearchSelectorFamily::PlainListSwap,
                DefaultSelectorCapabilityPolicy::PlainListSwapWithoutCrossPositionDistance,
            ),
            (
                DefaultLocalSearchSelectorFamily::ListRuin,
                DefaultSelectorCapabilityPolicy::DeclaredCapabilities,
            ),
        ],
    );

    let excluded = cvrp_without_position_metrics
        .excluded
        .iter()
        .map(|excluded| excluded.family)
        .collect::<Vec<_>>();
    assert_eq!(
        excluded,
        [
            "list_precedence_and_permute",
            "nearby_list_change",
            "list_swap",
            "sublist_change_and_swap",
            "list_reverse",
            "k_opt",
        ]
    );
    let nearby = &cvrp_without_position_metrics.excluded[1];
    assert_eq!(nearby.reason, "missing=cross_position_distance");

    let plan = cvrp_without_position_metrics.candidate_trace_plan();
    assert!(plan.attributes.iter().any(|attribute| {
        attribute.key == "excluded_families" && attribute.value.contains("nearby_list_change@")
    }));
    assert!(plan.children.iter().any(|child| {
        child.attributes.iter().any(|attribute| {
            attribute.key == "capability_policy"
                && attribute.value == "plain_list_change_without_cross_position_distance"
        })
    }));
}
