//! Integration tests for SolverManager with termination and solving.

use super::*;
use std::any::TypeId;
use std::sync::Arc;
use std::time::Duration;

use solverforge_core::domain::{EntityDescriptor, SolutionDescriptor, TypedEntityExtractor};
use solverforge_core::score::SimpleScore;
use solverforge_scoring::SimpleScoreDirector;

use crate::heuristic::r#move::ChangeMove;
use crate::heuristic::selector::ChangeMoveSelector;
use crate::termination::StepCountTermination;

// ============================================================================
// Type Aliases for Score Directors
// ============================================================================

/// Score director type for TestSolution
type TestDirector = SimpleScoreDirector<TestSolution, fn(&TestSolution) -> SimpleScore>;

/// Score director type for EntityTestSolution
type EntityTestDirector =
    SimpleScoreDirector<EntityTestSolution, fn(&EntityTestSolution) -> SimpleScore>;

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
    #[allow(dead_code)]
    id: i64,
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
            TestEntity {
                id: 0,
                value: Some(0),
            },
            TestEntity {
                id: 1,
                value: Some(0),
            },
            TestEntity {
                id: 2,
                value: Some(0),
            },
        ],
        target_sum: 10,
        score: None,
    };

    let _director = create_entity_director(solution);

    let termination_factory: Box<
        dyn Fn() -> Box<dyn crate::termination::Termination<EntityTestSolution, EntityTestDirector>>
            + Send
            + Sync,
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

    let manager: SolverManager<EntityTestSolution, EntityTestDirector, _> = SolverManager::new(
        calculate_entity_score,
        vec![Box::new(phase_factory)],
        Some(termination_factory),
    );

    // Verify that SolverManager was created successfully by checking score calculation
    let phases = manager.create_phases();
    assert_eq!(phases.len(), 1);

    // Verify termination was created
    let termination = manager.create_termination();
    assert!(termination.is_some());
}

#[test]
fn test_solver_with_time_limit_termination() {
    let solution = EntityTestSolution {
        entities: vec![
            TestEntity {
                id: 0,
                value: Some(1),
            },
            TestEntity {
                id: 1,
                value: Some(2),
            },
        ],
        target_sum: 5,
        score: None,
    };

    let _director = create_entity_director(solution.clone());

    let manager =
        SolverManager::<EntityTestSolution, EntityTestDirector>::builder(calculate_entity_score)
            .with_time_limit(Duration::from_millis(100))
            .build()
            .expect("Failed to build manager");

    // Verify the manager can calculate scores
    let score = manager.calculate_score(&solution);
    assert_eq!(score, SimpleScore::of(0)); // 1 + 2 + 2 = 5, target is 5, diff = 0
}

#[test]
fn test_solver_with_combined_termination() {
    let solution = EntityTestSolution {
        entities: vec![
            TestEntity {
                id: 0,
                value: Some(0),
            },
            TestEntity {
                id: 1,
                value: Some(0),
            },
        ],
        target_sum: 6,
        score: None,
    };

    let _director = create_entity_director(solution.clone());

    let manager =
        SolverManager::<EntityTestSolution, EntityTestDirector>::builder(calculate_entity_score)
            .with_time_limit(Duration::from_secs(1))
            .with_step_limit(5)
            .build()
            .expect("Failed to build manager");

    // Verify the manager can calculate scores
    let score = manager.calculate_score(&solution);
    assert_eq!(score, SimpleScore::of(-6)); // sum = 0, target = 6, diff = 6
}

#[test]
fn test_solver_manager_creates_phases_from_factories() {
    let solution = EntityTestSolution {
        entities: vec![
            TestEntity {
                id: 0,
                value: Some(0),
            },
            TestEntity {
                id: 1,
                value: Some(0),
            },
            TestEntity {
                id: 2,
                value: Some(0),
            },
        ],
        target_sum: 6,
        score: None,
    };

    let initial_score = calculate_entity_score(&solution);
    assert_eq!(initial_score, SimpleScore::of(-6));

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

    let manager: SolverManager<EntityTestSolution, EntityTestDirector, _> =
        SolverManager::new(calculate_entity_score, vec![Box::new(phase_factory)], None);

    // Verify that the manager creates phases correctly
    let phases = manager.create_phases();
    assert_eq!(phases.len(), 1);

    // Verify score calculation works
    let score = manager.calculate_score(&solution);
    assert_eq!(score, SimpleScore::of(-6));
}

#[test]
fn test_solver_manager_creates_independent_phases_for_parallel_solving() {
    let manager = SolverManager::<TestSolution, TestDirector>::builder(|s| SimpleScore::of(-s.value))
        .with_step_limit(10)
        .build()
        .expect("Failed to build manager");

    // Verify multiple phase sets can be created
    let phases1 = manager.create_phases();
    let phases2 = manager.create_phases();
    let phases3 = manager.create_phases();

    // Empty phases since we didn't add any phase factories
    assert_eq!(phases1.len(), 0);
    assert_eq!(phases2.len(), 0);
    assert_eq!(phases3.len(), 0);

    // Verify termination can be created for each
    let term1 = manager.create_termination();
    let term2 = manager.create_termination();
    assert!(term1.is_some());
    assert!(term2.is_some());
}

/// A simple test phase that just sets best solution
#[derive(Debug, Clone)]
struct NoOpPhase;

impl<S: PlanningSolution, D: solverforge_scoring::ScoreDirector<S>> crate::phase::Phase<S, D>
    for NoOpPhase
{
    fn solve(&mut self, solver_scope: &mut crate::scope::SolverScope<S, D>) {
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

    let phase_factory = ClosurePhaseFactory::<TestSolution, TestDirector, _>::new(move || {
        count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Box::new(NoOpPhase) as Box<dyn crate::phase::Phase<TestSolution, TestDirector>>
    });

    let manager: SolverManager<TestSolution, TestDirector, _> = SolverManager::new(
        |s: &TestSolution| SimpleScore::of(-s.value),
        vec![Box::new(phase_factory)],
        None,
    );

    // Each call to create_phases() should invoke the factory
    let _ = manager.create_phases();
    let _ = manager.create_phases();
    let _ = manager.create_phases();

    assert_eq!(creation_count.load(std::sync::atomic::Ordering::SeqCst), 3);
}

#[test]
fn test_solver_manager_with_entity_solution() {
    let solution = EntityTestSolution {
        entities: vec![
            TestEntity {
                id: 0,
                value: Some(2),
            },
            TestEntity {
                id: 1,
                value: Some(3),
            },
        ],
        target_sum: 5,
        score: None,
    };

    let manager =
        SolverManager::<EntityTestSolution, EntityTestDirector>::builder(calculate_entity_score)
            .build()
            .expect("Failed to build manager");

    let score = manager.calculate_score(&solution);
    assert_eq!(score, SimpleScore::of(0));
}

#[test]
fn test_termination_factory_creates_fresh_termination() {
    let termination_factory: Box<
        dyn Fn() -> Box<dyn crate::termination::Termination<TestSolution, TestDirector>>
            + Send
            + Sync,
    > = Box::new(move || Box::new(StepCountTermination::new(10)));

    let manager: SolverManager<TestSolution, TestDirector, _> = SolverManager::new(
        |s: &TestSolution| SimpleScore::of(-s.value),
        vec![],
        Some(termination_factory),
    );

    // Each call to create_termination should invoke the factory
    let term1 = manager.create_termination();
    let term2 = manager.create_termination();
    assert!(term1.is_some());
    assert!(term2.is_some());
}

#[test]
fn test_solver_manager_builder_with_local_search_variants() {
    let hill_climbing =
        SolverManager::<TestSolution, TestDirector>::builder(|s| SimpleScore::of(-s.value))
            .with_local_search(LocalSearchType::HillClimbing)
            .build()
            .expect("Failed to build with hill climbing");

    let tabu_search =
        SolverManager::<TestSolution, TestDirector>::builder(|s| SimpleScore::of(-s.value))
            .with_local_search(LocalSearchType::TabuSearch { tabu_size: 10 })
            .build()
            .expect("Failed to build with tabu search");

    let simulated_annealing =
        SolverManager::<TestSolution, TestDirector>::builder(|s| SimpleScore::of(-s.value))
            .with_local_search(LocalSearchType::SimulatedAnnealing {
                starting_temp: 1.0,
                decay_rate: 0.99,
            })
            .build()
            .expect("Failed to build with simulated annealing");

    let late_acceptance =
        SolverManager::<TestSolution, TestDirector>::builder(|s| SimpleScore::of(-s.value))
            .with_local_search(LocalSearchType::LateAcceptance { size: 100 })
            .build()
            .expect("Failed to build with late acceptance");

    let solution = TestSolution {
        value: 5,
        score: None,
    };
    assert_eq!(
        hill_climbing.calculate_score(&solution),
        SimpleScore::of(-5)
    );
    assert_eq!(tabu_search.calculate_score(&solution), SimpleScore::of(-5));
    assert_eq!(
        simulated_annealing.calculate_score(&solution),
        SimpleScore::of(-5)
    );
    assert_eq!(
        late_acceptance.calculate_score(&solution),
        SimpleScore::of(-5)
    );
}

#[test]
fn test_solver_manager_builder_with_construction_types() {
    let first_fit =
        SolverManager::<TestSolution, TestDirector>::builder(|s| SimpleScore::of(-s.value))
            .with_construction_heuristic_type(ConstructionType::FirstFit)
            .build()
            .expect("Failed to build with first fit");

    let best_fit =
        SolverManager::<TestSolution, TestDirector>::builder(|s| SimpleScore::of(-s.value))
            .with_construction_heuristic_type(ConstructionType::BestFit)
            .build()
            .expect("Failed to build with best fit");

    let solution = TestSolution {
        value: 3,
        score: None,
    };
    assert_eq!(first_fit.calculate_score(&solution), SimpleScore::of(-3));
    assert_eq!(best_fit.calculate_score(&solution), SimpleScore::of(-3));
}

#[test]
fn test_solver_manager_with_local_search_steps() {
    let manager =
        SolverManager::<TestSolution, TestDirector>::builder(|s| SimpleScore::of(-s.value))
            .with_local_search_steps(LocalSearchType::HillClimbing, 50)
            .build()
            .expect("Failed to build with local search steps");

    let solution = TestSolution {
        value: 6,
        score: None,
    };
    assert_eq!(manager.calculate_score(&solution), SimpleScore::of(-6));
}
