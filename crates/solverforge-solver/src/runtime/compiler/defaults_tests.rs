//! Default-runtime expansion parity for native and dynamic list bindings.

use crate::builder::search::custom::NoDynamicExtensions;

use std::any::TypeId;
use std::sync::Arc;

use solverforge_config::{ConstructionHeuristicType, SolverConfig};
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
use crate::builder::{ListVariableSlot, RuntimeModel, SearchContext, VariableSlot};
use crate::heuristic::selector::nearby_list_change::DefaultCrossEntityDistanceMeter;

mod dynamic;
use dynamic::{dynamic_model, dynamic_model_with_profile};

mod selection;

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
        list_insert,
        list_get,
        list_set,
        list_reverse,
        sublist_remove,
        sublist_insert,
        ruin_remove,
        ruin_insert,
        index_to_element,
        crate::builder::context::list::usize_element_source_key,
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
    let context = SearchContext::new(descriptor(), model, config.random_seed)
        .expect("search context should bind");
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
    let context = SearchContext::new(descriptor(), model, config.random_seed)
        .expect("search context should bind");
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
    let context = SearchContext::new(descriptor(), model, config.random_seed)
        .expect("search context should bind");
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
    let context = SearchContext::new(descriptor(), model, config.random_seed)
        .expect("search context should bind");
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
