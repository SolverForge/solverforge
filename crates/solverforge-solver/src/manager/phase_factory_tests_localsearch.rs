//! Local search phase factory tests.

use crate::heuristic::r#move::ChangeMove;
use crate::heuristic::selector::{
    ChangeMoveSelector, FromSolutionEntitySelector, StaticTypedValueSelector,
};
use crate::phase::construction::{
    ConstructionHeuristicPhase, FirstFitForager, QueuedEntityPlacer,
};
use crate::phase::localsearch::{AcceptedCountForager, HillClimbingAcceptor, LocalSearchPhase};
use crate::phase::Phase;
use crate::scope::SolverScope;
use solverforge_core::domain::{
    EntityDescriptor, PlanningSolution, SolutionDescriptor, TypedEntityExtractor,
};
use solverforge_core::score::SimpleScore;
use solverforge_scoring::SimpleScoreDirector;
use std::any::TypeId;

use super::{ConstructionPhaseFactory, LocalSearchPhaseFactory, SolverPhaseFactory};

// ==================== Test Domain ====================

#[derive(Clone, Debug)]
struct Task {
    id: usize,
    priority: Option<i64>,
}

#[derive(Clone, Debug)]
struct TestSolution {
    tasks: Vec<Task>,
    score: Option<SimpleScore>,
}

impl PlanningSolution for TestSolution {
    type Score = SimpleScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn get_tasks(s: &TestSolution) -> &Vec<Task> {
    &s.tasks
}

fn get_tasks_mut(s: &mut TestSolution) -> &mut Vec<Task> {
    &mut s.tasks
}

fn get_task_priority(s: &TestSolution, idx: usize) -> Option<i64> {
    s.tasks.get(idx).and_then(|t| t.priority)
}

fn set_task_priority(s: &mut TestSolution, idx: usize, v: Option<i64>) {
    if let Some(task) = s.tasks.get_mut(idx) {
        task.priority = v;
    }
}

fn calculate_score(solution: &TestSolution) -> SimpleScore {
    let mut score = 0i64;
    for task in &solution.tasks {
        match task.priority {
            Some(p) => score += p,
            None => score -= 100,
        }
    }
    SimpleScore::of(score)
}

type TestDirector = SimpleScoreDirector<TestSolution, fn(&TestSolution) -> SimpleScore>;

fn create_test_director(tasks: Vec<Task>) -> TestDirector {
    let solution = TestSolution { tasks, score: None };

    let extractor = Box::new(TypedEntityExtractor::new(
        "Task",
        "tasks",
        get_tasks,
        get_tasks_mut,
    ));
    let entity_desc =
        EntityDescriptor::new("Task", TypeId::of::<Task>(), "tasks").with_extractor(extractor);

    let descriptor = SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>())
        .with_entity(entity_desc);

    SimpleScoreDirector::with_calculator(
        solution,
        descriptor,
        calculate_score as fn(&TestSolution) -> SimpleScore,
    )
}

fn create_unassigned_solver_scope(count: usize) -> SolverScope<TestSolution, TestDirector> {
    let tasks: Vec<Task> = (0..count)
        .map(|id| Task { id, priority: None })
        .collect();
    let director = create_test_director(tasks);
    SolverScope::new(director)
}

fn create_assigned_solver_scope(priorities: &[i64]) -> SolverScope<TestSolution, TestDirector> {
    let tasks: Vec<Task> = priorities
        .iter()
        .enumerate()
        .map(|(id, &p)| Task {
            id,
            priority: Some(p),
        })
        .collect();
    let director = create_test_director(tasks);
    SolverScope::new(director)
}

type TestMove = ChangeMove<TestSolution, i64>;
type TestMoveSelector = ChangeMoveSelector<TestSolution, i64>;

type TestEntitySelector = FromSolutionEntitySelector;
type TestValueSelector = StaticTypedValueSelector<TestSolution, i64>;
type TestPlacer = QueuedEntityPlacer<TestSolution, i64, TestEntitySelector, TestValueSelector>;

type TestConstructionPhase = ConstructionHeuristicPhase<
    TestSolution,
    TestMove,
    TestPlacer,
    FirstFitForager<TestSolution, TestMove>,
>;

type TestLocalSearchPhase = LocalSearchPhase<
    TestSolution,
    TestMove,
    TestMoveSelector,
    HillClimbingAcceptor,
    AcceptedCountForager<TestSolution, TestMove>,
>;

fn create_placer() -> TestPlacer {
    QueuedEntityPlacer::new(
        FromSolutionEntitySelector::new(0),
        StaticTypedValueSelector::new(vec![1i64, 2, 3, 4, 5]),
        get_task_priority,
        set_task_priority,
        0,
        "priority",
    )
}

fn create_move_selector() -> TestMoveSelector {
    ChangeMoveSelector::simple(
        get_task_priority,
        set_task_priority,
        0,
        "priority",
        vec![1i64, 2, 3, 4, 5],
    )
}

// ==================== LocalSearchPhaseFactory Tests ====================

#[test]
fn test_local_search_phase_factory_hill_climbing_creates_phase() {
    let factory = LocalSearchPhaseFactory::<TestSolution, TestMove, TestMoveSelector, _>::hill_climbing(
        create_move_selector,
    );

    let phase: TestLocalSearchPhase = factory.create_phase();
    assert_eq!(phase.phase_type_name(), "LocalSearch");
}

#[test]
fn test_local_search_phase_factory_hill_climbing_solves() {
    let factory = LocalSearchPhaseFactory::<TestSolution, TestMove, TestMoveSelector, _>::hill_climbing(
        create_move_selector,
    )
    .with_step_limit(10);

    let mut solver_scope = create_assigned_solver_scope(&[1, 1, 1]);
    let initial_score = solver_scope.calculate_score();

    let mut phase: TestLocalSearchPhase = factory.create_phase();
    phase.solve(&mut solver_scope);

    let final_score = solver_scope.best_score().cloned().unwrap_or(initial_score);
    assert!(
        final_score >= initial_score,
        "Score should not decrease: {:?} >= {:?}",
        final_score,
        initial_score
    );
}

#[test]
fn test_local_search_phase_factory_creates_fresh_phases() {
    let factory = LocalSearchPhaseFactory::<TestSolution, TestMove, TestMoveSelector, _>::hill_climbing(
        create_move_selector,
    );

    let phase1: TestLocalSearchPhase = factory.create_phase();
    let phase2: TestLocalSearchPhase = factory.create_phase();

    assert_eq!(phase1.phase_type_name(), "LocalSearch");
    assert_eq!(phase2.phase_type_name(), "LocalSearch");
}

// ==================== Integration Tests ====================

#[test]
fn test_construction_then_local_search() {
    let construction_factory =
        ConstructionPhaseFactory::<TestSolution, TestMove, TestPlacer, _, _, _>::first_fit(
            create_placer,
        );

    let local_search_factory =
        LocalSearchPhaseFactory::<TestSolution, TestMove, TestMoveSelector, _>::hill_climbing(
            create_move_selector,
        )
        .with_step_limit(10);

    let mut solver_scope = create_unassigned_solver_scope(3);

    let mut construction_phase: TestConstructionPhase = construction_factory.create_phase();
    construction_phase.solve(&mut solver_scope);

    let after_construction = solver_scope.working_solution();
    for task in &after_construction.tasks {
        assert!(
            task.priority.is_some(),
            "Task should be assigned after construction"
        );
    }

    let mut local_search_phase: TestLocalSearchPhase = local_search_factory.create_phase();
    local_search_phase.solve(&mut solver_scope);

    let final_solution = solver_scope.working_solution();
    for task in &final_solution.tasks {
        assert!(
            task.priority.is_some(),
            "Task should remain assigned after local search"
        );
    }
}

#[test]
fn test_local_search_phase_with_single_entity() {
    let factory = LocalSearchPhaseFactory::<TestSolution, TestMove, TestMoveSelector, _>::hill_climbing(
        create_move_selector,
    )
    .with_step_limit(5);

    let mut solver_scope = create_assigned_solver_scope(&[1]);

    let mut phase: TestLocalSearchPhase = factory.create_phase();
    phase.solve(&mut solver_scope);

    let final_solution = solver_scope.working_solution();
    assert_eq!(final_solution.tasks.len(), 1);
    assert!(final_solution.tasks[0].priority.is_some());
}
