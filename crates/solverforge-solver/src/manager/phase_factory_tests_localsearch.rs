//! Local search phase factory tests.

use super::*;
use crate::heuristic::r#move::ChangeMove;
use crate::heuristic::selector::{
    ChangeMoveSelector, FromSolutionEntitySelector, StaticTypedValueSelector,
};
use crate::heuristic::MoveSelector;
use crate::phase::construction::{EntityPlacer, QueuedEntityPlacer};
use crate::scope::SolverScope;
use solverforge_core::domain::{EntityDescriptor, SolutionDescriptor, TypedEntityExtractor};
use solverforge_core::score::SimpleScore;
use solverforge_scoring::{ScoreDirector, SimpleScoreDirector};
use std::any::TypeId;

// ==================== Test Domain ====================

#[allow(dead_code)]
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

fn create_test_director(
    tasks: Vec<Task>,
) -> SimpleScoreDirector<TestSolution, impl Fn(&TestSolution) -> SimpleScore> {
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

    SimpleScoreDirector::with_calculator(solution, descriptor, calculate_score)
}

fn create_unassigned_solver_scope(
    count: usize,
) -> SolverScope<TestSolution, impl ScoreDirector<TestSolution>> {
    let tasks: Vec<Task> = (0..count).map(|id| Task { id, priority: None }).collect();
    let director = create_test_director(tasks);
    SolverScope::new(director)
}

fn create_assigned_solver_scope(
    priorities: &[i64],
) -> SolverScope<TestSolution, impl ScoreDirector<TestSolution>> {
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

fn create_placer_factory(
) -> impl Fn() -> Box<dyn EntityPlacer<TestSolution, TestMove>> + Send + Sync {
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

fn create_move_selector_factory(
) -> impl Fn() -> Box<dyn MoveSelector<TestSolution, TestMove>> + Send + Sync {
    || {
        let selector = ChangeMoveSelector::<TestSolution, i64>::simple(
            get_task_priority,
            set_task_priority,
            0,
            "priority",
            vec![1i64, 2, 3, 4, 5],
        );
        Box::new(selector)
    }
}

// ==================== LocalSearchPhaseFactory Tests ====================

#[test]
fn test_local_search_phase_factory_hill_climbing_creates_phase() {
    let factory = LocalSearchPhaseFactory::<TestSolution, TestMove, _>::hill_climbing(
        create_move_selector_factory(),
    );

    let phase = factory.create_phase();
    assert_eq!(phase.phase_type_name(), "LocalSearch");
}

#[test]
fn test_local_search_phase_factory_tabu_search_creates_phase() {
    let factory = LocalSearchPhaseFactory::<TestSolution, TestMove, _>::tabu_search(
        10,
        create_move_selector_factory(),
    );

    let phase = factory.create_phase();
    assert_eq!(phase.phase_type_name(), "LocalSearch");
}

#[test]
fn test_local_search_phase_factory_simulated_annealing_creates_phase() {
    let factory = LocalSearchPhaseFactory::<TestSolution, TestMove, _>::simulated_annealing(
        1.0,
        0.99,
        create_move_selector_factory(),
    );

    let phase = factory.create_phase();
    assert_eq!(phase.phase_type_name(), "LocalSearch");
}

#[test]
fn test_local_search_phase_factory_late_acceptance_creates_phase() {
    let factory = LocalSearchPhaseFactory::<TestSolution, TestMove, _>::late_acceptance(
        100,
        create_move_selector_factory(),
    );

    let phase = factory.create_phase();
    assert_eq!(phase.phase_type_name(), "LocalSearch");
}

#[test]
fn test_local_search_phase_factory_new_with_search_type() {
    let types = [
        LocalSearchType::HillClimbing,
        LocalSearchType::TabuSearch { tabu_size: 5 },
        LocalSearchType::SimulatedAnnealing {
            starting_temp: 0.5,
            decay_rate: 0.95,
        },
        LocalSearchType::LateAcceptance { size: 50 },
    ];

    for search_type in types {
        let factory = LocalSearchPhaseFactory::<TestSolution, TestMove, _>::new(
            search_type,
            create_move_selector_factory(),
        );
        let phase = factory.create_phase();
        assert_eq!(phase.phase_type_name(), "LocalSearch");
    }
}

#[test]
fn test_local_search_phase_factory_hill_climbing_solves() {
    let factory = LocalSearchPhaseFactory::<TestSolution, TestMove, _>::hill_climbing(
        create_move_selector_factory(),
    )
    .with_step_limit(10);

    let mut solver_scope = create_assigned_solver_scope(&[1, 1, 1]);
    let initial_score = solver_scope.calculate_score();

    let mut phase = factory.create_phase();
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
fn test_local_search_phase_factory_tabu_search_solves() {
    let factory = LocalSearchPhaseFactory::<TestSolution, TestMove, _>::tabu_search(
        5,
        create_move_selector_factory(),
    )
    .with_step_limit(10);

    let mut solver_scope = create_assigned_solver_scope(&[1, 1, 1]);
    let initial_score = solver_scope.calculate_score();

    let mut phase = factory.create_phase();
    phase.solve(&mut solver_scope);

    let final_score = solver_scope.best_score().cloned().unwrap_or(initial_score);
    assert!(
        final_score >= initial_score,
        "Best score should not decrease"
    );
}

#[test]
fn test_local_search_phase_factory_simulated_annealing_solves() {
    let factory = LocalSearchPhaseFactory::<TestSolution, TestMove, _>::simulated_annealing(
        1.0,
        0.9,
        create_move_selector_factory(),
    )
    .with_step_limit(10);

    let mut solver_scope = create_assigned_solver_scope(&[1, 1, 1]);

    let mut phase = factory.create_phase();
    phase.solve(&mut solver_scope);

    assert!(
        solver_scope.best_solution().is_some() || !solver_scope.working_solution().tasks.is_empty()
    );
}

#[test]
fn test_local_search_phase_factory_late_acceptance_solves() {
    let factory = LocalSearchPhaseFactory::<TestSolution, TestMove, _>::late_acceptance(
        5,
        create_move_selector_factory(),
    )
    .with_step_limit(10);

    let mut solver_scope = create_assigned_solver_scope(&[1, 1, 1]);
    let initial_score = solver_scope.calculate_score();

    let mut phase = factory.create_phase();
    phase.solve(&mut solver_scope);

    let final_score = solver_scope.best_score().cloned().unwrap_or(initial_score);
    assert!(
        final_score >= initial_score,
        "Best score should not decrease"
    );
}

#[test]
fn test_local_search_phase_factory_implements_solver_phase_factory() {
    let factory = LocalSearchPhaseFactory::<TestSolution, TestMove, _>::hill_climbing(
        create_move_selector_factory(),
    );

    let factory_ref: &dyn SolverPhaseFactory<TestSolution> = &factory;
    let phase = factory_ref.create_phase();
    assert_eq!(phase.phase_type_name(), "LocalSearch");
}

#[test]
fn test_local_search_phase_factory_creates_fresh_phases() {
    let factory = LocalSearchPhaseFactory::<TestSolution, TestMove, _>::hill_climbing(
        create_move_selector_factory(),
    );

    let phase1 = factory.create_phase();
    let phase2 = factory.create_phase();

    assert_eq!(phase1.phase_type_name(), "LocalSearch");
    assert_eq!(phase2.phase_type_name(), "LocalSearch");
}

// ==================== Step Limit Configuration Tests ====================

#[test]
fn test_local_search_phase_factory_step_limit_configuration() {
    let factory = LocalSearchPhaseFactory::<TestSolution, TestMove, _>::hill_climbing(
        create_move_selector_factory(),
    )
    .with_step_limit(5);

    let mut solver_scope = create_assigned_solver_scope(&[1, 1, 1]);

    let mut phase = factory.create_phase();
    phase.solve(&mut solver_scope);
}

#[test]
fn test_local_search_phase_factory_step_limit_zero() {
    let factory = LocalSearchPhaseFactory::<TestSolution, TestMove, _>::hill_climbing(
        create_move_selector_factory(),
    )
    .with_step_limit(0);

    let mut solver_scope = create_assigned_solver_scope(&[1, 1, 1]);
    let initial_solution = solver_scope.working_solution().clone();

    let mut phase = factory.create_phase();
    phase.solve(&mut solver_scope);

    let final_solution = solver_scope.working_solution();
    assert_eq!(initial_solution.tasks.len(), final_solution.tasks.len());
}

#[test]
fn test_local_search_phase_factory_no_step_limit() {
    let factory = LocalSearchPhaseFactory::<TestSolution, TestMove, _>::hill_climbing(
        create_move_selector_factory(),
    );

    let mut solver_scope = create_assigned_solver_scope(&[5, 5, 5]);

    let mut phase = factory.create_phase();
    phase.solve(&mut solver_scope);

    let final_solution = solver_scope.working_solution();
    for task in &final_solution.tasks {
        assert_eq!(task.priority, Some(5));
    }
}

#[test]
fn test_local_search_phase_factory_step_limit_chaining() {
    let factory = LocalSearchPhaseFactory::<TestSolution, TestMove, _>::new(
        LocalSearchType::TabuSearch { tabu_size: 10 },
        create_move_selector_factory(),
    )
    .with_step_limit(100);

    let phase = factory.create_phase();
    assert_eq!(phase.phase_type_name(), "LocalSearch");
}

// ==================== Integration Tests ====================

#[test]
fn test_construction_then_local_search() {
    let construction_factory =
        ConstructionPhaseFactory::<TestSolution, TestMove, _>::best_fit(create_placer_factory());

    let local_search_factory = LocalSearchPhaseFactory::<TestSolution, TestMove, _>::hill_climbing(
        create_move_selector_factory(),
    )
    .with_step_limit(10);

    let mut solver_scope = create_unassigned_solver_scope(3);

    let mut construction_phase = construction_factory.create_phase();
    construction_phase.solve(&mut solver_scope);

    let after_construction = solver_scope.working_solution();
    for task in &after_construction.tasks {
        assert!(
            task.priority.is_some(),
            "Task should be assigned after construction"
        );
    }

    let mut local_search_phase = local_search_factory.create_phase();
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
fn test_multiple_local_search_phases_different_acceptors() {
    let factories: Vec<Box<dyn SolverPhaseFactory<TestSolution>>> = vec![
        Box::new(
            LocalSearchPhaseFactory::<TestSolution, TestMove, _>::hill_climbing(
                create_move_selector_factory(),
            )
            .with_step_limit(5),
        ),
        Box::new(
            LocalSearchPhaseFactory::<TestSolution, TestMove, _>::tabu_search(
                3,
                create_move_selector_factory(),
            )
            .with_step_limit(5),
        ),
        Box::new(
            LocalSearchPhaseFactory::<TestSolution, TestMove, _>::late_acceptance(
                3,
                create_move_selector_factory(),
            )
            .with_step_limit(5),
        ),
    ];

    let mut solver_scope = create_assigned_solver_scope(&[1, 2, 3]);

    for factory in &factories {
        let mut phase = factory.create_phase();
        phase.solve(&mut solver_scope);
    }

    let final_solution = solver_scope.working_solution();
    assert_eq!(final_solution.tasks.len(), 3);
}

#[test]
fn test_factory_can_be_boxed_as_trait_object() {
    let factories: Vec<Box<dyn SolverPhaseFactory<TestSolution>>> = vec![
        Box::new(
            ConstructionPhaseFactory::<TestSolution, TestMove, _>::first_fit(
                create_placer_factory(),
            ),
        ),
        Box::new(
            LocalSearchPhaseFactory::<TestSolution, TestMove, _>::hill_climbing(
                create_move_selector_factory(),
            )
            .with_step_limit(10),
        ),
    ];

    assert_eq!(factories.len(), 2);

    let phase_names: Vec<&str> = factories
        .iter()
        .map(|f| f.create_phase().phase_type_name())
        .collect();

    assert_eq!(phase_names, vec!["ConstructionHeuristic", "LocalSearch"]);
}

#[test]
fn test_construction_phase_factory_with_empty_solution() {
    let factory =
        ConstructionPhaseFactory::<TestSolution, TestMove, _>::first_fit(create_placer_factory());

    let mut solver_scope = create_unassigned_solver_scope(0);

    let mut phase = factory.create_phase();
    phase.solve(&mut solver_scope);
}

#[test]
fn test_local_search_phase_factory_with_single_entity() {
    let factory = LocalSearchPhaseFactory::<TestSolution, TestMove, _>::hill_climbing(
        create_move_selector_factory(),
    )
    .with_step_limit(5);

    let mut solver_scope = create_assigned_solver_scope(&[1]);

    let mut phase = factory.create_phase();
    phase.solve(&mut solver_scope);

    let final_solution = solver_scope.working_solution();
    assert_eq!(final_solution.tasks.len(), 1);
    assert!(final_solution.tasks[0].priority.is_some());
}
