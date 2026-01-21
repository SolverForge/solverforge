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
fn test_hill_climbing_rejects_equal() {
    let acceptor = HillClimbingAcceptor::<DummySolution>::new();
    assert!(!acceptor.is_accepted(&SimpleScore::of(-5), &SimpleScore::of(-5)));
}

#[test]
fn test_simulated_annealing_accepts_improving() {
    let acceptor = SimulatedAnnealingAcceptor::<DummySolution>::new(1.0, 0.99);
    assert!(acceptor.is_accepted(&SimpleScore::of(-10), &SimpleScore::of(-5)));
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
fn test_tabu_search_rejects_tabu() {
    let mut acceptor = TabuSearchAcceptor::<DummySolution>::new(5);
    acceptor.phase_started(&SimpleScore::of(-10));
    acceptor.step_ended(&SimpleScore::of(-5));
    assert!(!acceptor.is_accepted(&SimpleScore::of(-10), &SimpleScore::of(-5)));
}

#[test]
fn test_tabu_search_aspiration_accepts_new_best() {
    let mut acceptor = TabuSearchAcceptor::<DummySolution>::new(5);
    acceptor.phase_started(&SimpleScore::of(-10));
    acceptor.step_ended(&SimpleScore::of(-3));
    assert!(acceptor.is_accepted(&SimpleScore::of(-5), &SimpleScore::of(-2)));
}

#[test]
fn test_tabu_search_without_aspiration() {
    let mut acceptor = TabuSearchAcceptor::<DummySolution>::without_aspiration(5);
    acceptor.phase_started(&SimpleScore::of(-10));
    acceptor.step_ended(&SimpleScore::of(-3));
    assert!(!acceptor.is_accepted(&SimpleScore::of(-5), &SimpleScore::of(-3)));
}

#[test]
fn test_tabu_search_tabu_list_eviction() {
    let mut acceptor = TabuSearchAcceptor::<DummySolution>::new(3);
    acceptor.phase_started(&SimpleScore::of(-10));

    acceptor.step_ended(&SimpleScore::of(-9));
    acceptor.step_ended(&SimpleScore::of(-8));
    acceptor.step_ended(&SimpleScore::of(-7));

    assert!(!acceptor.is_accepted(&SimpleScore::of(-10), &SimpleScore::of(-9)));
    assert!(!acceptor.is_accepted(&SimpleScore::of(-10), &SimpleScore::of(-8)));
    assert!(!acceptor.is_accepted(&SimpleScore::of(-10), &SimpleScore::of(-7)));

    acceptor.step_ended(&SimpleScore::of(-6));

    assert!(acceptor.is_accepted(&SimpleScore::of(-10), &SimpleScore::of(-9)));
    assert!(!acceptor.is_accepted(&SimpleScore::of(-10), &SimpleScore::of(-8)));
    assert!(!acceptor.is_accepted(&SimpleScore::of(-10), &SimpleScore::of(-7)));
    assert!(!acceptor.is_accepted(&SimpleScore::of(-10), &SimpleScore::of(-6)));
}

#[test]
fn test_tabu_search_phase_ended_clears_tabu() {
    let mut acceptor = TabuSearchAcceptor::<DummySolution>::new(5);
    acceptor.phase_started(&SimpleScore::of(-10));
    acceptor.step_ended(&SimpleScore::of(-5));
    assert!(!acceptor.is_accepted(&SimpleScore::of(-10), &SimpleScore::of(-5)));

    acceptor.phase_ended();
    acceptor.phase_started(&SimpleScore::of(-10));

    assert!(acceptor.is_accepted(&SimpleScore::of(-10), &SimpleScore::of(-5)));
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
