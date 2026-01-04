//! Tests for phase factories.

use super::*;
use crate::heuristic::r#move::ChangeMove;
use crate::heuristic::selector::{FromSolutionEntitySelector, StaticTypedValueSelector};
use crate::phase::construction::{EntityPlacer, ForagerType, QueuedEntityPlacer};
use crate::scope::SolverScope;
use solverforge_scoring::SimpleScoreDirector;
use solverforge_core::domain::{
    EntityDescriptor, SolutionDescriptor, TypedEntityExtractor,
};
use solverforge_core::score::SimpleScore;
use std::any::TypeId;

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

// Zero-erasure typed getter/setter for solution-level access
fn get_task_priority(s: &TestSolution, idx: usize) -> Option<i64> {
    s.tasks.get(idx).and_then(|t| t.priority)
}

fn set_task_priority(s: &mut TestSolution, idx: usize, v: Option<i64>) {
    if let Some(task) = s.tasks.get_mut(idx) {
        task.priority = v;
    }
}

/// Score calculator: sum of priorities (higher is better), penalty for unassigned.
fn calculate_score(solution: &TestSolution) -> SimpleScore {
    let mut score = 0i64;
    for task in &solution.tasks {
        match task.priority {
            Some(p) => score += p,
            None => score -= 100, // Penalty for unassigned
        }
    }
    SimpleScore::of(score)
}

/// Creates a score director with the given tasks.
fn create_test_director(tasks: Vec<Task>) -> SimpleScoreDirector<TestSolution, impl Fn(&TestSolution) -> SimpleScore> {
    let solution = TestSolution { tasks, score: None };

    let extractor = Box::new(TypedEntityExtractor::new(
        "Task",
        "tasks",
        get_tasks,
        get_tasks_mut,
    ));let entity_desc = EntityDescriptor::new("Task", TypeId::of::<Task>(), "tasks")
        .with_extractor(extractor);

    let descriptor = SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>())
        .with_entity(entity_desc);

    SimpleScoreDirector::with_calculator(solution, descriptor, calculate_score)
}

/// Creates a solver scope with unassigned tasks.
fn create_unassigned_solver_scope(count: usize) -> SolverScope<TestSolution> {
    let tasks: Vec<Task> = (0..count)
        .map(|id| Task { id, priority: None })
        .collect();
    let director = create_test_director(tasks);
    SolverScope::new(Box::new(director))
}

type TestMove = ChangeMove<TestSolution, i64>;

// ==================== Helper Factories ====================

/// Creates a simple placer factory for construction phases.
fn create_placer_factory() -> impl Fn() -> Box<dyn EntityPlacer<TestSolution, TestMove>> + Send + Sync
{
    || {
        let entity_selector = Box::new(FromSolutionEntitySelector::new(0));
        let value_selector = Box::new(StaticTypedValueSelector::new(vec![1i64, 2, 3, 4, 5]));
        Box::new(QueuedEntityPlacer::new(
            entity_selector,
            value_selector,
            get_task_priority,
            set_task_priority,
            0,
            "priority",
        ))
    }
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

#[test]
fn test_forager_type_variants() {
    let _first = ForagerType::FirstFit;
    let _best = ForagerType::BestFit;
}

// ==================== ConstructionPhaseFactory Tests ====================

#[test]
fn test_construction_phase_factory_first_fit_creates_phase() {
    let factory = ConstructionPhaseFactory::<TestSolution, TestMove, _>::first_fit(
        create_placer_factory(),
    );

    let phase = factory.create_phase();
    assert_eq!(phase.phase_type_name(), "ConstructionHeuristic");
}

#[test]
fn test_construction_phase_factory_best_fit_creates_phase() {
    let factory = ConstructionPhaseFactory::<TestSolution, TestMove, _>::best_fit(
        create_placer_factory(),
    );

    let phase = factory.create_phase();
    assert_eq!(phase.phase_type_name(), "ConstructionHeuristic");
}

#[test]
fn test_construction_phase_factory_new_with_forager_type() {
    // Test FirstFit via new()
    let factory_first = ConstructionPhaseFactory::<TestSolution, TestMove, _>::new(
        ForagerType::FirstFit,
        create_placer_factory(),
    );
    let phase_first = factory_first.create_phase();
    assert_eq!(phase_first.phase_type_name(), "ConstructionHeuristic");

    // Test BestFit via new()
    let factory_best = ConstructionPhaseFactory::<TestSolution, TestMove, _>::new(
        ForagerType::BestFit,
        create_placer_factory(),
    );
    let phase_best = factory_best.create_phase();
    assert_eq!(phase_best.phase_type_name(), "ConstructionHeuristic");
}

#[test]
fn test_construction_phase_factory_first_fit_solves() {
    let factory = ConstructionPhaseFactory::<TestSolution, TestMove, _>::first_fit(
        create_placer_factory(),
    );

    let mut solver_scope = create_unassigned_solver_scope(3);

    // Verify tasks start unassigned
    let initial_solution = solver_scope.working_solution();
    for task in &initial_solution.tasks {
        assert!(task.priority.is_none());
    }

    // Create and run phase
    let mut phase = factory.create_phase();
    phase.solve(&mut solver_scope);

    // Verify all tasks are now assigned
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
fn test_construction_phase_factory_best_fit_solves() {
    let factory = ConstructionPhaseFactory::<TestSolution, TestMove, _>::best_fit(
        create_placer_factory(),
    );

    let mut solver_scope = create_unassigned_solver_scope(3);

    // Verify tasks start unassigned
    let initial_solution = solver_scope.working_solution();
    for task in &initial_solution.tasks {
        assert!(task.priority.is_none());
    }

    // Create and run phase
    let mut phase = factory.create_phase();
    phase.solve(&mut solver_scope);

    // Verify all tasks are now assigned
    let final_solution = solver_scope.working_solution();
    for task in &final_solution.tasks {
        assert!(
            task.priority.is_some(),
            "Task {} should have priority assigned",
            task.id
        );
    }

    // Best fit should pick the highest priorities (5, 5, 5 or similar)
    // Since our score calculator rewards higher priorities
    let total_priority: i64 = final_solution
        .tasks
        .iter()
        .filter_map(|t| t.priority)
        .sum();
    // With BestFit and values [1,2,3,4,5], each task should get 5
    assert_eq!(total_priority, 15, "BestFit should assign highest priorities");
}

#[test]
fn test_construction_phase_factory_creates_fresh_phases() {
    let factory = ConstructionPhaseFactory::<TestSolution, TestMove, _>::first_fit(
        create_placer_factory(),
    );

    // Create multiple phases - they should be independent
    let phase1 = factory.create_phase();
    let phase2 = factory.create_phase();

    // Both phases should work independently
    assert_eq!(phase1.phase_type_name(), "ConstructionHeuristic");
    assert_eq!(phase2.phase_type_name(), "ConstructionHeuristic");
}

#[test]
fn test_construction_phase_factory_implements_solver_phase_factory() {
    let factory = ConstructionPhaseFactory::<TestSolution, TestMove, _>::first_fit(
        create_placer_factory(),
    );

    // Verify we can use it as a trait object
    let factory_ref: &dyn SolverPhaseFactory<TestSolution> = &factory;
    let phase = factory_ref.create_phase();
    assert_eq!(phase.phase_type_name(), "ConstructionHeuristic");
}

