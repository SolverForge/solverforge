//! Public/static/dynamic parity for canonical Clarke-Wright construction.

use std::any::TypeId;
use std::sync::Arc;

use solverforge_core::domain::{
    DynamicListAccess, DynamicListAccessCapabilities, DynamicListMetadata,
    DynamicListMetadataCapabilities, DynamicListVariableSlot, EntityClassId, EntityDescriptor,
    SolutionDescriptor, VariableDescriptor, VariableId,
};
use solverforge_scoring::ScoreDirector;

use super::{
    assign_route, depot, distance, element_count, entity_count, feasible, get_assigned,
    index_to_element, route_len, Plan, Route,
};
use crate::builder::context::{
    bind_runtime_list_source, unassigned_from_current_assignment, RuntimeListSlot,
};
use crate::builder::{usize_element_source_key, ListVariableSlot};
use crate::heuristic::selector::nearby_list_change::DefaultCrossEntityDistanceMeter;
use crate::manager::ListClarkeWrightPhase;
use crate::phase::Phase;
use crate::runtime::compiler::executor::list_construction::execute_runtime_list_clarke_wright;
use crate::scope::SolverScope;
use crate::stats::{
    CandidateTraceExecutionPolicy, CandidateTraceHeader, CandidateTracePhasePlan,
    CandidateTraceTelemetry,
};

type Slot =
    RuntimeListSlot<Plan, usize, DefaultCrossEntityDistanceMeter, DefaultCrossEntityDistanceMeter>;

fn descriptor() -> SolutionDescriptor {
    SolutionDescriptor::new("ClarkeWrightParityPlan", TypeId::of::<Plan>()).with_entity(
        EntityDescriptor::new("Vehicle", TypeId::of::<Route>(), "vehicles")
            .with_logical_id(EntityClassId(0))
            .with_variable(VariableDescriptor::list("visits").with_logical_id(VariableId(0))),
    )
}

fn list_remove(plan: &mut Plan, entity: usize, position: usize) -> Option<usize> {
    (position < plan.routes[entity].visits.len())
        .then(|| plan.routes[entity].visits.remove(position))
}

fn construction_remove(plan: &mut Plan, entity: usize, position: usize) -> usize {
    plan.routes[entity].visits.remove(position)
}

fn list_insert(plan: &mut Plan, entity: usize, position: usize, value: usize) {
    plan.routes[entity].visits.insert(position, value);
}

fn list_get(plan: &Plan, entity: usize, position: usize) -> Option<usize> {
    plan.routes.get(entity)?.visits.get(position).copied()
}

fn list_set(plan: &mut Plan, entity: usize, position: usize, value: usize) {
    plan.routes[entity].visits[position] = value;
}

fn list_reverse(plan: &mut Plan, entity: usize, start: usize, end: usize) {
    plan.routes[entity].visits[start..end].reverse();
}

fn sublist_remove(plan: &mut Plan, entity: usize, start: usize, end: usize) -> Vec<usize> {
    plan.routes[entity].visits.drain(start..end).collect()
}

fn sublist_insert(plan: &mut Plan, entity: usize, position: usize, values: Vec<usize>) {
    plan.routes[entity]
        .visits
        .splice(position..position, values);
}

fn owner(_: &Plan, element: &usize) -> Option<usize> {
    Some(*element % 2)
}

fn metric_class(_: &Plan, entity: usize) -> usize {
    entity
}

fn static_slot() -> Slot {
    RuntimeListSlot::from_static(
        ListVariableSlot::new(
            "Vehicle",
            element_count,
            get_assigned,
            route_len,
            list_remove,
            construction_remove,
            list_insert,
            list_get,
            list_set,
            list_reverse,
            sublist_remove,
            sublist_insert,
            construction_remove,
            list_insert,
            index_to_element,
            usize_element_source_key,
            entity_count,
            DefaultCrossEntityDistanceMeter,
            DefaultCrossEntityDistanceMeter,
            "visits",
            0,
            None,
            Some(assign_route),
            None,
            None,
            None,
            Some(depot),
            Some(metric_class),
            Some(distance),
            Some(feasible),
        )
        .with_element_owner_fn(Some(owner)),
        0,
    )
}

#[derive(Debug)]
struct DynamicAccess;

impl DynamicListAccess<Plan> for DynamicAccess {
    fn entity_class(&self) -> EntityClassId {
        EntityClassId(0)
    }

    fn variable(&self) -> VariableId {
        VariableId(0)
    }

    fn entity_count(&self, plan: &Plan) -> usize {
        entity_count(plan)
    }

    fn element_count(&self, plan: &Plan) -> usize {
        element_count(plan)
    }

    fn element(&self, plan: &Plan, source_index: usize) -> Option<usize> {
        plan.customer_values.get(source_index).copied()
    }

    fn assigned_elements(&self, plan: &Plan) -> Vec<usize> {
        get_assigned(plan)
    }

    fn len(&self, plan: &Plan, entity: usize) -> usize {
        route_len(plan, entity)
    }

    fn get(&self, plan: &Plan, entity: usize, position: usize) -> Option<usize> {
        list_get(plan, entity, position)
    }

    fn insert(&self, plan: &mut Plan, entity: usize, position: usize, value: usize) {
        list_insert(plan, entity, position, value);
    }

    fn remove(&self, plan: &mut Plan, entity: usize, position: usize) -> Option<usize> {
        list_remove(plan, entity, position)
    }

    fn capabilities(&self) -> DynamicListAccessCapabilities {
        DynamicListAccessCapabilities {
            replace: true,
            ..DynamicListAccessCapabilities::default()
        }
    }

    fn replace(&self, plan: &mut Plan, entity: usize, values: Vec<usize>) -> bool {
        assign_route(plan, entity, values);
        true
    }
}

#[derive(Debug)]
struct DynamicMetadata;

impl DynamicListMetadata<Plan> for DynamicMetadata {
    fn entity_class(&self) -> EntityClassId {
        EntityClassId(0)
    }

    fn variable(&self) -> VariableId {
        VariableId(0)
    }

    fn capabilities(&self) -> DynamicListMetadataCapabilities {
        DynamicListMetadataCapabilities {
            element_owner: true,
            savings: true,
            ..DynamicListMetadataCapabilities::default()
        }
    }

    fn element_owner(&self, plan: &Plan, element: usize) -> Option<usize> {
        owner(plan, &element)
    }

    fn construction_order_key(&self, _: &Plan, _: usize) -> Option<i64> {
        None
    }

    fn precedence_duration(&self, _: &Plan, _: usize) -> Option<usize> {
        None
    }

    fn extend_precedence_successors(&self, _: &Plan, _: usize, _: &mut Vec<usize>) -> bool {
        false
    }

    fn cross_position_distance(
        &self,
        _: &Plan,
        _: usize,
        _: usize,
        _: usize,
        _: usize,
    ) -> Option<f64> {
        None
    }

    fn intra_position_distance(&self, _: &Plan, _: usize, _: usize, _: usize) -> Option<f64> {
        None
    }

    fn route_depot(&self, _: &Plan, _: usize) -> Option<usize> {
        None
    }

    fn route_distance(&self, _: &Plan, _: usize, _: usize, _: usize) -> Option<i64> {
        None
    }

    fn route_feasible(&self, _: &Plan, _: usize, _: &[usize]) -> Option<bool> {
        None
    }

    fn savings_depot(&self, plan: &Plan, entity: usize) -> Option<usize> {
        Some(depot(plan, entity))
    }

    fn savings_metric_class(&self, plan: &Plan, entity: usize) -> Option<usize> {
        Some(metric_class(plan, entity))
    }

    fn savings_distance(&self, plan: &Plan, entity: usize, from: usize, to: usize) -> Option<i64> {
        Some(distance(plan, entity, from, to))
    }

    fn savings_feasible(&self, plan: &Plan, entity: usize, route: &[usize]) -> Option<bool> {
        Some(feasible(plan, entity, route))
    }
}

fn dynamic_slot() -> Slot {
    let dynamic = DynamicListVariableSlot::with_access_and_metadata(
        EntityClassId(0),
        VariableId(0),
        "Vehicle",
        "visits",
        Arc::new(DynamicAccess),
        Arc::new(DynamicMetadata),
    )
    .expect("dynamic test bindings share one logical slot")
    .resolved_against(&descriptor())
    .expect("dynamic test slot resolves against its descriptor");
    RuntimeListSlot::from_dynamic(dynamic)
}

fn initial_plan() -> Plan {
    Plan {
        customer_values: vec![10, 11, 12],
        routes: vec![Route { visits: vec![10] }, Route { visits: Vec::new() }],
        score: None,
    }
}

fn trace_header() -> CandidateTraceHeader {
    CandidateTraceHeader::new(
        "public-compiled-clarke-wright-parity".to_string(),
        CandidateTraceExecutionPolicy::known("test", std::iter::empty::<(String, String)>()),
        CandidateTracePhasePlan::known("test", std::iter::empty::<(String, String)>(), Vec::new()),
        None,
    )
}

fn director(plan: Plan) -> ScoreDirector<Plan, ()> {
    ScoreDirector::simple(plan, descriptor(), |plan, descriptor_index| {
        if descriptor_index == 0 {
            plan.routes.len()
        } else {
            0
        }
    })
}

fn finish(
    scope: SolverScope<'_, Plan, ScoreDirector<Plan, ()>, ()>,
) -> (Vec<Vec<usize>>, CandidateTraceTelemetry) {
    let routes = scope
        .working_solution()
        .routes
        .iter()
        .map(|route| route.visits.clone())
        .collect();
    let trace = scope
        .stats()
        .snapshot()
        .candidate_trace
        .expect("trace remains available on the solver scope");
    (routes, trace)
}

fn run_public() -> (Vec<Vec<usize>>, CandidateTraceTelemetry) {
    let mut scope = SolverScope::new(director(initial_plan()));
    scope.enable_candidate_trace(trace_header(), 128);
    let mut phase = ListClarkeWrightPhase::new(
        element_count,
        get_assigned,
        entity_count,
        route_len,
        assign_route,
        index_to_element,
        usize_element_source_key,
        depot,
        distance,
        feasible,
        0,
    )
    .with_metric_class_fn(metric_class)
    .with_element_owner_fn(Some(owner));
    phase.solve(&mut scope);
    finish(scope)
}

fn run_runtime(slot: Slot) -> (Vec<Vec<usize>>, CandidateTraceTelemetry) {
    let plan = initial_plan();
    let binding = bind_runtime_list_source(&slot, &plan)
        .expect("well-formed source binds before phase execution");
    let source_index = binding.into_source_index();
    let unassigned = unassigned_from_current_assignment(&slot, &source_index, &plan)
        .expect("current assignment resolves against the bound source");
    let mut scope = SolverScope::new(director(plan));
    scope.enable_candidate_trace(trace_header(), 128);
    execute_runtime_list_clarke_wright(
        &slot,
        &source_index,
        &unassigned,
        crate::scope::StepControlPolicy::ObserveConfigLimits,
        &mut scope,
    )
    .expect("compiled Clarke-Wright executes");
    finish(scope)
}

#[test]
fn public_static_and_dynamic_clarke_wright_preserve_preassigned_owner_sensitive_routes_and_trace() {
    let (public_routes, public_trace) = run_public();
    let (static_routes, static_trace) = run_runtime(static_slot());
    let (dynamic_routes, dynamic_trace) = run_runtime(dynamic_slot());

    assert_eq!(public_routes, vec![vec![10], vec![11]]);
    assert_eq!(static_routes, public_routes);
    assert_eq!(dynamic_routes, public_routes);
    assert_eq!(static_trace, public_trace);
    assert_eq!(dynamic_trace, public_trace);
    assert!(public_trace.pulls.iter().any(|pull| matches!(
        pull.source,
        crate::stats::CandidateTraceSource::ListClarkeWrightSavings
    )));
    assert!(public_trace.pulls.iter().any(|pull| matches!(
        pull.source,
        crate::stats::CandidateTraceSource::ListClarkeWrightMerge
    )));
}
