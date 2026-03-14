// Tests for SolverFactoryBuilder.

use std::time::Duration;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::SoftScore;
use solverforge_scoring::ScoreDirector;

use super::{ConstructionType, LocalSearchType, SolverFactoryBuilder};

#[derive(Clone, Debug)]
struct TestSolution {
    value: i64,
    score: Option<SoftScore>,
}

impl PlanningSolution for TestSolution {
    type Score = SoftScore;
    fn score(&self) -> Option<Self::Score> {
        self.score
    }
    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

// Type alias for the score director used in tests.
type TestDirector = ScoreDirector<TestSolution, ()>;

#[test]
fn test_builder_with_time_limit() {
    fn calculator(s: &TestSolution) -> SoftScore {
        SoftScore::of(-s.value)
    }
    let factory = SolverFactoryBuilder::<TestSolution, TestDirector, _, _, _>::new(
        calculator as fn(&TestSolution) -> SoftScore,
    )
    .with_time_limit(Duration::from_secs(30))
    .build()
    .expect("Failed to build factory");

    let solution = TestSolution {
        value: 5,
        score: None,
    };
    let score = factory.calculate_score(&solution);
    assert_eq!(score, SoftScore::of(-5));
}

#[test]
fn test_builder_with_step_limit() {
    fn calculator(s: &TestSolution) -> SoftScore {
        SoftScore::of(-s.value)
    }
    let factory = SolverFactoryBuilder::<TestSolution, TestDirector, _, _, _>::new(
        calculator as fn(&TestSolution) -> SoftScore,
    )
    .with_step_limit(100)
    .build()
    .expect("Failed to build factory");

    let solution = TestSolution {
        value: 10,
        score: None,
    };
    let score = factory.calculate_score(&solution);
    assert_eq!(score, SoftScore::of(-10));
}

#[test]
fn test_local_search_types() {
    assert_eq!(LocalSearchType::default(), LocalSearchType::HillClimbing);

    let tabu = LocalSearchType::TabuSearch { tabu_size: 10 };
    assert!(matches!(
        tabu,
        LocalSearchType::TabuSearch { tabu_size: 10 }
    ));

    let sa = LocalSearchType::SimulatedAnnealing {
        starting_temp: 1.0,
        decay_rate: 0.99,
    };
    assert!(matches!(
        sa,
        LocalSearchType::SimulatedAnnealing {
            starting_temp: 1.0,
            decay_rate: 0.99
        }
    ));
}

#[test]
fn test_construction_types() {
    assert_eq!(ConstructionType::default(), ConstructionType::FirstFit);
}
