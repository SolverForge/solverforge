use crate::builder::search::custom::NoDynamicExtensions;
use crate::builder::search::custom::NoTypedExtensions;
use std::any::TypeId;
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use solverforge_config::{
    ConstructionHeuristicConfig, ConstructionHeuristicType, CustomPhaseConfig, PhaseConfig,
    SolverConfig, TerminationConfig,
};
use solverforge_core::domain::{
    EntityClassId, EntityDescriptor, PlanningSolution, SolutionDescriptor, VariableDescriptor,
    VariableId,
};
use solverforge_core::score::SoftScore;
use solverforge_scoring::ScoreDirector;

use super::super::completion::publish_if_mandatory_complete;
use super::super::{
    execute_prepared_construction, execute_prepared_default_construction,
    take_runtime_execution_failure, CompiledRuntimePhaseRunner,
    ResolvedConstructionExecutionOutcome,
};
use super::*;
use crate::builder::search::CustomPhaseNode;
use crate::builder::{
    CustomSearchPhase, ListVariableSlot, RuntimeModel, SearchContext, VariableSlot,
};
use crate::heuristic::selector::nearby_list_change::DefaultCrossEntityDistanceMeter;
use crate::runtime::compiler::{
    compile_runtime_graph, DefaultPreconstructionStage, RuntimeGraphInput,
};
use crate::runtime_build_error::RuntimeBuildError;
use crate::scope::{ProgressCallback, SolverProgressKind, SolverProgressRef, SolverScope};
use crate::solver::Solver;

mod completion;
mod defaults;
mod publication;

type Meter = DefaultCrossEntityDistanceMeter;
type Model = RuntimeModel<Plan, usize, Meter, Meter>;

static SOURCE_KEY_CALLS: AtomicUsize = AtomicUsize::new(0);
static EXTENSION_BUILD_CALLS: AtomicUsize = AtomicUsize::new(0);
static TEST_LOCK: Mutex<()> = Mutex::new(());

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

fn assigned(plan: &Plan) -> Vec<usize> {
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

fn counting_source_key(_: &Plan, value: &usize) -> usize {
    SOURCE_KEY_CALLS.fetch_add(1, Ordering::SeqCst);
    *value
}

fn route_values(plan: &Plan, entity: usize) -> Vec<usize> {
    plan.routes[entity].clone()
}

fn replace_route(plan: &mut Plan, entity: usize, route: Vec<usize>) {
    plan.routes[entity] = route;
}

fn depot(_: &Plan, _: usize) -> usize {
    0
}

fn distance(_: &Plan, _: usize, from: usize, to: usize) -> i64 {
    from.abs_diff(to) as i64
}

fn feasible(_: &Plan, _: usize, _: &[usize]) -> bool {
    true
}

fn model() -> Model {
    let slot = ListVariableSlot::new(
        "Vehicle",
        element_count,
        assigned,
        list_len,
        list_remove,
        list_insert,
        list_get,
        list_set,
        list_reverse,
        sublist_remove,
        sublist_insert,
        construction_remove,
        list_insert,
        index_to_element,
        counting_source_key,
        entity_count,
        DefaultCrossEntityDistanceMeter,
        DefaultCrossEntityDistanceMeter,
        "visits",
        0,
        Some(route_values),
        Some(replace_route),
        Some(depot),
        Some(distance),
        Some(feasible),
        Some(depot),
        None,
        Some(distance),
        Some(feasible),
    );
    RuntimeModel::new(vec![VariableSlot::List(slot)])
}

fn construction(heuristic: ConstructionHeuristicType) -> PhaseConfig {
    PhaseConfig::ConstructionHeuristic(ConstructionHeuristicConfig {
        construction_heuristic_type: heuristic,
        ..ConstructionHeuristicConfig::default()
    })
}

fn plan(elements: Vec<usize>, routes: Vec<Vec<usize>>) -> Plan {
    Plan {
        score: None,
        elements,
        routes,
    }
}

fn executor(
    config: &SolverConfig,
) -> CompiledRuntimeExecutor<Plan, usize, Meter, Meter, NoDynamicExtensions> {
    let context = SearchContext::new(descriptor(), model(), config.random_seed)
        .expect("search context should bind");
    let graph = compile_runtime_graph(config, RuntimeGraphInput::new(context, NoDynamicExtensions))
        .expect("test graph compiles");
    CompiledRuntimeExecutor::new(graph)
}

#[test]
fn registration_is_structural_and_reuses_one_reached_source_binding() {
    let _guard = TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    SOURCE_KEY_CALLS.store(0, Ordering::SeqCst);
    let config = SolverConfig {
        phases: vec![
            construction(ConstructionHeuristicType::ListCheapestInsertion),
            construction(ConstructionHeuristicType::ListCheapestInsertion),
        ],
        ..SolverConfig::default()
    };
    let mut prepared = executor(&config)
        .instantiate()
        .expect("registration must not inspect a solve source");

    assert_eq!(SOURCE_KEY_CALLS.load(Ordering::SeqCst), 0);
    assert_eq!(prepared.bound_list_source_count(), 0);
    let (first_catalog_index, second_catalog_index) = match prepared.phases.as_slice() {
        [PreparedRuntimePhase::Construction(PreparedConstruction::CheapestInsertion {
            slots: first,
            ..
        }), PreparedRuntimePhase::Construction(PreparedConstruction::CheapestInsertion {
            slots: second,
            ..
        })] => (first[0].catalog_index, second[0].catalog_index),
        _ => panic!("both configured phases should retain cheapest-insertion identity"),
    };
    assert_eq!(first_catalog_index, second_catalog_index);

    let first = match prepared.phases.remove(0) {
        PreparedRuntimePhase::Construction(construction) => construction,
        _ => panic!("first phase should be construction"),
    };
    let second = match prepared.phases.remove(0) {
        PreparedRuntimePhase::Construction(construction) => construction,
        _ => panic!("second phase should be construction"),
    };
    let input = plan(vec![1, 2, 3], vec![Vec::new()]);
    let director = ScoreDirector::simple(input, descriptor(), |plan, _| entity_count(plan));
    let mut scope = SolverScope::new(director);

    assert!(
        execute_prepared_construction(&mut prepared, &first, false, &mut scope)
            .expect("first reached construction binds its source")
            .ran()
    );
    assert_eq!(SOURCE_KEY_CALLS.load(Ordering::SeqCst), 3);
    assert_eq!(prepared.bound_list_source_count(), 1);

    scope.mutate(|director| director.working_solution_mut().routes = vec![Vec::new()]);
    assert!(
        execute_prepared_construction(&mut prepared, &second, false, &mut scope)
            .expect("second construction reuses its solve-owned binding")
            .ran()
    );
    assert_eq!(SOURCE_KEY_CALLS.load(Ordering::SeqCst), 3);
    assert_eq!(prepared.bound_list_source_count(), 1);
}

#[test]
fn reached_construction_binds_the_current_source_once() {
    let _guard = TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    SOURCE_KEY_CALLS.store(0, Ordering::SeqCst);
    let config = SolverConfig {
        phases: vec![construction(
            ConstructionHeuristicType::ListCheapestInsertion,
        )],
        ..SolverConfig::default()
    };
    let mut prepared = executor(&config)
        .instantiate()
        .expect("preparation must not bind a source");
    let construction = match prepared.phases.remove(0) {
        PreparedRuntimePhase::Construction(construction) => construction,
        _ => panic!("configured list construction must prepare one construction node"),
    };
    let input = plan(vec![99, 2, 3], vec![Vec::new()]);
    let director = ScoreDirector::simple(input, descriptor(), |plan, _| entity_count(plan));
    let mut scope = SolverScope::new(director);

    assert_eq!(SOURCE_KEY_CALLS.load(Ordering::SeqCst), 0);
    assert!(
        execute_prepared_construction(&mut prepared, &construction, false, &mut scope)
            .expect("reached source should bind and execute")
            .ran()
    );
    assert_eq!(SOURCE_KEY_CALLS.load(Ordering::SeqCst), 3);
    assert!(scope
        .working_solution()
        .routes
        .iter()
        .flatten()
        .any(|value| *value == 99));
}

#[test]
fn prepared_construction_dispatches_the_canonical_clarke_wright_kernel() {
    let _guard = TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    SOURCE_KEY_CALLS.store(0, Ordering::SeqCst);
    let config = SolverConfig {
        phases: vec![construction(ConstructionHeuristicType::ListClarkeWright)],
        ..SolverConfig::default()
    };
    let mut prepared = executor(&config)
        .instantiate()
        .expect("CW registers structurally");
    let construction = match prepared.phases.remove(0) {
        PreparedRuntimePhase::Construction(
            construction @ PreparedConstruction::ClarkeWright { .. },
        ) => construction,
        _ => panic!("configured CW must remain a distinct prepared construction node"),
    };
    let input = plan(vec![1, 2, 3], vec![Vec::new()]);
    let director = ScoreDirector::simple(input, descriptor(), |plan, _| entity_count(plan));
    let mut scope = SolverScope::new(director);

    assert_eq!(SOURCE_KEY_CALLS.load(Ordering::SeqCst), 0);
    assert!(
        execute_prepared_construction(&mut prepared, &construction, false, &mut scope)
            .expect("CW source binds only at its reached construction boundary")
            .ran()
    );
    assert_eq!(
        scope.working_solution().routes.iter().flatten().count(),
        3,
        "the prepared branch runs CW construction rather than insertion substitution"
    );
    assert_eq!(SOURCE_KEY_CALLS.load(Ordering::SeqCst), 3);
}

#[test]
fn kopt_registers_completion_metadata_without_binding_the_source_stream() {
    let _guard = TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    SOURCE_KEY_CALLS.store(0, Ordering::SeqCst);
    let config = SolverConfig {
        phases: vec![construction(ConstructionHeuristicType::ListKOpt)],
        ..SolverConfig::default()
    };
    let prepared = executor(&config)
        .instantiate()
        .expect("K-opt reads existing routes and does not consume a source stream");

    assert_eq!(SOURCE_KEY_CALLS.load(Ordering::SeqCst), 0);
    assert_eq!(prepared.bound_list_source_count(), 0);
    assert_eq!(prepared.list_source_indices.len(), 1);
    assert!(matches!(
        &prepared.phases[0],
        PreparedRuntimePhase::Construction(PreparedConstruction::KOpt { slots, .. })
            if slots.len() == 1
    ));
}

#[test]
fn phase_terminated_construction_does_not_bind_a_source() {
    let _guard = TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    SOURCE_KEY_CALLS.store(0, Ordering::SeqCst);
    let mut phase = construction(ConstructionHeuristicType::ListCheapestInsertion);
    let PhaseConfig::ConstructionHeuristic(config) = &mut phase else {
        unreachable!("construction helper must return a construction phase");
    };
    config.termination = Some(TerminationConfig {
        step_count_limit: Some(0),
        ..TerminationConfig::default()
    });
    let config = SolverConfig {
        phases: vec![phase],
        ..SolverConfig::default()
    };
    let mut prepared = executor(&config)
        .instantiate()
        .expect("phase termination must not force source binding during preparation");
    let construction = match prepared.phases.remove(0) {
        PreparedRuntimePhase::Construction(construction) => construction,
        _ => panic!("configured phase should remain construction"),
    };
    let input = plan(vec![1, 2, 3], vec![Vec::new()]);
    let director = ScoreDirector::simple(input, descriptor(), |plan, _| entity_count(plan));
    let mut scope = SolverScope::new(director);

    assert!(
        !execute_prepared_construction(&mut prepared, &construction, false, &mut scope)
            .expect("an expired phase boundary should skip construction")
            .ran()
    );
    assert_eq!(SOURCE_KEY_CALLS.load(Ordering::SeqCst), 0);
    assert_eq!(prepared.bound_list_source_count(), 0);
}
