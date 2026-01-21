//! Integration tests for SolverFactory with termination and solving.

use super::*;
use std::time::Duration;

use solverforge_core::score::SimpleScore;
use solverforge_scoring::SimpleScoreDirector;

use crate::scope::SolverScope;

// ============================================================================
// Type Aliases for Score Directors
// ============================================================================

/// Score director type for TestSolution
type TestDirector = SimpleScoreDirector<TestSolution, fn(&TestSolution) -> SimpleScore>;

/// Score director type for EntityTestSolution
type EntityTestDirector =
    SimpleScoreDirector<EntityTestSolution, fn(&EntityTestSolution) -> SimpleScore>;

// ============================================================================
// Test Solution Types
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

fn calculate_entity_score(solution: &EntityTestSolution) -> SimpleScore {
    let sum: i64 = solution.entities.iter().filter_map(|e| e.value).sum();
    let diff = (sum - solution.target_sum).abs();
    SimpleScore::of(-diff)
}

// ============================================================================
// Test with Termination Conditions
// ============================================================================

/// A simple test phase that just sets best solution
#[derive(Debug, Clone)]
struct NoOpPhase;

impl<S: PlanningSolution, D: solverforge_scoring::ScoreDirector<S>> crate::phase::Phase<S, D>
    for NoOpPhase
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D>) {
        solver_scope.update_best_solution();
    }

    fn phase_type_name(&self) -> &'static str {
        "NoOpPhase"
    }
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
            TestEntity {
                id: 2,
                value: Some(2),
            },
        ],
        target_sum: 5,
        score: None,
    };

    let factory =
        solver_factory_builder::<EntityTestSolution, EntityTestDirector, _>(calculate_entity_score)
            .with_time_limit(Duration::from_millis(100))
            .build()
            .expect("Failed to build factory");

    // Verify the factory can calculate scores
    let score = factory.calculate_score(&solution);
    assert_eq!(score, SimpleScore::of(0)); // 1 + 2 + 2 = 5, target is 5, diff = 0
}

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
        ],
        target_sum: 6,
        score: None,
    };

    let factory =
        solver_factory_builder::<EntityTestSolution, EntityTestDirector, _>(calculate_entity_score)
            .with_step_limit(5)
            .build()
            .expect("Failed to build factory");

    // Verify the factory can calculate scores
    let score = factory.calculate_score(&solution);
    assert_eq!(score, SimpleScore::of(-6)); // sum = 0, target = 6, diff = 6
}

#[test]
fn test_solver_factory_with_entity_solution() {
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

    let factory =
        solver_factory_builder::<EntityTestSolution, EntityTestDirector, _>(calculate_entity_score)
            .build()
            .expect("Failed to build factory");

    let score = factory.calculate_score(&solution);
    assert_eq!(score, SimpleScore::of(0));
}

#[test]
fn test_solver_factory_with_phases() {
    let factory = solver_factory_builder::<TestSolution, TestDirector, _>(|s: &TestSolution| {
        SimpleScore::of(-s.value)
    })
    .with_phase(NoOpPhase)
    .with_step_limit(10)
    .build()
    .expect("Failed to build factory");

    let solution = TestSolution {
        value: 5,
        score: None,
    };
    let score = factory.calculate_score(&solution);
    assert_eq!(score, SimpleScore::of(-5));
}

#[test]
fn test_solver_factory_with_multiple_phases() {
    let factory = solver_factory_builder::<TestSolution, TestDirector, _>(|s: &TestSolution| {
        SimpleScore::of(-s.value)
    })
    .with_phase(NoOpPhase)
    .with_phase(NoOpPhase)
    .with_time_limit(Duration::from_secs(1))
    .build()
    .expect("Failed to build factory");

    let solution = TestSolution {
        value: 7,
        score: None,
    };
    let score = factory.calculate_score(&solution);
    assert_eq!(score, SimpleScore::of(-7));
}

#[test]
fn test_construction_and_local_search_types_exist() {
    // Just verify the enum variants exist
    assert_eq!(ConstructionType::default(), ConstructionType::FirstFit);
    assert_eq!(LocalSearchType::default(), LocalSearchType::HillClimbing);

    let _tabu = LocalSearchType::TabuSearch { tabu_size: 10 };
    let _sa = LocalSearchType::SimulatedAnnealing {
        starting_temp: 1.0,
        decay_rate: 0.99,
    };
    let _la = LocalSearchType::LateAcceptance { size: 100 };
    let _bf = ConstructionType::BestFit;
}
