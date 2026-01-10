//! Tests for phase factories.

use crate::heuristic::r#move::ChangeMove;
use crate::heuristic::selector::{FromSolutionEntitySelector, StaticTypedValueSelector};
use crate::phase::construction::{
    ConstructionHeuristicPhase, FirstFitForager, QueuedEntityPlacer,
};
use crate::phase::Phase;
use crate::scope::SolverScope;
use solverforge_core::domain::{
    EntityDescriptor, PlanningSolution, SolutionDescriptor, TypedEntityExtractor,
};
use solverforge_core::score::SimpleScore;
use solverforge_scoring::SimpleScoreDirector;
use std::any::TypeId;

use super::config::LocalSearchType;
use super::{ConstructionPhaseFactory, SolverPhaseFactory};

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

type TestEntitySelector = FromSolutionEntitySelector;
type TestValueSelector = StaticTypedValueSelector<TestSolution, i64>;
type TestPlacer = QueuedEntityPlacer<TestSolution, i64, TestEntitySelector, TestValueSelector>;
type TestMove = ChangeMove<TestSolution, i64>;
type TestPhase = ConstructionHeuristicPhase<
    TestSolution,
    TestMove,
    TestPlacer,
    FirstFitForager<TestSolution, TestMove>,
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

// ==================== Basic Variant Tests ====================

#[test]
fn test_local_search_type_variants() {
    let _hill = LocalSearchType::HillClimbing;
    let _tabu = LocalSearchType::TabuSearch { tabu_size: 10 };
    let _sa = LocalSearchType::SimulatedAnnealing {
        starting_temp: 1.0,
        decay_rate: 0.99,
    };
    let _late = LocalSearchType::LateAcceptance { size: 100 };
}

// ==================== ConstructionPhaseFactory Tests ====================

#[test]
fn test_construction_phase_factory_first_fit_creates_phase() {
    let factory = ConstructionPhaseFactory::<TestSolution, TestMove, TestPlacer, _, _, _>::first_fit(
        create_placer,
    );

    let phase: TestPhase = factory.create_phase();
    assert_eq!(phase.phase_type_name(), "ConstructionHeuristic");
}

#[test]
fn test_construction_phase_factory_first_fit_solves() {
    let factory = ConstructionPhaseFactory::<TestSolution, TestMove, TestPlacer, _, _, _>::first_fit(
        create_placer,
    );

    let mut solver_scope = create_unassigned_solver_scope(3);

    let initial_solution = solver_scope.working_solution();
    for task in &initial_solution.tasks {
        assert!(task.priority.is_none());
    }

    let mut phase: TestPhase = factory.create_phase();
    phase.solve(&mut solver_scope);

    let final_solution = solver_scope.working_solution();
    for task in &final_solution.tasks {
        assert!(
            task.priority.is_some(),
            "Task {} should have priority assigned",
            task.id
        );
    }
}

#[test]
fn test_construction_phase_factory_creates_fresh_phases() {
    let factory = ConstructionPhaseFactory::<TestSolution, TestMove, TestPlacer, _, _, _>::first_fit(
        create_placer,
    );

    let phase1: TestPhase = factory.create_phase();
    let phase2: TestPhase = factory.create_phase();

    assert_eq!(phase1.phase_type_name(), "ConstructionHeuristic");
    assert_eq!(phase2.phase_type_name(), "ConstructionHeuristic");
}
