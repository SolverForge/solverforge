//! Public/static/dynamic trace parity for canonical regret insertion.

use std::any::TypeId;
use std::sync::Arc;

use solverforge_core::domain::{
    DynamicListAccess, DynamicListAccessCapabilities, DynamicListVariableSlot, EntityClassId,
    EntityDescriptor, SolutionDescriptor, VariableDescriptor, VariableId,
};

use super::*;
use crate::builder::context::{
    bind_runtime_list_source, unassigned_from_current_assignment, RuntimeListSlot,
};
use crate::builder::{usize_element_source_key, ListVariableSlot};
use crate::heuristic::selector::nearby_list_change::DefaultCrossEntityDistanceMeter;
use crate::phase::Phase;
use crate::runtime::compiler::executor::list_construction::execute_runtime_list_regret_insertion;
use crate::stats::{
    CandidateTraceExecutionPolicy, CandidateTraceHeader, CandidateTracePhasePlan,
    CandidateTraceTelemetry,
};

type Slot =
    RuntimeListSlot<Plan, usize, DefaultCrossEntityDistanceMeter, DefaultCrossEntityDistanceMeter>;

fn runtime_descriptor() -> SolutionDescriptor {
    SolutionDescriptor::new("Plan", TypeId::of::<Plan>()).with_entity(
        EntityDescriptor::new("Vehicle", TypeId::of::<Vec<usize>>(), "vehicles")
            .with_logical_id(EntityClassId(0))
            .with_variable(VariableDescriptor::list("visits").with_logical_id(VariableId(0))),
    )
}

fn list_remove_option(plan: &mut Plan, entity: usize, position: usize) -> Option<usize> {
    (position < plan.routes[entity].len()).then(|| plan.routes[entity].remove(position))
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

fn static_slot() -> Slot {
    RuntimeListSlot::from_static(
        ListVariableSlot::new(
            "Vehicle",
            element_count,
            assigned,
            list_len,
            list_remove_option,
            list_remove,
            list_insert,
            list_get,
            list_set,
            list_reverse,
            sublist_remove,
            sublist_insert,
            list_remove,
            list_insert,
            index_to_element,
            usize_element_source_key,
            entity_count,
            DefaultCrossEntityDistanceMeter,
            DefaultCrossEntityDistanceMeter,
            "visits",
            0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ),
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
        plan.elements.get(source_index).copied()
    }

    fn assigned_elements(&self, plan: &Plan) -> Vec<usize> {
        assigned(plan)
    }

    fn len(&self, plan: &Plan, entity: usize) -> usize {
        list_len(plan, entity)
    }

    fn get(&self, plan: &Plan, entity: usize, position: usize) -> Option<usize> {
        list_get(plan, entity, position)
    }

    fn insert(&self, plan: &mut Plan, entity: usize, position: usize, value: usize) {
        list_insert(plan, entity, position, value);
    }

    fn remove(&self, plan: &mut Plan, entity: usize, position: usize) -> Option<usize> {
        list_remove_option(plan, entity, position)
    }

    fn capabilities(&self) -> DynamicListAccessCapabilities {
        DynamicListAccessCapabilities::default()
    }
}

fn dynamic_slot() -> Slot {
    let dynamic = DynamicListVariableSlot::try_with_access(
        EntityClassId(0),
        VariableId(0),
        "Vehicle",
        "visits",
        Arc::new(DynamicAccess),
    )
    .expect("dynamic test access identity matches its slot")
    .resolved_against(&runtime_descriptor())
    .expect("dynamic test slot resolves against its descriptor");
    RuntimeListSlot::from_dynamic(dynamic)
}

fn initial_plan() -> Plan {
    Plan {
        elements: vec![1, 2, 3],
        routes: vec![Vec::new(), Vec::new()],
        score: None,
    }
}

fn trace_header() -> CandidateTraceHeader {
    CandidateTraceHeader::new(
        "compiled-regret-parity".to_string(),
        CandidateTraceExecutionPolicy::known("test", std::iter::empty::<(String, String)>()),
        CandidateTracePhasePlan::known("test", std::iter::empty::<(String, String)>(), Vec::new()),
        None,
    )
}

fn finish(
    scope: SolverScope<'_, Plan, TestDirector, ()>,
) -> (Vec<Vec<usize>>, CandidateTraceTelemetry) {
    let routes = scope.working_solution().routes.clone();
    let trace = scope
        .stats()
        .snapshot()
        .candidate_trace
        .expect("trace remains available on the solver scope");
    (routes, trace)
}

fn run_public() -> (Vec<Vec<usize>>, CandidateTraceTelemetry) {
    let mut scope = SolverScope::new(director(initial_plan(), zero_score));
    scope.enable_candidate_trace(trace_header(), 128);
    let mut phase = phase();
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
    let mut scope = SolverScope::new(director(plan, zero_score));
    scope.enable_candidate_trace(trace_header(), 128);
    execute_runtime_list_regret_insertion(
        &slot,
        &source_index,
        &unassigned,
        crate::scope::StepControlPolicy::ObserveConfigLimits,
        &mut scope,
    )
    .expect("compiled regret insertion executes");
    finish(scope)
}

#[test]
fn public_static_and_dynamic_regret_share_routes_and_candidate_trace() {
    let (public_routes, public_trace) = run_public();
    let (static_routes, static_trace) = run_runtime(static_slot());
    let (dynamic_routes, dynamic_trace) = run_runtime(dynamic_slot());

    assert_eq!(static_routes, public_routes);
    assert_eq!(dynamic_routes, public_routes);
    assert_eq!(static_trace, public_trace);
    assert_eq!(dynamic_trace, public_trace);
    assert!(dynamic_trace.pulls.iter().any(|pull| matches!(
        pull.source,
        crate::stats::CandidateTraceSource::ListRegretInsertionTrial
    )));
}

#[test]
fn compiled_regret_refreshes_current_assignment_before_a_second_execution() {
    let slot = static_slot();
    let plan = initial_plan();
    let binding = bind_runtime_list_source(&slot, &plan)
        .expect("well-formed source binds before phase construction");
    let source_index = binding.into_source_index();
    let unassigned = unassigned_from_current_assignment(&slot, &source_index, &plan)
        .expect("initial assignment resolves against the bound source");
    let mut scope = SolverScope::new(director(plan, zero_score));
    execute_runtime_list_regret_insertion(
        &slot,
        &source_index,
        &unassigned,
        crate::scope::StepControlPolicy::ObserveConfigLimits,
        &mut scope,
    )
    .expect("compiled regret insertion executes");
    let first_routes = scope.working_solution().routes.clone();
    let unassigned =
        unassigned_from_current_assignment(&slot, &source_index, scope.working_solution())
            .expect("updated assignment resolves against the bound source");
    execute_runtime_list_regret_insertion(
        &slot,
        &source_index,
        &unassigned,
        crate::scope::StepControlPolicy::ObserveConfigLimits,
        &mut scope,
    )
    .expect("compiled regret insertion refreshes current assignment");

    assert_eq!(scope.working_solution().routes, first_routes);
}
