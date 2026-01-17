//! Tests for SolverFactoryBuilder.

use std::time::Duration;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::SimpleScore;
use solverforge_scoring::SimpleScoreDirector;

use super::SolverFactoryBuilder;

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

/// Type alias for the score director used in tests.
type TestDirector = SimpleScoreDirector<TestSolution, fn(&TestSolution) -> SimpleScore>;

#[test]
fn test_builder_with_time_limit() {
    fn calculator(s: &TestSolution) -> SimpleScore {
        SimpleScore::of(-s.value)
    }
    let factory = SolverFactoryBuilder::<TestSolution, TestDirector, _, _, _>::new(
        calculator as fn(&TestSolution) -> SimpleScore,
    )
    .with_time_limit(Duration::from_secs(30))
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
fn test_builder_with_step_limit() {
    fn calculator(s: &TestSolution) -> SimpleScore {
        SimpleScore::of(-s.value)
    }
    let factory = SolverFactoryBuilder::<TestSolution, TestDirector, _, _, _>::new(
        calculator as fn(&TestSolution) -> SimpleScore,
    )
    .with_step_limit(100)
    .build()
    .expect("Failed to build factory");

    let solution = TestSolution {
        value: 10,
        score: None,
    };
    let score = factory.calculate_score(&solution);
    assert_eq!(score, SimpleScore::of(-10));
}
