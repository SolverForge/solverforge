//! Static/dynamic parity for the one compiled Clarke-Wright kernel.

use std::any::TypeId;
use std::sync::Arc;

use solverforge_core::domain::{
    DynamicListAccess, DynamicListAccessCapabilities, DynamicListMetadata,
    DynamicListMetadataCapabilities, DynamicListVariableSlot, EntityClassId, EntityDescriptor,
    PlanningSolution, SolutionDescriptor, VariableDescriptor, VariableId,
};
use solverforge_core::score::SoftScore;
use solverforge_scoring::ScoreDirector;

use crate::builder::context::{
    bind_runtime_list_source, unassigned_from_current_assignment, RuntimeListSlot,
};
use crate::builder::{usize_element_source_key, ListVariableSlot};
use crate::heuristic::selector::nearby_list_change::DefaultCrossEntityDistanceMeter;
use crate::manager::ListConstructionPhaseBuilder;
use crate::phase::Phase;
use crate::runtime::compiler::executor::list_construction::{
    execute_runtime_list_clarke_wright, execute_runtime_list_round_robin,
};
use crate::scope::SolverScope;
use crate::stats::{
    CandidateTraceExecutionPolicy, CandidateTraceHeader, CandidateTracePhasePlan,
    CandidateTraceTelemetry,
};

type Slot =
    RuntimeListSlot<Plan, usize, DefaultCrossEntityDistanceMeter, DefaultCrossEntityDistanceMeter>;

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

fn construction_remove(plan: &mut Plan, entity: usize, position: usize) -> usize {
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

fn index_to_element(plan: &Plan, source_index: usize) -> usize {
    plan.elements[source_index]
}

fn replace_route(plan: &mut Plan, entity: usize, values: Vec<usize>) {
    plan.routes[entity] = values;
}

fn depot(_: &Plan, _: usize) -> usize {
    0
}

fn metric_class(_: &Plan, entity: usize) -> usize {
    entity
}

fn distance(_: &Plan, _: usize, from: usize, to: usize) -> i64 {
    from.abs_diff(to) as i64
}

fn feasible(_: &Plan, _: usize, route: &[usize]) -> bool {
    route.len() <= 2
}

fn static_slot() -> Slot {
    RuntimeListSlot::from_static(
        ListVariableSlot::new(
            "Vehicle",
            element_count,
            assigned_elements,
            list_len,
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
            Some(replace_route),
            None,
            None,
            None,
            Some(depot),
            Some(metric_class),
            Some(distance),
            Some(feasible),
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
        assigned_elements(plan)
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
        list_remove(plan, entity, position)
    }

    fn capabilities(&self) -> DynamicListAccessCapabilities {
        DynamicListAccessCapabilities {
            replace: true,
            ..DynamicListAccessCapabilities::default()
        }
    }

    fn replace(&self, plan: &mut Plan, entity: usize, values: Vec<usize>) -> bool {
        replace_route(plan, entity, values);
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
            savings: true,
            ..DynamicListMetadataCapabilities::default()
        }
    }

    fn element_owner(&self, _: &Plan, _: usize) -> Option<usize> {
        None
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

fn trace_header() -> CandidateTraceHeader {
    CandidateTraceHeader::new(
        "compiled-cw-parity".to_string(),
        CandidateTraceExecutionPolicy::known("test", std::iter::empty::<(String, String)>()),
        CandidateTracePhasePlan::known("test", std::iter::empty::<(String, String)>(), Vec::new()),
        None,
    )
}

fn round_robin_trace_header() -> CandidateTraceHeader {
    CandidateTraceHeader::new(
        "compiled-round-robin-parity".to_string(),
        CandidateTraceExecutionPolicy::known("test", std::iter::empty::<(String, String)>()),
        CandidateTracePhasePlan::known("test", std::iter::empty::<(String, String)>(), Vec::new()),
        None,
    )
}

fn run(slot: Slot) -> (Vec<Vec<usize>>, CandidateTraceTelemetry) {
    let plan = Plan {
        score: None,
        elements: vec![1, 2, 3, 4],
        routes: vec![Vec::new(), Vec::new()],
    };
    let binding = bind_runtime_list_source(&slot, &plan)
        .expect("well-formed parity source binds before phase execution");
    let source_index = binding.into_source_index();
    let unassigned = unassigned_from_current_assignment(&slot, &source_index, &plan)
        .expect("current assignment resolves against the bound source");
    let director = ScoreDirector::simple(plan, descriptor(), |plan, descriptor_index| {
        if descriptor_index == 0 {
            plan.routes.len()
        } else {
            0
        }
    });
    let mut scope = SolverScope::new(director);
    scope.enable_candidate_trace(trace_header(), 128);
    execute_runtime_list_clarke_wright(
        &slot,
        &source_index,
        &unassigned,
        crate::scope::StepControlPolicy::ObserveConfigLimits,
        &mut scope,
    )
    .expect("compiled Clarke-Wright executes");
    let routes = scope.working_solution().routes.clone();
    let trace = scope
        .stats()
        .snapshot()
        .candidate_trace
        .expect("trace remains available on the solver scope");
    (routes, trace)
}

fn append_at_end(plan: &mut Plan, entity: usize, value: usize) {
    let position = list_len(plan, entity);
    list_insert(plan, entity, position, value);
}

fn round_robin_plan() -> Plan {
    Plan {
        score: None,
        elements: vec![1, 2, 3, 4],
        routes: vec![Vec::new(), Vec::new()],
    }
}

fn round_robin_director(plan: Plan) -> ScoreDirector<Plan, ()> {
    ScoreDirector::simple(plan, descriptor(), |plan, descriptor_index| {
        if descriptor_index == 0 {
            plan.routes.len()
        } else {
            0
        }
    })
}

fn round_robin_trace(
    scope: &SolverScope<'_, Plan, ScoreDirector<Plan, ()>, ()>,
) -> CandidateTraceTelemetry {
    scope
        .stats()
        .snapshot()
        .candidate_trace
        .expect("trace remains available on the solver scope")
}

fn run_runtime_round_robin(slot: Slot) -> (Vec<Vec<usize>>, CandidateTraceTelemetry) {
    let plan = round_robin_plan();
    let binding = bind_runtime_list_source(&slot, &plan)
        .expect("well-formed parity source binds before phase execution");
    let source_index = binding.into_source_index();
    let unassigned = unassigned_from_current_assignment(&slot, &source_index, &plan)
        .expect("current assignment resolves against the bound source");
    let mut scope = SolverScope::new(round_robin_director(plan));
    scope.enable_candidate_trace(round_robin_trace_header(), 128);
    execute_runtime_list_round_robin(
        &slot,
        &source_index,
        &unassigned,
        crate::scope::StepControlPolicy::ObserveConfigLimits,
        &mut scope,
    )
    .expect("compiled round-robin executes");
    let routes = scope.working_solution().routes.clone();
    let trace = round_robin_trace(&scope);
    (routes, trace)
}

fn run_public_round_robin() -> (Vec<Vec<usize>>, CandidateTraceTelemetry) {
    let mut scope = SolverScope::new(round_robin_director(round_robin_plan()));
    scope.enable_candidate_trace(round_robin_trace_header(), 128);
    let builder = ListConstructionPhaseBuilder::<Plan, usize>::new(
        element_count,
        assigned_elements,
        entity_count,
        append_at_end,
        index_to_element,
        usize_element_source_key,
        0,
    );
    let mut phase = builder.create_phase();
    phase.solve(&mut scope);
    let routes = scope.working_solution().routes.clone();
    let trace = round_robin_trace(&scope);
    (routes, trace)
}

#[test]
fn runtime_static_and_dynamic_clarke_wright_have_identical_routes_and_trace() {
    let (static_routes, static_trace) = run(static_slot());
    let (dynamic_routes, dynamic_trace) = run(dynamic_slot());

    assert_eq!(dynamic_routes, static_routes);
    assert_eq!(dynamic_trace, static_trace);
    assert!(dynamic_trace.pulls.iter().any(|pull| matches!(
        pull.source,
        crate::stats::CandidateTraceSource::ListClarkeWrightSavings
    )));
    assert!(dynamic_trace.pulls.iter().any(|pull| matches!(
        pull.source,
        crate::stats::CandidateTraceSource::ListClarkeWrightMerge
    )));
}

#[test]
fn public_static_and_dynamic_round_robin_have_identical_routes_and_trace() {
    let (public_routes, public_trace) = run_public_round_robin();
    let (static_routes, static_trace) = run_runtime_round_robin(static_slot());
    let (dynamic_routes, dynamic_trace) = run_runtime_round_robin(dynamic_slot());

    assert_eq!(public_routes, vec![vec![1, 3], vec![2, 4]]);
    assert_eq!(static_routes, public_routes);
    assert_eq!(dynamic_routes, public_routes);
    assert_eq!(static_trace, public_trace);
    assert_eq!(dynamic_trace, public_trace);
    assert!(dynamic_trace.pulls.iter().any(|pull| matches!(
        pull.source,
        crate::stats::CandidateTraceSource::ListRoundRobinConstruction
    )));
}

#[test]
fn compiled_clarke_wright_refreshes_current_assignment_before_a_second_execution() {
    let slot = static_slot();
    let plan = Plan {
        score: None,
        elements: vec![1, 2, 3, 4],
        routes: vec![Vec::new(), Vec::new()],
    };
    let binding = bind_runtime_list_source(&slot, &plan)
        .expect("well-formed source binds before phase construction");
    let source_index = binding.into_source_index();
    let unassigned = unassigned_from_current_assignment(&slot, &source_index, &plan)
        .expect("initial assignment resolves against the bound source");
    let mut scope = SolverScope::new(round_robin_director(plan));
    execute_runtime_list_clarke_wright(
        &slot,
        &source_index,
        &unassigned,
        crate::scope::StepControlPolicy::ObserveConfigLimits,
        &mut scope,
    )
    .expect("compiled Clarke-Wright executes");
    let first_routes = scope.working_solution().routes.clone();
    let unassigned =
        unassigned_from_current_assignment(&slot, &source_index, scope.working_solution())
            .expect("updated assignment resolves against the bound source");
    execute_runtime_list_clarke_wright(
        &slot,
        &source_index,
        &unassigned,
        crate::scope::StepControlPolicy::ObserveConfigLimits,
        &mut scope,
    )
    .expect("compiled Clarke-Wright refreshes current assignment");

    assert_eq!(scope.working_solution().routes, first_routes);
}

#[test]
fn compiled_round_robin_refreshes_current_assignment_before_a_second_execution() {
    let slot = static_slot();
    let plan = round_robin_plan();
    let binding = bind_runtime_list_source(&slot, &plan)
        .expect("well-formed source binds before phase construction");
    let source_index = binding.into_source_index();
    let unassigned = unassigned_from_current_assignment(&slot, &source_index, &plan)
        .expect("initial assignment resolves against the bound source");
    let mut scope = SolverScope::new(round_robin_director(plan));
    execute_runtime_list_round_robin(
        &slot,
        &source_index,
        &unassigned,
        crate::scope::StepControlPolicy::ObserveConfigLimits,
        &mut scope,
    )
    .expect("compiled round-robin executes");
    let first_routes = scope.working_solution().routes.clone();
    let unassigned =
        unassigned_from_current_assignment(&slot, &source_index, scope.working_solution())
            .expect("updated assignment resolves against the bound source");
    execute_runtime_list_round_robin(
        &slot,
        &source_index,
        &unassigned,
        crate::scope::StepControlPolicy::ObserveConfigLimits,
        &mut scope,
    )
    .expect("compiled round-robin refreshes current assignment");

    assert_eq!(scope.working_solution().routes, first_routes);
}
