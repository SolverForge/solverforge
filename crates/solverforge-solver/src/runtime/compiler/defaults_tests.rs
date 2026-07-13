//! Default-runtime expansion parity for native and dynamic list bindings.

use std::any::TypeId;
use std::sync::Arc;

use solverforge_config::{ConstructionHeuristicType, SolverConfig, TerminationConfig};
use solverforge_core::domain::{
    DynamicListAccess, DynamicListAccessCapabilities, DynamicListMetadata,
    DynamicListMetadataCapabilities, DynamicListVariableSlot, EntityClassId, EntityDescriptor,
    PlanningSolution, SolutionDescriptor, VariableDescriptor, VariableId,
};
use solverforge_core::score::SoftScore;

use super::{
    compile_runtime_graph, DefaultConstructionStage, DefaultConstructionStepKind,
    DefaultLocalSearchAcceptorPolicy, DefaultLocalSearchEligibility,
    DefaultLocalSearchForagerPolicy, DefaultLocalSearchPlan, DefaultLocalSearchSelectorFamily,
    DefaultPreconstructionStage, DefaultSelectorCapabilityPolicy, RuntimeGraphInput,
};
use crate::builder::{
    ListVariableSlot, NoDynamicExtensions, RuntimeModel, SearchContext, VariableSlot,
};
use crate::heuristic::selector::nearby_list_change::DefaultCrossEntityDistanceMeter;

type Meter = DefaultCrossEntityDistanceMeter;
type Model = RuntimeModel<Plan, usize, Meter, Meter>;

#[derive(Clone, Debug)]
struct Plan {
    score: Option<SoftScore>,
    elements: Vec<usize>,
    routes: Vec<Vec<usize>>,
}

impl PlanningSolution for Plan {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn descriptor() -> SolutionDescriptor {
    SolutionDescriptor::new("Plan", TypeId::of::<Plan>()).with_entity(
        EntityDescriptor::new("Vehicle", TypeId::of::<Vec<usize>>(), "vehicles")
            .with_logical_id(EntityClassId(0))
            .with_variable(VariableDescriptor::list("visits").with_logical_id(VariableId(0))),
    )
}

fn element_count(plan: &Plan) -> usize {
    plan.elements.len()
}

fn assigned_elements(plan: &Plan) -> Vec<usize> {
    plan.routes.iter().flatten().copied().collect()
}

fn entity_count(plan: &Plan) -> usize {
    plan.routes.len()
}

fn list_len(plan: &Plan, entity: usize) -> usize {
    plan.routes[entity].len()
}

fn list_remove(plan: &mut Plan, entity: usize, position: usize) -> Option<usize> {
    (position < plan.routes[entity].len()).then(|| plan.routes[entity].remove(position))
}

fn construction_list_remove(plan: &mut Plan, entity: usize, position: usize) -> usize {
    plan.routes[entity].remove(position)
}

fn list_insert(plan: &mut Plan, entity: usize, position: usize, value: usize) {
    plan.routes[entity].insert(position, value);
}

fn list_get(plan: &Plan, entity: usize, position: usize) -> Option<usize> {
    plan.routes.get(entity)?.get(position).copied()
}

fn list_set(plan: &mut Plan, entity: usize, position: usize, value: usize) {
    plan.routes[entity][position] = value;
}

fn list_reverse(plan: &mut Plan, entity: usize, start: usize, end: usize) {
    plan.routes[entity][start..end].reverse();
}

fn sublist_remove(plan: &mut Plan, entity: usize, start: usize, end: usize) -> Vec<usize> {
    plan.routes[entity].drain(start..end).collect()
}

fn sublist_insert(plan: &mut Plan, entity: usize, position: usize, values: Vec<usize>) {
    plan.routes[entity].splice(position..position, values);
}

fn ruin_remove(plan: &mut Plan, entity: usize, position: usize) -> usize {
    plan.routes[entity].remove(position)
}

fn ruin_insert(plan: &mut Plan, entity: usize, position: usize, value: usize) {
    plan.routes[entity].insert(position, value);
}

fn index_to_element(plan: &Plan, element_index: usize) -> usize {
    plan.elements[element_index]
}

fn route_values(plan: &Plan, entity: usize) -> Vec<usize> {
    plan.routes[entity].clone()
}

fn replace_route(plan: &mut Plan, entity: usize, values: Vec<usize>) {
    plan.routes[entity] = values;
}

fn depot(_: &Plan, _: usize) -> usize {
    0
}

fn distance(_: &Plan, from: usize, to: usize) -> i64 {
    from.abs_diff(to) as i64
}

fn route_distance(plan: &Plan, _: usize, from: usize, to: usize) -> i64 {
    distance(plan, from, to)
}

fn savings_distance(plan: &Plan, _: usize, from: usize, to: usize) -> i64 {
    distance(plan, from, to)
}

fn feasible(_: &Plan, _: usize, _: &[usize]) -> bool {
    true
}

fn owner(_: &Plan, _: &usize) -> Option<usize> {
    None
}

fn order(_: &Plan, element: usize) -> i64 {
    element as i64
}

fn duration(_: &Plan, _: usize) -> usize {
    1
}

fn successors(_: &Plan, _: usize, _: &mut Vec<usize>) {}

#[derive(Clone, Copy, Debug)]
enum Scenario {
    Cheapest,
    Jssp,
    Cvrp,
}

#[derive(Clone, Copy, Debug)]
enum DynamicListProfile {
    Reduced,
    SetWithoutMetric,
    Equivalent,
}

fn static_model(scenario: Scenario) -> Model {
    let route_hooks = matches!(scenario, Scenario::Cvrp).then_some((
        Some(route_values as fn(&Plan, usize) -> Vec<usize>),
        Some(replace_route as fn(&mut Plan, usize, Vec<usize>)),
        Some(depot as fn(&Plan, usize) -> usize),
        Some(route_distance as fn(&Plan, usize, usize, usize) -> i64),
        None,
        Some(depot as fn(&Plan, usize) -> usize),
        None,
        Some(savings_distance as fn(&Plan, usize, usize, usize) -> i64),
        Some(feasible as fn(&Plan, usize, &[usize]) -> bool),
    ));
    let (
        route_get,
        route_set,
        route_depot,
        route_distance,
        route_feasible,
        savings_depot,
        savings_metric,
        savings_distance,
        savings_feasible,
    ) = route_hooks.unwrap_or((None, None, None, None, None, None, None, None, None));
    let slot = ListVariableSlot::new(
        "Vehicle",
        element_count,
        assigned_elements,
        list_len,
        list_remove,
        construction_list_remove,
        list_insert,
        list_get,
        list_set,
        list_reverse,
        sublist_remove,
        sublist_insert,
        ruin_remove,
        ruin_insert,
        index_to_element,
        crate::builder::usize_element_source_key,
        entity_count,
        DefaultCrossEntityDistanceMeter,
        DefaultCrossEntityDistanceMeter,
        "visits",
        0,
        route_get,
        route_set,
        route_depot,
        route_distance,
        route_feasible,
        savings_depot,
        savings_metric,
        savings_distance,
        savings_feasible,
    );
    let slot = match scenario {
        Scenario::Jssp => slot
            .with_element_owner_fn(Some(owner))
            .with_construction_element_order_key(Some(order))
            .with_precedence_hooks(Some(duration), Some(successors)),
        Scenario::Cheapest | Scenario::Cvrp => slot,
    };
    RuntimeModel::new(vec![VariableSlot::List(slot)])
}

#[derive(Debug)]
struct DynamicAccess {
    profile: DynamicListProfile,
}

impl DynamicListAccess<Plan> for DynamicAccess {
    fn entity_class(&self) -> EntityClassId {
        EntityClassId(0)
    }

    fn variable(&self) -> VariableId {
        VariableId(0)
    }

    fn entity_count(&self, solution: &Plan) -> usize {
        entity_count(solution)
    }

    fn element_count(&self, solution: &Plan) -> usize {
        element_count(solution)
    }

    fn element(&self, solution: &Plan, element_index: usize) -> Option<usize> {
        solution.elements.get(element_index).copied()
    }

    fn assigned_elements(&self, solution: &Plan) -> Vec<usize> {
        assigned_elements(solution)
    }

    fn len(&self, solution: &Plan, entity: usize) -> usize {
        list_len(solution, entity)
    }

    fn get(&self, solution: &Plan, entity: usize, position: usize) -> Option<usize> {
        list_get(solution, entity, position)
    }

    fn insert(&self, solution: &mut Plan, entity: usize, position: usize, value: usize) {
        list_insert(solution, entity, position, value);
    }

    fn remove(&self, solution: &mut Plan, entity: usize, position: usize) -> Option<usize> {
        list_remove(solution, entity, position)
    }

    fn capabilities(&self) -> DynamicListAccessCapabilities {
        match self.profile {
            DynamicListProfile::Reduced => DynamicListAccessCapabilities {
                replace: true,
                ..DynamicListAccessCapabilities::default()
            },
            DynamicListProfile::SetWithoutMetric => DynamicListAccessCapabilities {
                set: true,
                ..DynamicListAccessCapabilities::default()
            },
            DynamicListProfile::Equivalent => DynamicListAccessCapabilities {
                set: true,
                replace: true,
                reverse: true,
                sublist: true,
            },
        }
    }

    fn set(&self, solution: &mut Plan, entity: usize, position: usize, value: usize) -> bool {
        list_set(solution, entity, position, value);
        true
    }

    fn replace(&self, solution: &mut Plan, entity: usize, values: Vec<usize>) -> bool {
        replace_route(solution, entity, values);
        true
    }

    fn reverse(&self, solution: &mut Plan, entity: usize, start: usize, end: usize) -> bool {
        list_reverse(solution, entity, start, end);
        true
    }

    fn sublist_remove(
        &self,
        solution: &mut Plan,
        entity: usize,
        start: usize,
        end: usize,
    ) -> Option<Vec<usize>> {
        Some(sublist_remove(solution, entity, start, end))
    }

    fn sublist_insert(
        &self,
        solution: &mut Plan,
        entity: usize,
        position: usize,
        values: Vec<usize>,
    ) -> bool {
        sublist_insert(solution, entity, position, values);
        true
    }
}

#[derive(Debug)]
struct DynamicMetadata {
    scenario: Scenario,
    profile: DynamicListProfile,
}

impl DynamicListMetadata<Plan> for DynamicMetadata {
    fn entity_class(&self) -> EntityClassId {
        EntityClassId(0)
    }

    fn variable(&self) -> VariableId {
        VariableId(0)
    }

    fn capabilities(&self) -> DynamicListMetadataCapabilities {
        match self.scenario {
            Scenario::Cheapest => DynamicListMetadataCapabilities::default(),
            Scenario::Jssp => DynamicListMetadataCapabilities {
                element_owner: true,
                construction_order_key: true,
                precedence_duration: true,
                precedence_successors: true,
                ..DynamicListMetadataCapabilities::default()
            },
            Scenario::Cvrp => DynamicListMetadataCapabilities {
                cross_position_distance: matches!(self.profile, DynamicListProfile::Equivalent),
                intra_position_distance: matches!(self.profile, DynamicListProfile::Equivalent),
                route: true,
                savings: true,
                ..DynamicListMetadataCapabilities::default()
            },
        }
    }

    fn element_owner(&self, solution: &Plan, element: usize) -> Option<usize> {
        owner(solution, &element)
    }

    fn construction_order_key(&self, solution: &Plan, element: usize) -> Option<i64> {
        matches!(self.scenario, Scenario::Jssp).then(|| order(solution, element))
    }

    fn precedence_duration(&self, solution: &Plan, element: usize) -> Option<usize> {
        matches!(self.scenario, Scenario::Jssp).then(|| duration(solution, element))
    }

    fn extend_precedence_successors(
        &self,
        solution: &Plan,
        element: usize,
        successors_out: &mut Vec<usize>,
    ) -> bool {
        if !matches!(self.scenario, Scenario::Jssp) {
            return false;
        }
        successors(solution, element, successors_out);
        true
    }

    fn cross_position_distance(
        &self,
        _: &Plan,
        _: usize,
        _: usize,
        _: usize,
        _: usize,
    ) -> Option<f64> {
        matches!(self.profile, DynamicListProfile::Equivalent).then_some(0.0)
    }

    fn intra_position_distance(&self, _: &Plan, _: usize, _: usize, _: usize) -> Option<f64> {
        matches!(self.profile, DynamicListProfile::Equivalent).then_some(0.0)
    }

    fn route_depot(&self, solution: &Plan, entity: usize) -> Option<usize> {
        matches!(self.scenario, Scenario::Cvrp).then(|| depot(solution, entity))
    }

    fn route_distance(
        &self,
        solution: &Plan,
        entity: usize,
        from: usize,
        to: usize,
    ) -> Option<i64> {
        matches!(self.scenario, Scenario::Cvrp).then(|| route_distance(solution, entity, from, to))
    }

    fn route_feasible(&self, solution: &Plan, entity: usize, route: &[usize]) -> Option<bool> {
        matches!(self.scenario, Scenario::Cvrp).then(|| feasible(solution, entity, route))
    }

    fn savings_depot(&self, solution: &Plan, entity: usize) -> Option<usize> {
        matches!(self.scenario, Scenario::Cvrp).then(|| depot(solution, entity))
    }

    fn savings_metric_class(&self, _: &Plan, entity: usize) -> Option<usize> {
        matches!(self.scenario, Scenario::Cvrp).then_some(entity)
    }

    fn savings_distance(
        &self,
        solution: &Plan,
        entity: usize,
        from: usize,
        to: usize,
    ) -> Option<i64> {
        matches!(self.scenario, Scenario::Cvrp)
            .then(|| savings_distance(solution, entity, from, to))
    }

    fn savings_feasible(&self, solution: &Plan, entity: usize, route: &[usize]) -> Option<bool> {
        matches!(self.scenario, Scenario::Cvrp).then(|| feasible(solution, entity, route))
    }
}

fn dynamic_model(scenario: Scenario) -> Model {
    dynamic_model_with_profile(scenario, DynamicListProfile::Reduced)
}

fn dynamic_model_with_profile(scenario: Scenario, profile: DynamicListProfile) -> Model {
    let slot = DynamicListVariableSlot::with_access_and_metadata(
        EntityClassId(0),
        VariableId(0),
        "Vehicle",
        "visits",
        Arc::new(DynamicAccess { profile }),
        Arc::new(DynamicMetadata { scenario, profile }),
    )
    .expect("test access and metadata identities match");
    RuntimeModel::new(vec![VariableSlot::DynamicList(slot)])
}

fn initial_plan() -> Plan {
    Plan {
        score: None,
        elements: vec![0, 1, 2],
        routes: vec![Vec::new(), Vec::new()],
    }
}

fn resolved_preconstruction_kinds(
    model: Model,
    config: SolverConfig,
    stage: DefaultPreconstructionStage,
    plan: &Plan,
) -> (DefaultConstructionStage, Vec<DefaultConstructionStepKind>) {
    let context = SearchContext::new(descriptor(), model, config.random_seed);
    let graph = compile_runtime_graph(
        &config,
        RuntimeGraphInput::new(context, NoDynamicExtensions),
    )
    .expect("default graph compiles for its declared capabilities");
    let resolved = super::defaults::resolve_default_preconstruction_stage(
        graph.default_bindings(),
        stage,
        plan,
    );
    (
        resolved.stage,
        resolved.steps.into_iter().map(|step| step.kind).collect(),
    )
}

fn resolved_postconstruction_kinds(
    model: Model,
    config: SolverConfig,
    plan: &Plan,
) -> (DefaultConstructionStage, Vec<DefaultConstructionStepKind>) {
    let context = SearchContext::new(descriptor(), model, config.random_seed);
    let graph = compile_runtime_graph(
        &config,
        RuntimeGraphInput::new(context, NoDynamicExtensions),
    )
    .expect("default graph compiles for its declared capabilities");
    let resolved =
        super::defaults::resolve_default_postconstruction_kopt(graph.default_bindings(), plan);
    (
        resolved.stage,
        resolved.steps.into_iter().map(|step| step.kind).collect(),
    )
}

fn default_local_search_eligibility(
    model: Model,
    config: SolverConfig,
) -> DefaultLocalSearchEligibility {
    let context = SearchContext::new(descriptor(), model, config.random_seed);
    let graph = compile_runtime_graph(
        &config,
        RuntimeGraphInput::new(context, NoDynamicExtensions),
    )
    .expect("default graph compiles for its declared capabilities");
    graph
        .default_bindings()
        .local_search_policy
        .eligibility::<Plan>(&config)
}

fn default_local_search_plan(model: Model) -> DefaultLocalSearchPlan {
    let config = SolverConfig::default();
    let context = SearchContext::new(descriptor(), model, config.random_seed);
    compile_runtime_graph(
        &config,
        RuntimeGraphInput::new(context, NoDynamicExtensions),
    )
    .expect("default graph compiles for its declared capabilities")
    .default_bindings()
    .local_search_plan
    .clone()
    .expect("empty phase configuration must compile one omitted-local-search plan")
}

fn selector_signature(
    plan: &DefaultLocalSearchPlan,
) -> Vec<(
    DefaultLocalSearchSelectorFamily,
    DefaultSelectorCapabilityPolicy,
)> {
    plan.selectors
        .iter()
        .map(|selector| (selector.family, selector.capability_policy))
        .collect()
}

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
    let invalid_score_only = SolverConfig {
        termination: Some(TerminationConfig {
            best_score_limit: Some("not-a-soft-score".to_string()),
            ..TerminationConfig::default()
        }),
        ..SolverConfig::default()
    };

    for model in [static_model(Scenario::Cvrp), dynamic_model(Scenario::Cvrp)] {
        assert_eq!(
            default_local_search_eligibility(model.clone(), SolverConfig::default()),
            DefaultLocalSearchEligibility::IneligibleWithoutEffectiveTermination,
        );
        assert_eq!(
            default_local_search_eligibility(model.clone(), invalid_score_only.clone()),
            DefaultLocalSearchEligibility::IneligibleWithoutEffectiveTermination,
            "a present but unparsable score limit is not an effective solver boundary",
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

    let plan = cvrp_without_position_metrics.candidate_trace_plan();
    assert!(plan.children.iter().any(|child| {
        child.attributes.iter().any(|attribute| {
            attribute.key == "capability_policy"
                && attribute.value == "plain_list_change_without_cross_position_distance"
        })
    }));
}
