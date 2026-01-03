//! Tests for the event system.

use super::*;
use solverforge_core::score::SimpleScore;

#[derive(Clone, Debug)]
struct TestSolution {
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

#[test]
fn test_event_support_new() {
    let support: SolverEventSupport<TestSolution> = SolverEventSupport::new();

    assert_eq!(support.solver_listener_count(), 0);
    assert_eq!(support.phase_listener_count(), 0);
    assert_eq!(support.step_listener_count(), 0);
    assert!(!support.has_listeners());
}

#[test]
fn test_event_support_add_listeners() {
    let mut support: SolverEventSupport<TestSolution> = SolverEventSupport::new();

    let listener = Arc::new(CountingEventListener::new());
    support.add_solver_listener(listener.clone());
    support.add_phase_listener(listener.clone());
    support.add_step_listener(listener);

    assert_eq!(support.solver_listener_count(), 1);
    assert_eq!(support.phase_listener_count(), 1);
    assert_eq!(support.step_listener_count(), 1);
    assert!(support.has_listeners());
}

#[test]
fn test_event_support_fire_events() {
    let mut support: SolverEventSupport<TestSolution> = SolverEventSupport::new();

    let listener = Arc::new(CountingEventListener::new());
    support.add_solver_listener(listener.clone());
    support.add_phase_listener(listener.clone());
    support.add_step_listener(listener.clone());

    let solution = TestSolution {
        score: Some(SimpleScore::of(-5)),
    };

    support.fire_solving_started(&solution);
    support.fire_best_solution_changed(&solution, &SimpleScore::of(-5));
    support.fire_phase_started(0, "LocalSearch");
    support.fire_step_started(0);
    support.fire_step_ended(0, &SimpleScore::of(-3));
    support.fire_phase_ended(0, "LocalSearch");
    support.fire_solving_ended(&solution, false);

    assert_eq!(listener.solving_started_count(), 1);
    assert_eq!(listener.best_solution_count(), 1);
    assert_eq!(listener.phase_started_count(), 1);
    assert_eq!(listener.step_started_count(), 1);
    assert_eq!(listener.step_ended_count(), 1);
    assert_eq!(listener.phase_ended_count(), 1);
    assert_eq!(listener.solving_ended_count(), 1);
}

#[test]
fn test_event_support_clear_listeners() {
    let mut support: SolverEventSupport<TestSolution> = SolverEventSupport::new();

    let listener = Arc::new(CountingEventListener::new());
    support.add_solver_listener(listener);

    assert!(support.has_listeners());

    support.clear_listeners();

    assert!(!support.has_listeners());
    assert_eq!(support.solver_listener_count(), 0);
}

#[test]
fn test_counting_listener_reset() {
    let listener = CountingEventListener::new();

    listener
        .best_solution_count
        .store(5, std::sync::atomic::Ordering::SeqCst);
    listener
        .phase_started_count
        .store(3, std::sync::atomic::Ordering::SeqCst);

    listener.reset();

    assert_eq!(listener.best_solution_count(), 0);
    assert_eq!(listener.phase_started_count(), 0);
}

#[test]
fn test_logging_listener_creation() {
    let listener = LoggingEventListener::new();
    assert_eq!(listener.prefix, "");

    let listener_with_prefix = LoggingEventListener::with_prefix("[Test] ");
    assert_eq!(listener_with_prefix.prefix, "[Test] ");
}

#[test]
fn test_event_support_debug() {
    let support: SolverEventSupport<TestSolution> = SolverEventSupport::new();
    let debug = format!("{:?}", support);

    assert!(debug.contains("SolverEventSupport"));
    assert!(debug.contains("solver_listeners"));
}

#[test]
fn test_multiple_listeners() {
    let mut support: SolverEventSupport<TestSolution> = SolverEventSupport::new();

    let listener1 = Arc::new(CountingEventListener::new());
    let listener2 = Arc::new(CountingEventListener::new());

    support.add_solver_listener(listener1.clone());
    support.add_solver_listener(listener2.clone());

    let solution = TestSolution {
        score: Some(SimpleScore::of(0)),
    };
    support.fire_best_solution_changed(&solution, &SimpleScore::of(0));

    // Both listeners should have been notified
    assert_eq!(listener1.best_solution_count(), 1);
    assert_eq!(listener2.best_solution_count(), 1);
}
