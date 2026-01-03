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
    let acceptor: Box<dyn Acceptor<DummySolution>> = Box::new(HillClimbingAcceptor::new());

    // Should accept improving move
    assert!(acceptor.is_accepted(&SimpleScore::of(-10), &SimpleScore::of(-5)));
}

#[test]
fn test_hill_climbing_rejects_worsening() {
    let acceptor: Box<dyn Acceptor<DummySolution>> = Box::new(HillClimbingAcceptor::new());

    // Should reject worsening move
    assert!(!acceptor.is_accepted(&SimpleScore::of(-5), &SimpleScore::of(-10)));
}

#[test]
fn test_hill_climbing_rejects_equal() {
    let acceptor: Box<dyn Acceptor<DummySolution>> = Box::new(HillClimbingAcceptor::new());

    // Should reject equal move (not strictly improving)
    assert!(!acceptor.is_accepted(&SimpleScore::of(-5), &SimpleScore::of(-5)));
}

#[test]
fn test_simulated_annealing_accepts_improving() {
    let acceptor: Box<dyn Acceptor<DummySolution>> =
        Box::new(SimulatedAnnealingAcceptor::new(1.0, 0.99));

    // Should always accept improving move
    assert!(acceptor.is_accepted(&SimpleScore::of(-10), &SimpleScore::of(-5)));
}

#[test]
fn test_late_acceptance_history() {
    let mut acceptor = LateAcceptanceAcceptor::<DummySolution>::new(5);

    // Initialize with a score
    acceptor.phase_started(&SimpleScore::of(-10));

    // Should accept improving move
    assert!(acceptor.is_accepted(&SimpleScore::of(-10), &SimpleScore::of(-5)));

    // Should accept equal to late score
    assert!(acceptor.is_accepted(&SimpleScore::of(-10), &SimpleScore::of(-10)));

    // Should reject worse than late score
    assert!(!acceptor.is_accepted(&SimpleScore::of(-10), &SimpleScore::of(-15)));
}

#[test]
fn test_tabu_search_accepts_improving() {
    let mut acceptor = TabuSearchAcceptor::<DummySolution>::new(5);
    acceptor.phase_started(&SimpleScore::of(-10));

    // Should accept improving move
    assert!(acceptor.is_accepted(&SimpleScore::of(-10), &SimpleScore::of(-5)));
}

#[test]
fn test_tabu_search_accepts_equal() {
    let mut acceptor = TabuSearchAcceptor::<DummySolution>::new(5);
    acceptor.phase_started(&SimpleScore::of(-10));

    // Should accept equal move (plateau exploration)
    assert!(acceptor.is_accepted(&SimpleScore::of(-10), &SimpleScore::of(-10)));
}

#[test]
fn test_tabu_search_rejects_tabu() {
    let mut acceptor = TabuSearchAcceptor::<DummySolution>::new(5);
    acceptor.phase_started(&SimpleScore::of(-10));

    // Record a step with score -5, which adds it to tabu list
    acceptor.step_ended(&SimpleScore::of(-5));

    // Should reject move to tabu score
    assert!(!acceptor.is_accepted(&SimpleScore::of(-10), &SimpleScore::of(-5)));
}

#[test]
fn test_tabu_search_aspiration_accepts_new_best() {
    let mut acceptor = TabuSearchAcceptor::<DummySolution>::new(5);
    acceptor.phase_started(&SimpleScore::of(-10));

    // Record a step with score -3, which adds it to tabu list
    acceptor.step_ended(&SimpleScore::of(-3));

    // Even though -3 is tabu, a new best score (-2) should be accepted via aspiration
    assert!(acceptor.is_accepted(&SimpleScore::of(-5), &SimpleScore::of(-2)));
}

#[test]
fn test_tabu_search_without_aspiration() {
    let mut acceptor = TabuSearchAcceptor::<DummySolution>::without_aspiration(5);
    acceptor.phase_started(&SimpleScore::of(-10));

    // Record a step with score -3
    acceptor.step_ended(&SimpleScore::of(-3));

    // Without aspiration, tabu scores are always rejected
    // Even if it would be a new best, we still reject tabu moves
    assert!(!acceptor.is_accepted(&SimpleScore::of(-5), &SimpleScore::of(-3)));
}

#[test]
fn test_tabu_search_tabu_list_eviction() {
    let mut acceptor = TabuSearchAcceptor::<DummySolution>::new(3);
    acceptor.phase_started(&SimpleScore::of(-10));

    // Fill up tabu list
    acceptor.step_ended(&SimpleScore::of(-9));
    acceptor.step_ended(&SimpleScore::of(-8));
    acceptor.step_ended(&SimpleScore::of(-7));

    // -9, -8, -7 should be tabu
    assert!(!acceptor.is_accepted(&SimpleScore::of(-10), &SimpleScore::of(-9)));
    assert!(!acceptor.is_accepted(&SimpleScore::of(-10), &SimpleScore::of(-8)));
    assert!(!acceptor.is_accepted(&SimpleScore::of(-10), &SimpleScore::of(-7)));

    // Add another score, which should evict -9
    acceptor.step_ended(&SimpleScore::of(-6));

    // -9 should no longer be tabu (evicted)
    assert!(acceptor.is_accepted(&SimpleScore::of(-10), &SimpleScore::of(-9)));

    // -8, -7, -6 should still be tabu
    assert!(!acceptor.is_accepted(&SimpleScore::of(-10), &SimpleScore::of(-8)));
    assert!(!acceptor.is_accepted(&SimpleScore::of(-10), &SimpleScore::of(-7)));
    assert!(!acceptor.is_accepted(&SimpleScore::of(-10), &SimpleScore::of(-6)));
}

#[test]
fn test_tabu_search_phase_ended_clears_tabu() {
    let mut acceptor = TabuSearchAcceptor::<DummySolution>::new(5);
    acceptor.phase_started(&SimpleScore::of(-10));

    // Add to tabu list
    acceptor.step_ended(&SimpleScore::of(-5));
    assert!(!acceptor.is_accepted(&SimpleScore::of(-10), &SimpleScore::of(-5)));

    // End phase, which should clear tabu list
    acceptor.phase_ended();

    // Start new phase
    acceptor.phase_started(&SimpleScore::of(-10));

    // -5 should no longer be tabu
    assert!(acceptor.is_accepted(&SimpleScore::of(-10), &SimpleScore::of(-5)));
}

#[test]
fn test_entity_tabu_accepts_improving() {
    let mut acceptor = EntityTabuAcceptor::new(5);
    <EntityTabuAcceptor as Acceptor<DummySolution>>::phase_started(
        &mut acceptor,
        &SimpleScore::of(-10),
    );

    // Should accept improving move
    assert!(<EntityTabuAcceptor as Acceptor<DummySolution>>::is_accepted(
        &acceptor,
        &SimpleScore::of(-10),
        &SimpleScore::of(-5)
    ));
}

#[test]
fn test_entity_tabu_accepts_equal() {
    let mut acceptor = EntityTabuAcceptor::new(5);
    <EntityTabuAcceptor as Acceptor<DummySolution>>::phase_started(
        &mut acceptor,
        &SimpleScore::of(-10),
    );

    // Should accept equal move
    assert!(<EntityTabuAcceptor as Acceptor<DummySolution>>::is_accepted(
        &acceptor,
        &SimpleScore::of(-10),
        &SimpleScore::of(-10)
    ));
}

#[test]
fn test_entity_tabu_rejects_worsening() {
    let mut acceptor = EntityTabuAcceptor::new(5);
    <EntityTabuAcceptor as Acceptor<DummySolution>>::phase_started(
        &mut acceptor,
        &SimpleScore::of(-10),
    );

    // Should reject worsening move
    assert!(!<EntityTabuAcceptor as Acceptor<DummySolution>>::is_accepted(
        &acceptor,
        &SimpleScore::of(-5),
        &SimpleScore::of(-10)
    ));
}

#[test]
fn test_entity_tabu_tracking() {
    let mut acceptor = EntityTabuAcceptor::new(3);
    <EntityTabuAcceptor as Acceptor<DummySolution>>::phase_started(
        &mut acceptor,
        &SimpleScore::of(-10),
    );

    // Record entity moves in a step
    <EntityTabuAcceptor as Acceptor<DummySolution>>::step_started(&mut acceptor);
    acceptor.record_entity_move(1);
    acceptor.record_entity_move(2);
    <EntityTabuAcceptor as Acceptor<DummySolution>>::step_ended(&mut acceptor, &SimpleScore::of(-5));

    // Entities 1 and 2 should be tabu
    assert!(acceptor.is_entity_tabu(1));
    assert!(acceptor.is_entity_tabu(2));
    assert!(!acceptor.is_entity_tabu(3));
}

#[test]
fn test_entity_tabu_eviction() {
    let mut acceptor = EntityTabuAcceptor::new(3);
    <EntityTabuAcceptor as Acceptor<DummySolution>>::phase_started(
        &mut acceptor,
        &SimpleScore::of(-10),
    );

    // Fill tabu list with 3 entities
    <EntityTabuAcceptor as Acceptor<DummySolution>>::step_started(&mut acceptor);
    acceptor.record_entity_move(1);
    <EntityTabuAcceptor as Acceptor<DummySolution>>::step_ended(&mut acceptor, &SimpleScore::of(-9));

    <EntityTabuAcceptor as Acceptor<DummySolution>>::step_started(&mut acceptor);
    acceptor.record_entity_move(2);
    <EntityTabuAcceptor as Acceptor<DummySolution>>::step_ended(&mut acceptor, &SimpleScore::of(-8));

    <EntityTabuAcceptor as Acceptor<DummySolution>>::step_started(&mut acceptor);
    acceptor.record_entity_move(3);
    <EntityTabuAcceptor as Acceptor<DummySolution>>::step_ended(&mut acceptor, &SimpleScore::of(-7));

    // All three should be tabu
    assert!(acceptor.is_entity_tabu(1));
    assert!(acceptor.is_entity_tabu(2));
    assert!(acceptor.is_entity_tabu(3));

    // Add another entity, which should evict entity 1
    <EntityTabuAcceptor as Acceptor<DummySolution>>::step_started(&mut acceptor);
    acceptor.record_entity_move(4);
    <EntityTabuAcceptor as Acceptor<DummySolution>>::step_ended(&mut acceptor, &SimpleScore::of(-6));

    // Entity 1 should no longer be tabu (evicted)
    assert!(!acceptor.is_entity_tabu(1));
    // Others should still be tabu
    assert!(acceptor.is_entity_tabu(2));
    assert!(acceptor.is_entity_tabu(3));
    assert!(acceptor.is_entity_tabu(4));
}

#[test]
fn test_entity_tabu_phase_ended_clears() {
    let mut acceptor = EntityTabuAcceptor::new(5);
    <EntityTabuAcceptor as Acceptor<DummySolution>>::phase_started(
        &mut acceptor,
        &SimpleScore::of(-10),
    );

    <EntityTabuAcceptor as Acceptor<DummySolution>>::step_started(&mut acceptor);
    acceptor.record_entity_move(1);
    <EntityTabuAcceptor as Acceptor<DummySolution>>::step_ended(&mut acceptor, &SimpleScore::of(-5));

    assert!(acceptor.is_entity_tabu(1));

    // End phase clears tabu list
    <EntityTabuAcceptor as Acceptor<DummySolution>>::phase_ended(&mut acceptor);

    assert!(!acceptor.is_entity_tabu(1));
}
