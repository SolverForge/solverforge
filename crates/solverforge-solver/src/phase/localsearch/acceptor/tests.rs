//! Tests for acceptors.

use super::*;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::SimpleScore;

#[derive(Clone, Debug)]
struct DummySolution {
    score: Option<SimpleScore>,
}

impl PlanningSolution for DummySolution {
    type Score = SimpleScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

#[test]
fn test_hill_climbing_accepts_improving() {
    let acceptor = HillClimbingAcceptor::<DummySolution>::new();
    assert!(acceptor.is_accepted(&SimpleScore::of(-10), &SimpleScore::of(-5)));
}

#[test]
fn test_hill_climbing_rejects_worsening() {
    let acceptor = HillClimbingAcceptor::<DummySolution>::new();
    assert!(!acceptor.is_accepted(&SimpleScore::of(-5), &SimpleScore::of(-10)));
}

#[test]
fn test_hill_climbing_accepts_equal() {
    // Hill climbing accepts equal scores for plateau exploration
    let acceptor = HillClimbingAcceptor::<DummySolution>::new();
    assert!(acceptor.is_accepted(&SimpleScore::of(-5), &SimpleScore::of(-5)));
}

#[test]
fn test_simulated_annealing_accepts_improving() {
    let acceptor = SimulatedAnnealingAcceptor::<DummySolution>::new(1.0);
    assert!(acceptor.is_accepted(&SimpleScore::of(-10), &SimpleScore::of(-5)));
}

#[test]
fn test_simulated_annealing_accepts_equal() {
    let acceptor = SimulatedAnnealingAcceptor::<DummySolution>::new(1.0);
    assert!(acceptor.is_accepted(&SimpleScore::of(-10), &SimpleScore::of(-10)));
}

#[test]
fn test_simulated_annealing_probabilistic_worsening() {
    // With high temperature, worsening moves should sometimes be accepted
    let acceptor = SimulatedAnnealingAcceptor::<DummySolution>::new(100.0);

    let mut accepted = 0;
    for _ in 0..1000 {
        if acceptor.is_accepted(&SimpleScore::of(100), &SimpleScore::of(90)) {
            accepted += 1;
        }
    }

    // With T=100 and delta=10, p = exp(-10/100) ≈ 0.905
    // Should accept roughly 90% of the time
    assert!(
        accepted > 800 && accepted < 970,
        "Expected ~90% acceptance rate, got {}/1000",
        accepted
    );
}

#[test]
fn test_simulated_annealing_rejects_at_zero_temp() {
    let mut acceptor = SimulatedAnnealingAcceptor::<DummySolution>::new(100.0);

    // Set time gradient to 1.0 (end of phase) -> temperature = 0
    acceptor.set_time_gradient(1.0);
    assert_eq!(acceptor.current_temperature(), 0.0);

    // Worsening moves should be rejected
    assert!(!acceptor.is_accepted(&SimpleScore::of(100), &SimpleScore::of(90)));

    // Improving/equal moves should still be accepted
    assert!(acceptor.is_accepted(&SimpleScore::of(100), &SimpleScore::of(110)));
    assert!(acceptor.is_accepted(&SimpleScore::of(100), &SimpleScore::of(100)));
}

#[test]
fn test_simulated_annealing_time_gradient_cooling() {
    let mut acceptor = SimulatedAnnealingAcceptor::<DummySolution>::new(100.0);

    // At start: T = 100 * (1 - 0) = 100
    acceptor.set_time_gradient(0.0);
    assert!((acceptor.current_temperature() - 100.0).abs() < 1e-10);

    // At 50%: T = 100 * (1 - 0.5) = 50
    acceptor.set_time_gradient(0.5);
    assert!((acceptor.current_temperature() - 50.0).abs() < 1e-10);

    // At end: T = 100 * (1 - 1) = 0
    acceptor.set_time_gradient(1.0);
    assert!((acceptor.current_temperature() - 0.0).abs() < 1e-10);
}

#[test]
fn test_late_acceptance_history() {
    let mut acceptor = LateAcceptanceAcceptor::<DummySolution>::new(5);
    acceptor.phase_started(&SimpleScore::of(-10));
    assert!(acceptor.is_accepted(&SimpleScore::of(-10), &SimpleScore::of(-5)));
    assert!(acceptor.is_accepted(&SimpleScore::of(-10), &SimpleScore::of(-10)));
    assert!(!acceptor.is_accepted(&SimpleScore::of(-10), &SimpleScore::of(-15)));
}

#[test]
fn test_tabu_search_accepts_improving() {
    let mut acceptor = TabuSearchAcceptor::<DummySolution>::new(5);
    acceptor.phase_started(&SimpleScore::of(-10));
    assert!(acceptor.is_accepted(&SimpleScore::of(-10), &SimpleScore::of(-5)));
}

#[test]
fn test_tabu_search_accepts_equal() {
    let mut acceptor = TabuSearchAcceptor::<DummySolution>::new(5);
    acceptor.phase_started(&SimpleScore::of(-10));
    assert!(acceptor.is_accepted(&SimpleScore::of(-10), &SimpleScore::of(-10)));
}

#[test]
fn test_tabu_search_rejects_tabu_entity() {
    let mut acceptor = TabuSearchAcceptor::<DummySolution>::new(5);
    acceptor.phase_started(&SimpleScore::of(-10));

    // Record entity 1 and end step
    acceptor.step_started();
    acceptor.record_move_entities(&[1]);
    acceptor.step_ended(&SimpleScore::of(-5));

    // New move with same entity should be rejected
    acceptor.step_started();
    acceptor.record_move_entities(&[1]);
    assert!(!acceptor.is_accepted(&SimpleScore::of(-5), &SimpleScore::of(-5)));

    // Different entity should be accepted
    acceptor.step_started();
    acceptor.record_move_entities(&[99]);
    assert!(acceptor.is_accepted(&SimpleScore::of(-5), &SimpleScore::of(-5)));
}

#[test]
fn test_tabu_search_aspiration_accepts_new_best() {
    let mut acceptor = TabuSearchAcceptor::<DummySolution>::new(5);
    acceptor.phase_started(&SimpleScore::of(-10));

    // Record entity and end step
    acceptor.step_started();
    acceptor.record_move_entities(&[1]);
    acceptor.step_ended(&SimpleScore::of(-5));

    // Tabu entity with better-than-best score should be accepted
    acceptor.step_started();
    acceptor.record_move_entities(&[1]);
    assert!(acceptor.is_accepted(&SimpleScore::of(-5), &SimpleScore::of(-2)));
}

#[test]
fn test_tabu_search_without_aspiration() {
    let mut acceptor = TabuSearchAcceptor::<DummySolution>::without_aspiration(5);
    acceptor.phase_started(&SimpleScore::of(-10));

    // Record entity and end step
    acceptor.step_started();
    acceptor.record_move_entities(&[1]);
    acceptor.step_ended(&SimpleScore::of(-5));

    // Tabu entity should be rejected even if better than best
    acceptor.step_started();
    acceptor.record_move_entities(&[1]);
    assert!(!acceptor.is_accepted(&SimpleScore::of(-5), &SimpleScore::of(-2)));
}

#[test]
fn test_tabu_search_entity_eviction() {
    // Test with tabu_size=3 and fading_size=3 (default)
    // Total capacity = 6, but hard tabu period = 3 steps
    let mut acceptor = TabuSearchAcceptor::<DummySolution>::new(3);
    acceptor.phase_started(&SimpleScore::of(-10));

    // Add entities 1, 2, 3 to tabu
    acceptor.step_started();
    acceptor.record_move_entities(&[1]);
    acceptor.step_ended(&SimpleScore::of(-9));

    acceptor.step_started();
    acceptor.record_move_entities(&[2]);
    acceptor.step_ended(&SimpleScore::of(-8));

    acceptor.step_started();
    acceptor.record_move_entities(&[3]);
    acceptor.step_ended(&SimpleScore::of(-7));

    // All should be tabu at this point (ages: 1=3, 2=2, 3=1)
    assert!(acceptor.is_entity_tabu(1));
    assert!(acceptor.is_entity_tabu(2));
    assert!(acceptor.is_entity_tabu(3));

    // Add entity 4
    acceptor.step_started();
    acceptor.record_move_entities(&[4]);
    acceptor.step_ended(&SimpleScore::of(-6));

    // Entity 1 should now be in fading period (age=4 > tabu_size=3)
    assert!(!acceptor.is_entity_tabu(1));
    assert!(acceptor.is_entity_fading(1));
    assert!(acceptor.is_entity_tabu(2));
    assert!(acceptor.is_entity_tabu(3));
    assert!(acceptor.is_entity_tabu(4));
}

#[test]
fn test_tabu_search_phase_ended_clears_tabu() {
    let mut acceptor = TabuSearchAcceptor::<DummySolution>::new(5);
    acceptor.phase_started(&SimpleScore::of(-10));

    acceptor.step_started();
    acceptor.record_move_entities(&[1]);
    acceptor.step_ended(&SimpleScore::of(-5));

    assert!(acceptor.is_entity_tabu(1));

    acceptor.phase_ended();
    acceptor.phase_started(&SimpleScore::of(-10));

    assert!(!acceptor.is_entity_tabu(1));
}

#[test]
fn test_tabu_search_fading_probabilistic() {
    // Create acceptor with fading enabled
    let mut acceptor = TabuSearchAcceptor::<DummySolution>::with_fading(3, 10);
    acceptor.phase_started(&SimpleScore::of(-10));

    // Add entity 1 to tabu
    acceptor.step_started();
    acceptor.record_move_entities(&[1]);
    acceptor.step_ended(&SimpleScore::of(-5));

    // Advance 4 more steps to put entity 1 in fading period (age=5)
    for _ in 0..4 {
        acceptor.step_started();
        acceptor.record_move_entities(&[99]); // Different entity
        acceptor.step_ended(&SimpleScore::of(-5));
    }

    // Entity 1 should be in fading period now
    assert!(!acceptor.is_entity_tabu(1));
    assert!(acceptor.is_entity_fading(1));

    // With fading, some attempts should be accepted (probabilistically)
    let mut accepted = 0;
    for _ in 0..100 {
        acceptor.step_started();
        acceptor.record_move_entities(&[1]);
        if acceptor.is_accepted(&SimpleScore::of(-5), &SimpleScore::of(-5)) {
            accepted += 1;
        }
    }

    // Should accept some but not all (probabilistic)
    assert!(accepted > 0, "Fading should allow some acceptances");
    assert!(accepted < 100, "Fading should not accept all");
}

#[test]
fn test_entity_tabu_accepts_improving() {
    let mut acceptor = EntityTabuAcceptor::<DummySolution>::new(5);
    acceptor.phase_started(&SimpleScore::of(-10));
    assert!(acceptor.is_accepted(&SimpleScore::of(-10), &SimpleScore::of(-5)));
}

#[test]
fn test_entity_tabu_accepts_equal() {
    let mut acceptor = EntityTabuAcceptor::<DummySolution>::new(5);
    acceptor.phase_started(&SimpleScore::of(-10));
    assert!(acceptor.is_accepted(&SimpleScore::of(-10), &SimpleScore::of(-10)));
}

#[test]
fn test_entity_tabu_rejects_worsening() {
    let mut acceptor = EntityTabuAcceptor::<DummySolution>::new(5);
    acceptor.phase_started(&SimpleScore::of(-10));
    assert!(!acceptor.is_accepted(&SimpleScore::of(-5), &SimpleScore::of(-10)));
}

#[test]
fn test_entity_tabu_tracking() {
    let mut acceptor = EntityTabuAcceptor::<DummySolution>::new(3);
    acceptor.phase_started(&SimpleScore::of(-10));

    acceptor.step_started();
    acceptor.record_entity_move(1);
    acceptor.record_entity_move(2);
    acceptor.step_ended(&SimpleScore::of(-5));

    assert!(acceptor.is_entity_tabu(1));
    assert!(acceptor.is_entity_tabu(2));
    assert!(!acceptor.is_entity_tabu(3));
}

#[test]
fn test_entity_tabu_eviction() {
    let mut acceptor = EntityTabuAcceptor::<DummySolution>::new(3);
    acceptor.phase_started(&SimpleScore::of(-10));

    acceptor.step_started();
    acceptor.record_entity_move(1);
    acceptor.step_ended(&SimpleScore::of(-9));

    acceptor.step_started();
    acceptor.record_entity_move(2);
    acceptor.step_ended(&SimpleScore::of(-8));

    acceptor.step_started();
    acceptor.record_entity_move(3);
    acceptor.step_ended(&SimpleScore::of(-7));

    assert!(acceptor.is_entity_tabu(1));
    assert!(acceptor.is_entity_tabu(2));
    assert!(acceptor.is_entity_tabu(3));

    acceptor.step_started();
    acceptor.record_entity_move(4);
    acceptor.step_ended(&SimpleScore::of(-6));

    assert!(!acceptor.is_entity_tabu(1));
    assert!(acceptor.is_entity_tabu(2));
    assert!(acceptor.is_entity_tabu(3));
    assert!(acceptor.is_entity_tabu(4));
}

#[test]
fn test_entity_tabu_phase_ended_clears() {
    let mut acceptor = EntityTabuAcceptor::<DummySolution>::new(5);
    acceptor.phase_started(&SimpleScore::of(-10));

    acceptor.step_started();
    acceptor.record_entity_move(1);
    acceptor.step_ended(&SimpleScore::of(-5));

    assert!(acceptor.is_entity_tabu(1));

    acceptor.phase_ended();

    assert!(!acceptor.is_entity_tabu(1));
}

// Tests for MoveTabuAcceptor tabu rejection
#[test]
fn test_move_tabu_rejects_tabu_move() {
    let mut acceptor = MoveTabuAcceptor::<DummySolution>::new(3);
    acceptor.phase_started(&SimpleScore::of(-10));

    // Record and end step with move hash 42
    acceptor.record_move(42);
    acceptor.step_ended(&SimpleScore::of(-5));

    // New step with same move should be rejected (even with equal score)
    acceptor.step_started();
    acceptor.record_move(42);
    assert!(!acceptor.is_accepted(&SimpleScore::of(-5), &SimpleScore::of(-5)));

    // Different move should be accepted
    acceptor.step_started();
    acceptor.record_move(99);
    assert!(acceptor.is_accepted(&SimpleScore::of(-5), &SimpleScore::of(-5)));
}

#[test]
fn test_move_tabu_aspiration_overrides() {
    let mut acceptor = MoveTabuAcceptor::<DummySolution>::new(3);
    acceptor.phase_started(&SimpleScore::of(-10));

    // Record and end step
    acceptor.record_move(42);
    acceptor.step_ended(&SimpleScore::of(-5));

    // Tabu move with better-than-best score should be accepted via aspiration
    acceptor.step_started();
    acceptor.record_move(42);
    assert!(acceptor.is_accepted(&SimpleScore::of(-5), &SimpleScore::of(-2)));
}

#[test]
fn test_move_tabu_without_aspiration() {
    let mut acceptor = MoveTabuAcceptor::<DummySolution>::without_aspiration(3);
    acceptor.phase_started(&SimpleScore::of(-10));

    // Record and end step
    acceptor.record_move(42);
    acceptor.step_ended(&SimpleScore::of(-5));

    // Tabu move should be rejected even if better than best (no aspiration)
    acceptor.step_started();
    acceptor.record_move(42);
    assert!(!acceptor.is_accepted(&SimpleScore::of(-5), &SimpleScore::of(-2)));
}

// Tests for EntityTabuAcceptor tabu rejection
#[test]
fn test_entity_tabu_rejects_tabu_entity() {
    let mut acceptor = EntityTabuAcceptor::<DummySolution>::new(3);
    acceptor.phase_started(&SimpleScore::of(-10));

    // Record entity 1 and end step
    acceptor.step_started();
    acceptor.record_entity_move(1);
    acceptor.step_ended(&SimpleScore::of(-5));

    // New step with same entity should be rejected
    acceptor.step_started();
    acceptor.record_entity_move(1);
    assert!(!acceptor.is_accepted(&SimpleScore::of(-5), &SimpleScore::of(-5)));

    // Different entity should be accepted
    acceptor.step_started();
    acceptor.record_entity_move(99);
    assert!(acceptor.is_accepted(&SimpleScore::of(-5), &SimpleScore::of(-5)));
}

#[test]
fn test_entity_tabu_aspiration_overrides() {
    let mut acceptor = EntityTabuAcceptor::<DummySolution>::new(3);
    acceptor.phase_started(&SimpleScore::of(-10));

    // Record entity and end step
    acceptor.step_started();
    acceptor.record_entity_move(1);
    acceptor.step_ended(&SimpleScore::of(-5));

    // Tabu entity with better-than-best score should be accepted
    acceptor.step_started();
    acceptor.record_entity_move(1);
    assert!(acceptor.is_accepted(&SimpleScore::of(-5), &SimpleScore::of(-2)));
}

#[test]
fn test_entity_tabu_without_aspiration() {
    let mut acceptor = EntityTabuAcceptor::<DummySolution>::without_aspiration(3);
    acceptor.phase_started(&SimpleScore::of(-10));

    // Record entity and end step
    acceptor.step_started();
    acceptor.record_entity_move(1);
    acceptor.step_ended(&SimpleScore::of(-5));

    // Tabu entity should be rejected even if better than best
    acceptor.step_started();
    acceptor.record_entity_move(1);
    assert!(!acceptor.is_accepted(&SimpleScore::of(-5), &SimpleScore::of(-2)));
}

// Tests for ValueTabuAcceptor tabu rejection
#[test]
fn test_value_tabu_rejects_tabu_value() {
    let mut acceptor = ValueTabuAcceptor::<DummySolution>::new(3);
    acceptor.phase_started(&SimpleScore::of(-10));

    // Record value 42 and end step
    acceptor.step_started();
    acceptor.record_value_assignment(42);
    acceptor.step_ended(&SimpleScore::of(-5));

    // New step with same value should be rejected
    acceptor.step_started();
    acceptor.record_value_assignment(42);
    assert!(!acceptor.is_accepted(&SimpleScore::of(-5), &SimpleScore::of(-5)));

    // Different value should be accepted
    acceptor.step_started();
    acceptor.record_value_assignment(99);
    assert!(acceptor.is_accepted(&SimpleScore::of(-5), &SimpleScore::of(-5)));
}

#[test]
fn test_value_tabu_aspiration_overrides() {
    let mut acceptor = ValueTabuAcceptor::<DummySolution>::new(3);
    acceptor.phase_started(&SimpleScore::of(-10));

    // Record value and end step
    acceptor.step_started();
    acceptor.record_value_assignment(42);
    acceptor.step_ended(&SimpleScore::of(-5));

    // Tabu value with better-than-best score should be accepted
    acceptor.step_started();
    acceptor.record_value_assignment(42);
    assert!(acceptor.is_accepted(&SimpleScore::of(-5), &SimpleScore::of(-2)));
}

#[test]
fn test_value_tabu_without_aspiration() {
    let mut acceptor = ValueTabuAcceptor::<DummySolution>::without_aspiration(3);
    acceptor.phase_started(&SimpleScore::of(-10));

    // Record value and end step
    acceptor.step_started();
    acceptor.record_value_assignment(42);
    acceptor.step_ended(&SimpleScore::of(-5));

    // Tabu value should be rejected even if better than best
    acceptor.step_started();
    acceptor.record_value_assignment(42);
    assert!(!acceptor.is_accepted(&SimpleScore::of(-5), &SimpleScore::of(-2)));
}
