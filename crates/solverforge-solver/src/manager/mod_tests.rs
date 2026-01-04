//! Tests for SolverManager and related types.

use super::*;
use std::time::Duration;

use solverforge_core::score::SimpleScore;

use crate::termination::StepCountTermination;

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

// ============================================================================
// 1. SolverManager Creation with Builder Pattern
// ============================================================================

#[test]
fn test_solver_manager_builder_creation() {
    let _builder = SolverManager::<TestSolution>::builder(|s| SimpleScore::of(-s.value));
    // Builder created successfully
}

#[test]
fn test_solver_manager_builder_builds_successfully() {
    let manager = SolverManager::<TestSolution>::builder(|s| SimpleScore::of(-s.value))
        .build()
        .expect("Failed to build SolverManager");

    // Verify manager is usable
    let solution = TestSolution {
        value: 10,
        score: None,
    };
    let score = manager.calculate_score(&solution);
    assert_eq!(score, SimpleScore::of(-10));
}

#[test]
fn test_solver_manager_builder_with_time_limit() {
    let manager = SolverManager::<TestSolution>::builder(|s| SimpleScore::of(-s.value))
        .with_time_limit(Duration::from_secs(30))
        .build()
        .expect("Failed to build SolverManager with time limit");

    let solution = TestSolution {
        value: 5,
        score: None,
    };
    let score = manager.calculate_score(&solution);
    assert_eq!(score, SimpleScore::of(-5));
}

#[test]
fn test_solver_manager_builder_with_step_limit() {
    let manager = SolverManager::<TestSolution>::builder(|s| SimpleScore::of(-s.value))
        .with_step_limit(100)
        .build()
        .expect("Failed to build SolverManager with step limit");

    let solution = TestSolution {
        value: 7,
        score: None,
    };
    let score = manager.calculate_score(&solution);
    assert_eq!(score, SimpleScore::of(-7));
}

#[test]
fn test_solver_manager_builder_with_combined_limits() {
    let manager = SolverManager::<TestSolution>::builder(|s| SimpleScore::of(-s.value))
        .with_time_limit(Duration::from_secs(60))
        .with_step_limit(500)
        .build()
        .expect("Failed to build SolverManager with combined limits");

    let solution = TestSolution {
        value: 3,
        score: None,
    };
    let score = manager.calculate_score(&solution);
    assert_eq!(score, SimpleScore::of(-3));
}

#[test]
fn test_solver_manager_builder_with_construction_heuristic() {
    let manager = SolverManager::<TestSolution>::builder(|s| SimpleScore::of(-s.value))
        .with_construction_heuristic()
        .build()
        .expect("Failed to build SolverManager with construction heuristic");

    let solution = TestSolution {
        value: 2,
        score: None,
    };
    let score = manager.calculate_score(&solution);
    assert_eq!(score, SimpleScore::of(-2));
}

#[test]
fn test_solver_manager_builder_with_local_search() {
    let manager = SolverManager::<TestSolution>::builder(|s| SimpleScore::of(-s.value))
        .with_local_search(LocalSearchType::HillClimbing)
        .build()
        .expect("Failed to build SolverManager with local search");

    let solution = TestSolution {
        value: 4,
        score: None,
    };
    let score = manager.calculate_score(&solution);
    assert_eq!(score, SimpleScore::of(-4));
}

#[test]
fn test_solver_manager_builder_chained_configuration() {
    let manager = SolverManager::<TestSolution>::builder(|s| SimpleScore::of(-s.value))
        .with_construction_heuristic()
        .with_local_search(LocalSearchType::HillClimbing)
        .with_time_limit(Duration::from_secs(30))
        .with_step_limit(1000)
        .build()
        .expect("Failed to build SolverManager with chained configuration");

    let solution = TestSolution {
        value: 8,
        score: None,
    };
    let score = manager.calculate_score(&solution);
    assert_eq!(score, SimpleScore::of(-8));
}

// ============================================================================
// 2. Adding Phase Factories to SolverManager
// ============================================================================

/// A simple test phase that just sets best solution
#[derive(Debug, Clone)]
struct NoOpPhase;

impl<S: PlanningSolution> crate::phase::Phase<S> for NoOpPhase {
    fn solve(&mut self, solver_scope: &mut crate::scope::SolverScope<S>) {
        // Just update best solution with current working solution
        solver_scope.update_best_solution();
    }

    fn phase_type_name(&self) -> &'static str {
        "NoOpPhase"
    }
}

#[test]
fn test_cloneable_phase_factory() {
    let factory = CloneablePhaseFactory::new(NoOpPhase);
    let phase: Box<dyn crate::phase::Phase<TestSolution>> = factory.create_phase();
    assert_eq!(phase.phase_type_name(), "NoOpPhase");
}

#[test]
fn test_cloneable_phase_factory_creates_fresh_instances() {
    let factory = CloneablePhaseFactory::new(NoOpPhase);

    let phase1: Box<dyn crate::phase::Phase<TestSolution>> = factory.create_phase();
    let phase2: Box<dyn crate::phase::Phase<TestSolution>> = factory.create_phase();

    // Both should be independent instances
    assert_eq!(phase1.phase_type_name(), "NoOpPhase");
    assert_eq!(phase2.phase_type_name(), "NoOpPhase");
}

#[test]
fn test_closure_phase_factory() {
    let factory = ClosurePhaseFactory::<TestSolution, _>::new(|| {
        Box::new(NoOpPhase) as Box<dyn crate::phase::Phase<TestSolution>>
    });

    let phase = factory.create_phase();
    assert_eq!(phase.phase_type_name(), "NoOpPhase");
}

#[test]
fn test_closure_phase_factory_creates_fresh_instances() {
    let call_count = std::sync::atomic::AtomicUsize::new(0);
    let factory = ClosurePhaseFactory::<TestSolution, _>::new(|| {
        call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Box::new(NoOpPhase) as Box<dyn crate::phase::Phase<TestSolution>>
    });

    let _ = factory.create_phase();
    let _ = factory.create_phase();
    let _ = factory.create_phase();

    assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 3);
}

#[test]
fn test_solver_manager_with_phase_factories() {
    let phase_factories: Vec<Box<dyn SolverPhaseFactory<TestSolution>>> = vec![
        Box::new(CloneablePhaseFactory::new(NoOpPhase)),
        Box::new(ClosurePhaseFactory::new(|| {
            Box::new(NoOpPhase) as Box<dyn crate::phase::Phase<TestSolution>>
        })),
    ];

    let manager = SolverManager::new(
        |s: &TestSolution| SimpleScore::of(-s.value),
        phase_factories,
        None,
    );

    // Create solver - should have 2 phases
    let solver = manager.create_solver();
    assert!(!solver.is_solving());
}

// ============================================================================
// 3. create_solver() Returns a Configured Solver
// ============================================================================

#[test]
fn test_create_solver_returns_valid_solver() {
    let manager = SolverManager::<TestSolution>::builder(|s| SimpleScore::of(-s.value))
        .build()
        .expect("Failed to build SolverManager");

    let solver = manager.create_solver();
    assert!(!solver.is_solving());
}

#[test]
fn test_create_solver_multiple_times_creates_independent_solvers() {
    let manager = SolverManager::<TestSolution>::builder(|s| SimpleScore::of(-s.value))
        .with_step_limit(10)
        .build()
        .expect("Failed to build SolverManager");

    let solver1 = manager.create_solver();
    let solver2 = manager.create_solver();

    // Both solvers should be independent and not solving
    assert!(!solver1.is_solving());
    assert!(!solver2.is_solving());
}

#[test]
fn test_create_solver_with_phase_factories() {
    let phase_factory = CloneablePhaseFactory::new(NoOpPhase);
    let phase_factories: Vec<Box<dyn SolverPhaseFactory<TestSolution>>> =
        vec![Box::new(phase_factory)];

    let manager = SolverManager::new(
        |s: &TestSolution| SimpleScore::of(-s.value),
        phase_factories,
        None,
    );

    let solver = manager.create_solver();
    assert!(!solver.is_solving());
}

#[test]
fn test_create_solver_with_termination() {
    let termination_factory: Box<
        dyn Fn() -> Box<dyn crate::termination::Termination<TestSolution>> + Send + Sync,
    > = Box::new(|| Box::new(StepCountTermination::new(50)));

    let manager = SolverManager::new(
        |s: &TestSolution| SimpleScore::of(-s.value),
        vec![],
        Some(termination_factory),
    );

    let solver = manager.create_solver();
    assert!(!solver.is_solving());
}

// ============================================================================
// 4. score_calculator() and calculate_score() Methods
// ============================================================================

#[test]
fn test_score_calculator_returns_arc() {
    let manager = SolverManager::<TestSolution>::builder(|s| SimpleScore::of(-s.value))
        .build()
        .expect("Failed to build SolverManager");

    let calculator = manager.score_calculator();

    // Test the calculator directly
    let solution = TestSolution {
        value: 15,
        score: None,
    };
    let score = calculator(&solution);
    assert_eq!(score, SimpleScore::of(-15));
}

#[test]
fn test_calculate_score_basic() {
    let manager = SolverManager::<TestSolution>::builder(|s| SimpleScore::of(-s.value))
        .build()
        .expect("Failed to build SolverManager");

    let solution = TestSolution {
        value: 10,
        score: None,
    };
    let score = manager.calculate_score(&solution);
    assert_eq!(score, SimpleScore::of(-10));
}

#[test]
fn test_calculate_score_zero() {
    let manager = SolverManager::<TestSolution>::builder(|s| SimpleScore::of(-s.value))
        .build()
        .expect("Failed to build SolverManager");

    let solution = TestSolution {
        value: 0,
        score: None,
    };
    let score = manager.calculate_score(&solution);
    assert_eq!(score, SimpleScore::of(0));
}

#[test]
fn test_calculate_score_negative_value() {
    let manager = SolverManager::<TestSolution>::builder(|s| SimpleScore::of(-s.value))
        .build()
        .expect("Failed to build SolverManager");

    let solution = TestSolution {
        value: -5,
        score: None,
    };
    let score = manager.calculate_score(&solution);
    assert_eq!(score, SimpleScore::of(5)); // -(-5) = 5
}

#[test]
fn test_calculate_score_multiple_solutions() {
    let manager = SolverManager::<TestSolution>::builder(|s| SimpleScore::of(-s.value))
        .build()
        .expect("Failed to build SolverManager");

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
    // More complex score calculator: sum of squares
    let manager = SolverManager::<TestSolution>::builder(|s| SimpleScore::of(-(s.value * s.value)))
        .build()
        .expect("Failed to build SolverManager");

    let solution = TestSolution {
        value: 4,
        score: None,
    };
    let score = manager.calculate_score(&solution);
    assert_eq!(score, SimpleScore::of(-16)); // -(4^2) = -16
}

#[test]
fn test_score_calculator_and_calculate_score_consistent() {
    let manager = SolverManager::<TestSolution>::builder(|s| SimpleScore::of(-s.value * 2))
        .build()
        .expect("Failed to build SolverManager");

    let solution = TestSolution {
        value: 7,
        score: None,
    };

    // Both methods should return the same result
    let calculator = manager.score_calculator();
    let score_via_calculator = calculator(&solution);
    let score_via_method = manager.calculate_score(&solution);

    assert_eq!(score_via_calculator, score_via_method);
    assert_eq!(score_via_method, SimpleScore::of(-14));
}
