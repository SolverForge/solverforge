//! Integration tests for SolverManager with termination and solving.

use super::*;
use std::any::TypeId;
use std::sync::Arc;

use solverforge_core::domain::{EntityDescriptor, SolutionDescriptor, TypedEntityExtractor};
use solverforge_core::score::SimpleScore;
use solverforge_scoring::SimpleScoreDirector;

use crate::heuristic::r#move::ChangeMove;
use crate::heuristic::selector::ChangeMoveSelector;
use crate::termination::StepCountTermination;

// ============================================================================
// Test Solution Types (duplicated for module independence)
// ============================================================================

#[derive(Clone, Debug)]
struct TestSolution {
    value: i64,
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

#[derive(Clone, Debug)]
struct TestEntity {
    value: Option<i64>,
}

#[derive(Clone, Debug)]
struct EntityTestSolution {
    entities: Vec<TestEntity>,
    target_sum: i64,
    score: Option<SimpleScore>,
}

impl PlanningSolution for EntityTestSolution {
    type Score = SimpleScore;
    fn score(&self) -> Option<Self::Score> {
        self.score
    }
    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn get_entities(s: &EntityTestSolution) -> &Vec<TestEntity> {
    &s.entities
}

fn get_entities_mut(s: &mut EntityTestSolution) -> &mut Vec<TestEntity> {
    &mut s.entities
}

fn get_entity_value(s: &EntityTestSolution, idx: usize) -> Option<i64> {
    s.entities.get(idx).and_then(|e| e.value)
}

fn set_entity_value(s: &mut EntityTestSolution, idx: usize, v: Option<i64>) {
    if let Some(entity) = s.entities.get_mut(idx) {
        entity.value = v;
    }
}

fn calculate_entity_score(solution: &EntityTestSolution) -> SimpleScore {
    let sum: i64 = solution.entities.iter().filter_map(|e| e.value).sum();
    let diff = (sum - solution.target_sum).abs();
    SimpleScore::of(-diff)
}

fn create_entity_director(
    solution: EntityTestSolution,
) -> SimpleScoreDirector<EntityTestSolution, impl Fn(&EntityTestSolution) -> SimpleScore> {
    let extractor = Box::new(TypedEntityExtractor::new(
        "TestEntity",
        "entities",
        get_entities,
        get_entities_mut,
    ));
    let entity_desc = EntityDescriptor::new("TestEntity", TypeId::of::<TestEntity>(), "entities")
        .with_extractor(extractor);

    let descriptor =
        SolutionDescriptor::new("EntityTestSolution", TypeId::of::<EntityTestSolution>())
            .with_entity(entity_desc);

    SimpleScoreDirector::with_calculator(solution, descriptor, calculate_entity_score)
}

// ============================================================================
// Test with Termination Conditions
// ============================================================================

type EntityMove = ChangeMove<EntityTestSolution, i64>;

#[test]
fn test_solver_with_step_limit_termination() {
    let solution = EntityTestSolution {
        entities: vec![
            TestEntity { value: Some(0) },
            TestEntity { value: Some(0) },
            TestEntity { value: Some(0) },
        ],
        target_sum: 10,
        score: None,
    };

    let director = create_entity_director(solution);

    let termination_factory: Box<
        dyn Fn() -> Box<dyn crate::termination::Termination<EntityTestSolution>> + Send + Sync,
    > = Box::new(|| Box::new(StepCountTermination::new(10)));

    let phase_factory =
        LocalSearchPhaseFactory::<EntityTestSolution, EntityMove, _>::hill_climbing(|| {
            let values: Vec<i64> = (0..=10).collect();
            Box::new(ChangeMoveSelector::<EntityTestSolution, i64>::simple(
                get_entity_value,
                set_entity_value,
                0,
                "value",
                values,
            ))
        })
        .with_step_limit(20);

    let manager = SolverManager::new(
        calculate_entity_score,
        vec![Box::new(phase_factory)],
        Some(termination_factory),
    );

    let mut solver = manager.create_solver();
    let result = solver.solve_with_director(Box::new(director));

    assert!(result.entities.iter().all(|e| e.value.is_some()));
}


#[test]
fn test_solver_manager_solve_improves_score() {
    let solution = EntityTestSolution {
        entities: vec![
            TestEntity { value: Some(0) },
            TestEntity { value: Some(0) },
            TestEntity { value: Some(0) },
        ],
        target_sum: 6,
        score: None,
    };

    let initial_score = calculate_entity_score(&solution);
    assert_eq!(initial_score, SimpleScore::of(-6));

    let director = create_entity_director(solution);

    let phase_factory =
        LocalSearchPhaseFactory::<EntityTestSolution, EntityMove, _>::hill_climbing(|| {
            let values: Vec<i64> = (0..=10).collect();
            Box::new(ChangeMoveSelector::<EntityTestSolution, i64>::simple(
                get_entity_value,
                set_entity_value,
                0,
                "value",
                values,
            ))
        })
        .with_step_limit(100);

    let manager = SolverManager::new(calculate_entity_score, vec![Box::new(phase_factory)], None);

    let mut solver = manager.create_solver();
    let result = solver.solve_with_director(Box::new(director));

    let final_score = calculate_entity_score(&result);
    assert!(final_score >= initial_score);
}

#[test]
fn test_solver_manager_creates_independent_solvers_for_parallel_solving() {
    let manager = SolverManager::<TestSolution>::builder(|s| SimpleScore::of(-s.value))
        .build()
        .expect("Failed to build manager");

    let solvers: Vec<_> = (0..5).map(|_| manager.create_solver()).collect();

    for solver in &solvers {
        assert!(!solver.is_solving());
    }

    assert_eq!(solvers.len(), 5);
}

/// A simple test phase that just sets best solution
#[derive(Debug, Clone)]
struct NoOpPhase;

impl<S: PlanningSolution> crate::phase::Phase<S> for NoOpPhase {
    fn solve(&mut self, solver_scope: &mut crate::scope::SolverScope<S>) {
        solver_scope.update_best_solution();
    }

    fn phase_type_name(&self) -> &'static str {
        "NoOpPhase"
    }
}

#[test]
fn test_phase_factory_creates_fresh_phases_each_time() {
    let creation_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let count_clone = creation_count.clone();

    let phase_factory = ClosurePhaseFactory::<TestSolution, _>::new(move || {
        count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Box::new(NoOpPhase) as Box<dyn crate::phase::Phase<TestSolution>>
    });

    let manager = SolverManager::new(
        |s: &TestSolution| SimpleScore::of(-s.value),
        vec![Box::new(phase_factory)],
        None,
    );

    let _ = manager.create_solver();
    let _ = manager.create_solver();
    let _ = manager.create_solver();

    assert_eq!(creation_count.load(std::sync::atomic::Ordering::SeqCst), 3);
}

#[test]
fn test_solver_manager_with_entity_solution() {
    let solution = EntityTestSolution {
        entities: vec![
            TestEntity { value: Some(2) },
            TestEntity { value: Some(3) },
        ],
        target_sum: 5,
        score: None,
    };

    let manager = SolverManager::<EntityTestSolution>::builder(calculate_entity_score)
        .build()
        .expect("Failed to build manager");

    let score = manager.calculate_score(&solution);
    assert_eq!(score, SimpleScore::of(0));
}

#[test]
fn test_termination_factory_creates_fresh_termination() {
    let termination_factory: Box<
        dyn Fn() -> Box<dyn crate::termination::Termination<TestSolution>> + Send + Sync,
    > = Box::new(move || Box::new(StepCountTermination::new(10)));

    let manager = SolverManager::new(
        |s: &TestSolution| SimpleScore::of(-s.value),
        vec![],
        Some(termination_factory),
    );

    let _ = manager.create_solver();
    let _ = manager.create_solver();
}

