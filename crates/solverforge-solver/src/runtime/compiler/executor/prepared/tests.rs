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
    CustomSearchPhase, ListVariableSlot, NoDynamicExtensions, NoTypedExtensions, RuntimeModel,
    SearchContext, VariableSlot,
};
use crate::heuristic::selector::nearby_list_change::DefaultCrossEntityDistanceMeter;
use crate::runtime::compiler::{
    compile_runtime_graph, DefaultPreconstructionStage, RuntimeGraphInput,
};
use crate::runtime_build_error::RuntimeBuildError;
use crate::scope::{ProgressCallback, SolverProgressKind, SolverProgressRef, SolverScope};
use crate::solver::Solver;

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
    let context = SearchContext::new(descriptor(), model(), config.random_seed);
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

#[test]
fn explicit_list_construction_stops_at_a_limit_reached_inside_the_kernel() {
    let _guard = TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    for heuristic in [
        ConstructionHeuristicType::ListRoundRobin,
        ConstructionHeuristicType::ListCheapestInsertion,
        ConstructionHeuristicType::ListRegretInsertion,
    ] {
        let mut phase = construction(heuristic);
        let PhaseConfig::ConstructionHeuristic(config) = &mut phase else {
            unreachable!("construction helper must return a construction phase");
        };
        config.termination = Some(TerminationConfig {
            step_count_limit: Some(1),
            ..TerminationConfig::default()
        });
        let config = SolverConfig {
            phases: vec![phase],
            ..SolverConfig::default()
        };
        let mut prepared = executor(&config)
            .instantiate()
            .expect("explicit list construction prepares");
        let construction = match prepared.phases.remove(0) {
            PreparedRuntimePhase::Construction(construction) => construction,
            _ => panic!("configured phase should remain construction"),
        };
        let input = plan((1..=16).collect(), vec![Vec::new(), Vec::new()]);
        let director = ScoreDirector::simple(input, descriptor(), |plan, _| entity_count(plan));
        let mut scope = SolverScope::new(director);

        assert!(
            execute_prepared_construction(&mut prepared, &construction, false, &mut scope)
                .expect("explicit list construction executes until its phase limit")
                .ran()
        );
        assert_eq!(
            scope.working_solution().routes.iter().flatten().count(),
            1,
            "{heuristic:?} must stop after the first committed step"
        );
    }
}

#[test]
fn required_construction_stops_at_an_ordinary_phase_limit() {
    let _guard = TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut phase = construction(ConstructionHeuristicType::ListRoundRobin);
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
        .expect("required construction prepares");
    let construction = match prepared.phases.remove(0) {
        PreparedRuntimePhase::Construction(construction) => construction,
        _ => panic!("configured phase should remain construction"),
    };
    let input = plan((1..=8).collect(), vec![Vec::new(), Vec::new()]);
    let director = ScoreDirector::simple(input, descriptor(), |plan, _| entity_count(plan));
    let mut scope = SolverScope::new(director);

    let execution = execute_prepared_construction(&mut prepared, &construction, true, &mut scope)
        .expect("required construction observes ordinary phase limits");
    assert!(!execution.ran());
    assert_eq!(scope.working_solution().routes.iter().flatten().count(), 0);
}

#[test]
fn compiled_runtime_fails_incomplete_list_work_without_publishing_a_best_solution() {
    let _guard = TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut phase = construction(ConstructionHeuristicType::ListRoundRobin);
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
    let executor = executor(&config);
    let runner =
        CompiledRuntimePhaseRunner::try_new(&executor).expect("list runtime runner must prepare");
    let best_solution_events = Arc::new(AtomicUsize::new(0));
    let observed_events = Arc::clone(&best_solution_events);
    let director = ScoreDirector::simple(
        plan((1..=8).collect(), vec![Vec::new(), Vec::new()]),
        descriptor(),
        |plan, _| entity_count(plan),
    );
    let solver = Solver::new((runner,))
        .with_config(config)
        .with_progress_callback(move |progress: SolverProgressRef<'_, Plan>| {
            if progress.kind == SolverProgressKind::BestSolution {
                observed_events.fetch_add(1, Ordering::SeqCst);
            }
        });

    let payload = std::panic::catch_unwind(AssertUnwindSafe(|| solver.solve(director)))
        .expect_err("incomplete mandatory list work must fail");
    let error = take_runtime_execution_failure(payload)
        .expect("compiled-runtime incompleteness must use the typed failure channel");
    assert!(matches!(
        error,
        RuntimeBuildError::Execution { phase_index: 0, .. }
    ));
    assert!(error.to_string().contains("8 unassigned element(s)"));
    assert_eq!(best_solution_events.load(Ordering::SeqCst), 0);
}

#[test]
fn compiled_runtime_publishes_only_after_list_work_is_complete() {
    let _guard = TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let config = SolverConfig {
        phases: vec![construction(ConstructionHeuristicType::ListRoundRobin)],
        ..SolverConfig::default()
    };
    let executor = executor(&config);
    let runner =
        CompiledRuntimePhaseRunner::try_new(&executor).expect("list runtime runner must prepare");
    let best_solution_events = Arc::new(AtomicUsize::new(0));
    let observed_events = Arc::clone(&best_solution_events);
    let director = ScoreDirector::simple(
        plan(vec![1, 2, 3], vec![Vec::new()]),
        descriptor(),
        |plan, _| entity_count(plan),
    );
    let solver = Solver::new((runner,))
        .with_config(config)
        .with_progress_callback(move |progress: SolverProgressRef<'_, Plan>| {
            if progress.kind == SolverProgressKind::BestSolution {
                let solution = progress
                    .solution
                    .expect("published best solution must carry a solution");
                assert_eq!(solution.routes.iter().flatten().count(), 3);
                observed_events.fetch_add(1, Ordering::SeqCst);
            }
        });

    let result = solver.solve(director);

    assert_eq!(result.solution.routes.iter().flatten().count(), 3);
    assert_eq!(best_solution_events.load(Ordering::SeqCst), 1);
}

#[test]
fn mandatory_completion_gate_recloses_if_work_becomes_incomplete() {
    let _guard = TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let config = SolverConfig {
        phases: vec![construction(ConstructionHeuristicType::ListRoundRobin)],
        ..SolverConfig::default()
    };
    let executor = executor(&config);
    let bindings = executor.graph().default_bindings().clone();
    let mut execution = executor
        .instantiate()
        .expect("list runtime execution must prepare");
    let published_sizes = Arc::new(Mutex::new(Vec::new()));
    let observed_sizes = Arc::clone(&published_sizes);
    let director = ScoreDirector::simple(
        plan(vec![1, 2, 3], vec![vec![1, 2, 3]]),
        descriptor(),
        |plan, _| entity_count(plan),
    );
    let mut scope = SolverScope::new_with_callback(
        director,
        move |progress: SolverProgressRef<'_, Plan>| {
            if progress.kind == SolverProgressKind::BestSolution {
                observed_sizes.lock().unwrap().push(
                    progress
                        .solution
                        .expect("best event carries a solution")
                        .routes
                        .iter()
                        .flatten()
                        .count(),
                );
            }
        },
        None,
        None,
    );
    scope.defer_best_solution_publication();
    scope.initialize_working_solution_as_best();
    let mut completion_published = false;

    assert!(publish_if_mandatory_complete(
        &mut execution,
        &bindings,
        &mut completion_published,
        0,
        &mut scope,
    )
    .expect("complete state publishes"));

    scope.replace_working_solution_and_reinitialize(plan(vec![1, 2, 3], vec![vec![1, 2]]));
    assert!(!publish_if_mandatory_complete(
        &mut execution,
        &bindings,
        &mut completion_published,
        0,
        &mut scope,
    )
    .expect("incomplete state recloses the gate"));
    scope.report_best_solution();

    scope.replace_working_solution_and_reinitialize(plan(vec![1, 2, 3], vec![vec![1, 2, 3]]));
    assert!(publish_if_mandatory_complete(
        &mut execution,
        &bindings,
        &mut completion_published,
        0,
        &mut scope,
    )
    .expect("restored complete state republishes"));

    assert_eq!(*published_sizes.lock().unwrap(), vec![3, 3]);
}

#[test]
fn already_complete_list_work_can_terminate_by_config_and_publish() {
    let _guard = TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let config = SolverConfig {
        phases: vec![construction(ConstructionHeuristicType::ListRoundRobin)],
        ..SolverConfig::default()
    };
    let executor = executor(&config);
    let runner =
        CompiledRuntimePhaseRunner::try_new(&executor).expect("list runtime runner must prepare");
    let best_solution_events = Arc::new(AtomicUsize::new(0));
    let observed_events = Arc::clone(&best_solution_events);
    let director = ScoreDirector::simple(
        plan(vec![1, 2, 3], vec![vec![1, 2, 3]]),
        descriptor(),
        |plan, _| entity_count(plan),
    );
    let solver = Solver::new((runner,))
        .with_config(config)
        .with_time_limit(Duration::ZERO)
        .with_progress_callback(move |progress: SolverProgressRef<'_, Plan>| {
            if progress.kind == SolverProgressKind::BestSolution {
                observed_events.fetch_add(1, Ordering::SeqCst);
            }
        });

    let result = solver.solve(director);

    assert_eq!(
        result.terminal_reason,
        crate::manager::SolverTerminalReason::TerminatedByConfig
    );
    assert_eq!(result.solution.routes, vec![vec![1, 2, 3]]);
    assert_eq!(best_solution_events.load(Ordering::SeqCst), 1);
}

#[test]
fn fully_counted_default_rejects_duplicate_assigned_source_key() {
    let _guard = TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    SOURCE_KEY_CALLS.store(0, Ordering::SeqCst);
    let mut prepared = executor(&SolverConfig::default())
        .instantiate()
        .expect("structural preparation must not validate a source");
    let default = match prepared.phases.remove(0) {
        PreparedRuntimePhase::DefaultRuntime(default) => default,
        _ => panic!("omitted configuration must retain one unresolved default node"),
    };
    let input = plan(vec![7, 8], vec![vec![7, 7]]);
    let director = ScoreDirector::simple(input, descriptor(), |plan, _| entity_count(plan));
    let mut scope = SolverScope::new(director);

    let error = execute_prepared_default_construction(&mut prepared, &default, &mut scope)
        .expect_err("equal raw counts must not hide a duplicate assigned source key");
    assert!(matches!(
        error.kind,
        RuntimeInstantiationErrorKind::SourceBinding {
            error: crate::builder::context::ListConstructionKernelError::DuplicateAssignedElement {
                source_index: 0,
                first_assigned_occurrence: 0,
                duplicate_assigned_occurrence: 1,
            },
            ..
        }
    ));
    assert_eq!(prepared.bound_list_source_count(), 0);
}

#[test]
fn fully_counted_default_rejects_undeclared_assigned_source_key() {
    let _guard = TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    SOURCE_KEY_CALLS.store(0, Ordering::SeqCst);
    let mut prepared = executor(&SolverConfig::default())
        .instantiate()
        .expect("structural preparation must not validate a source");
    let default = match prepared.phases.remove(0) {
        PreparedRuntimePhase::DefaultRuntime(default) => default,
        _ => panic!("omitted configuration must retain one unresolved default node"),
    };
    let input = plan(vec![7, 8], vec![vec![7, 9]]);
    let director = ScoreDirector::simple(input, descriptor(), |plan, _| entity_count(plan));
    let mut scope = SolverScope::new(director);

    let error = execute_prepared_default_construction(&mut prepared, &default, &mut scope)
        .expect_err("equal raw counts must not hide an undeclared assigned source key");
    assert!(matches!(
        error.kind,
        RuntimeInstantiationErrorKind::SourceBinding {
            error:
                crate::builder::context::ListConstructionKernelError::AssignedElementNotDeclared {
                    assigned_occurrence: 1,
                },
            ..
        }
    ));
    assert_eq!(prepared.bound_list_source_count(), 0);
}

#[test]
fn valid_fully_assigned_default_binds_once_and_skips_list_work() {
    let _guard = TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    SOURCE_KEY_CALLS.store(0, Ordering::SeqCst);
    let mut prepared = executor(&SolverConfig::default())
        .instantiate()
        .expect("structural preparation must not validate a source");
    let default = match prepared.phases.remove(0) {
        PreparedRuntimePhase::DefaultRuntime(default) => default,
        _ => panic!("omitted configuration must retain one unresolved default node"),
    };
    let input = plan(vec![7, 8], vec![vec![7, 8]]);
    let director = ScoreDirector::simple(input, descriptor(), |plan, _| entity_count(plan));
    let mut scope = SolverScope::new(director);

    let execution = execute_prepared_default_construction(&mut prepared, &default, &mut scope)
        .expect("a valid complete source should bind and report no list-construction work");
    let list_stage = execution
        .stages
        .iter()
        .find(|stage| {
            stage.stage
                == crate::runtime::compiler::DefaultConstructionStage::Preconstruction(
                    DefaultPreconstructionStage::ListConstruction,
                )
        })
        .expect("default execution records the list-construction boundary");
    assert_eq!(
        list_stage.outcome,
        ResolvedConstructionExecutionOutcome::SkippedNoWork
    );
    assert_eq!(prepared.bound_list_source_count(), 1);
    assert_eq!(SOURCE_KEY_CALLS.load(Ordering::SeqCst), 4);
}

#[test]
fn staged_default_prepares_clarke_wright_from_the_structural_catalog() {
    let _guard = TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    SOURCE_KEY_CALLS.store(0, Ordering::SeqCst);
    let executor = executor(&SolverConfig::default());
    let prepared = executor
        .instantiate()
        .expect("default source registration is structural");
    let phase_index = match prepared.phases.as_slice() {
        [PreparedRuntimePhase::DefaultRuntime(default)] => default.phase_index,
        _ => panic!("omitted configuration must remain one unresolved default node"),
    };
    let input = plan(vec![1, 2, 3], vec![Vec::new()]);
    let staged = super::super::super::defaults::resolve_default_preconstruction_stage(
        executor.graph().default_bindings(),
        DefaultPreconstructionStage::ListConstruction,
        &input,
    );
    let [step] = staged.steps.as_slice() else {
        panic!("savings-capable default must choose exactly one CW child");
    };

    let resolved = prepared
        .prepare_resolved_construction(phase_index, &step.construction)
        .expect("staged CW resolves the structural source catalog");
    assert!(matches!(
        resolved,
        PreparedConstruction::ClarkeWright { ref slots, .. }
            if slots.len() == 1 && slots[0].catalog_index == 0
    ));
    assert_eq!(SOURCE_KEY_CALLS.load(Ordering::SeqCst), 0);
}

#[test]
fn staged_default_executes_canonical_clarke_wright_at_the_reached_boundary() {
    let _guard = TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    SOURCE_KEY_CALLS.store(0, Ordering::SeqCst);
    let executor = executor(&SolverConfig::default());
    let mut prepared = executor
        .instantiate()
        .expect("default source registration is structural");
    let default = match prepared.phases.remove(0) {
        PreparedRuntimePhase::DefaultRuntime(default) => default,
        _ => panic!("omitted configuration must retain one staged default node"),
    };
    let input = plan(vec![1, 2, 3], vec![Vec::new()]);
    let director = ScoreDirector::simple(input, descriptor(), |plan, _| entity_count(plan));
    let mut scope = SolverScope::new(director);

    assert!(
        execute_prepared_default_construction(&mut prepared, &default, &mut scope)
            .expect("reached default CW binds and executes")
            .ran_child_phase
    );
    assert_eq!(scope.working_solution().routes.iter().flatten().count(), 3);
    assert_eq!(SOURCE_KEY_CALLS.load(Ordering::SeqCst), 3);
    assert_eq!(prepared.bound_list_source_count(), 1);
}

#[derive(Debug)]
struct Marker;

impl CustomSearchPhase<Plan> for Marker {
    fn solve<D, ProgressCb>(&mut self, _: &mut SolverScope<'_, Plan, D, ProgressCb>)
    where
        D: solverforge_scoring::Director<Plan>,
        ProgressCb: ProgressCallback<Plan>,
    {
    }
}

fn build_marker(_: &SearchContext<Plan, usize, Meter, Meter>) -> Marker {
    EXTENSION_BUILD_CALLS.fetch_add(1, Ordering::SeqCst);
    Marker
}

#[test]
fn eager_extension_preparation_precedes_lazy_source_binding() {
    let _guard = TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    EXTENSION_BUILD_CALLS.store(0, Ordering::SeqCst);
    let config = SolverConfig {
        phases: vec![
            PhaseConfig::Custom(CustomPhaseConfig {
                name: "marker".to_string(),
            }),
            construction(ConstructionHeuristicType::ListCheapestInsertion),
        ],
        ..SolverConfig::default()
    };
    let context = SearchContext::new(descriptor(), model(), config.random_seed);
    let input = RuntimeGraphInput::new(
        context,
        CustomPhaseNode::new(NoTypedExtensions, "marker", build_marker),
    );
    let graph = compile_runtime_graph(&config, input).expect("extension name is declared");
    let mut prepared = CompiledRuntimeExecutor::new(graph)
        .instantiate()
        .expect("source binding belongs to the reached construction boundary");
    assert_eq!(EXTENSION_BUILD_CALLS.load(Ordering::SeqCst), 1);
    let construction = match prepared.phases.remove(1) {
        PreparedRuntimePhase::Construction(construction) => construction,
        _ => panic!("configured construction should remain after eager extension preparation"),
    };
    let input = plan(vec![9, 9], vec![Vec::new()]);
    let director = ScoreDirector::simple(input, descriptor(), |plan, _| entity_count(plan));
    let mut scope = SolverScope::new(director);
    let error = execute_prepared_construction(&mut prepared, &construction, false, &mut scope)
        .expect_err("the reached malformed source must fail at its own boundary");

    assert!(matches!(
        error.kind,
        RuntimeInstantiationErrorKind::SourceBinding { .. }
    ));
    assert_eq!(EXTENSION_BUILD_CALLS.load(Ordering::SeqCst), 1);
}
