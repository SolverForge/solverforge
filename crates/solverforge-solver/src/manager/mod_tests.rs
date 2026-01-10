//! Tests for SolverManager and related types.

use std::time::Duration;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::SimpleScore;
use solverforge_scoring::SimpleScoreDirector;

use crate::scope::SolverScope;

use super::*;

// ============================================================================
// Test Solution Types
// ============================================================================

/// Simple test solution for basic tests
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

/// Score director type for tests
type TestDirector = SimpleScoreDirector<TestSolution, fn(&TestSolution) -> SimpleScore>;

// ============================================================================
// 1. SolverManager Creation with Builder Pattern
// ============================================================================

#[test]
fn test_solver_manager_builder_creation() {
    let _builder = solver_manager_builder::<TestSolution, TestDirector, _>(|s: &TestSolution| {
        SimpleScore::of(-s.value)
    });
}

#[test]
fn test_solver_manager_builder_builds_successfully() {
    let manager = solver_manager_builder::<TestSolution, TestDirector, _>(|s: &TestSolution| {
        SimpleScore::of(-s.value)
    })
    .build();

    let solution = TestSolution {
        value: 10,
        score: None,
    };
    let score = manager.calculate_score(&solution);
    assert_eq!(score, SimpleScore::of(-10));
}

#[test]
fn test_solver_manager_builder_with_time_limit() {
    let manager = solver_manager_builder::<TestSolution, TestDirector, _>(|s: &TestSolution| {
        SimpleScore::of(-s.value)
    })
    .with_time_limit(Duration::from_secs(30))
    .build();

    let solution = TestSolution {
        value: 5,
        score: None,
    };
    let score = manager.calculate_score(&solution);
    assert_eq!(score, SimpleScore::of(-5));
}

#[test]
fn test_solver_manager_builder_with_step_limit() {
    let manager = solver_manager_builder::<TestSolution, TestDirector, _>(|s: &TestSolution| {
        SimpleScore::of(-s.value)
    })
    .with_step_limit(100)
    .build();

    let solution = TestSolution {
        value: 7,
        score: None,
    };
    let score = manager.calculate_score(&solution);
    assert_eq!(score, SimpleScore::of(-7));
}

// ============================================================================
// 2. Phase Tests
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
fn test_solver_manager_with_phase() {
    let manager = solver_manager_builder::<TestSolution, TestDirector, _>(|s: &TestSolution| {
        SimpleScore::of(-s.value)
    })
    .with_phase(NoOpPhase)
    .build();

    let solution = TestSolution {
        value: 5,
        score: None,
    };
    let score = manager.calculate_score(&solution);
    assert_eq!(score, SimpleScore::of(-5));
}

#[test]
fn test_solver_manager_with_multiple_phases() {
    let manager = solver_manager_builder::<TestSolution, TestDirector, _>(|s: &TestSolution| {
        SimpleScore::of(-s.value)
    })
    .with_phase(NoOpPhase)
    .with_phase(NoOpPhase)
    .build();

    let solution = TestSolution {
        value: 3,
        score: None,
    };
    let score = manager.calculate_score(&solution);
    assert_eq!(score, SimpleScore::of(-3));
}

#[test]
fn test_solver_manager_with_phase_and_step_limit() {
    let manager = solver_manager_builder::<TestSolution, TestDirector, _>(|s: &TestSolution| {
        SimpleScore::of(-s.value)
    })
    .with_phase(NoOpPhase)
    .with_step_limit(50)
    .build();

    let solution = TestSolution {
        value: 8,
        score: None,
    };
    let score = manager.calculate_score(&solution);
    assert_eq!(score, SimpleScore::of(-8));
}

// ============================================================================
// 3. Score Calculator Tests
// ============================================================================

#[test]
fn test_score_calculator_returns_reference() {
    let manager = solver_manager_builder::<TestSolution, TestDirector, _>(|s: &TestSolution| {
        SimpleScore::of(-s.value)
    })
    .build();

    let calculator = manager.score_calculator();

    let solution = TestSolution {
        value: 15,
        score: None,
    };
    let score = calculator(&solution);
    assert_eq!(score, SimpleScore::of(-15));
}

#[test]
fn test_calculate_score_basic() {
    let manager = solver_manager_builder::<TestSolution, TestDirector, _>(|s: &TestSolution| {
        SimpleScore::of(-s.value)
    })
    .build();

    let solution = TestSolution {
        value: 10,
        score: None,
    };
    let score = manager.calculate_score(&solution);
    assert_eq!(score, SimpleScore::of(-10));
}

#[test]
fn test_calculate_score_zero() {
    let manager = solver_manager_builder::<TestSolution, TestDirector, _>(|s: &TestSolution| {
        SimpleScore::of(-s.value)
    })
    .build();

    let solution = TestSolution {
        value: 0,
        score: None,
    };
    let score = manager.calculate_score(&solution);
    assert_eq!(score, SimpleScore::of(0));
}

#[test]
fn test_calculate_score_negative_value() {
    let manager = solver_manager_builder::<TestSolution, TestDirector, _>(|s: &TestSolution| {
        SimpleScore::of(-s.value)
    })
    .build();

    let solution = TestSolution {
        value: -5,
        score: None,
    };
    let score = manager.calculate_score(&solution);
    assert_eq!(score, SimpleScore::of(5)); // -(-5) = 5
}

#[test]
fn test_calculate_score_multiple_solutions() {
    let manager = solver_manager_builder::<TestSolution, TestDirector, _>(|s: &TestSolution| {
        SimpleScore::of(-s.value)
    })
    .build();

    let solutions = [
        TestSolution {
            value: 1,
            score: None,
        },
        TestSolution {
            value: 2,
            score: None,
        },
        TestSolution {
            value: 3,
            score: None,
        },
    ];

    for (i, solution) in solutions.iter().enumerate() {
        let score = manager.calculate_score(solution);
        assert_eq!(score, SimpleScore::of(-((i + 1) as i64)));
    }
}

#[test]
fn test_calculate_score_complex_calculator() {
    let manager = solver_manager_builder::<TestSolution, TestDirector, _>(|s: &TestSolution| {
        SimpleScore::of(-(s.value * s.value))
    })
    .build();

    let solution = TestSolution {
        value: 4,
        score: None,
    };
    let score = manager.calculate_score(&solution);
    assert_eq!(score, SimpleScore::of(-16)); // -(4^2) = -16
}

#[test]
fn test_score_calculator_and_calculate_score_consistent() {
    let manager = solver_manager_builder::<TestSolution, TestDirector, _>(|s: &TestSolution| {
        SimpleScore::of(-s.value * 2)
    })
    .build();

    let solution = TestSolution {
        value: 7,
        score: None,
    };

    let calculator = manager.score_calculator();
    let score_via_calculator = calculator(&solution);
    let score_via_method = manager.calculate_score(&solution);

    assert_eq!(score_via_calculator, score_via_method);
    assert_eq!(score_via_method, SimpleScore::of(-14));
}
