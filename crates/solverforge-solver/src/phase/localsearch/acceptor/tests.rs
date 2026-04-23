use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::SoftScore;

use super::*;
use crate::heuristic::r#move::{metadata::MoveTabuScope, MoveTabuSignature};

#[derive(Clone, Debug)]
struct DummySolution {
    score: Option<SoftScore>,
}

impl PlanningSolution for DummySolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn signature(
    descriptor_index: usize,
    variable_name: &'static str,
    entity_ids: &[u64],
    destination_value_ids: &[u64],
    move_id: &[u64],
    undo_move_id: &[u64],
) -> MoveTabuSignature {
    let scope = MoveTabuScope::new(descriptor_index, variable_name);
    MoveTabuSignature::new(
        scope,
        move_id.iter().copied().collect(),
        undo_move_id.iter().copied().collect(),
    )
    .with_entity_tokens(
        entity_ids
            .iter()
            .copied()
            .map(|entity_id| scope.entity_token(entity_id)),
    )
    .with_destination_value_tokens(
        destination_value_ids
            .iter()
            .copied()
            .map(|value_id| scope.value_token(value_id)),
    )
}

fn policy(
    entity_tabu_size: Option<usize>,
    value_tabu_size: Option<usize>,
    move_tabu_size: Option<usize>,
    undo_move_tabu_size: Option<usize>,
    aspiration_enabled: bool,
) -> TabuSearchPolicy {
    TabuSearchPolicy {
        entity_tabu_size,
        value_tabu_size,
        move_tabu_size,
        undo_move_tabu_size,
        aspiration_enabled,
    }
}

#[test]
fn hill_climbing_accepts_only_improving_moves() {
    let mut acceptor = HillClimbingAcceptor::new();

    assert!(
        <HillClimbingAcceptor as Acceptor<DummySolution>>::is_accepted(
            &mut acceptor,
            &SoftScore::of(-10),
            &SoftScore::of(-5),
            None,
        )
    );
    assert!(
        !<HillClimbingAcceptor as Acceptor<DummySolution>>::is_accepted(
            &mut acceptor,
            &SoftScore::of(-5),
            &SoftScore::of(-10),
            None,
        )
    );
    assert!(
        !<HillClimbingAcceptor as Acceptor<DummySolution>>::is_accepted(
            &mut acceptor,
            &SoftScore::of(-5),
            &SoftScore::of(-5),
            None,
        )
    );
}

#[test]
fn late_acceptance_uses_late_score_threshold() {
    let mut acceptor = LateAcceptanceAcceptor::<DummySolution>::new(5);
    acceptor.phase_started(&SoftScore::of(-10));

    assert!(acceptor.is_accepted(&SoftScore::of(-10), &SoftScore::of(-5), None));
    assert!(acceptor.is_accepted(&SoftScore::of(-10), &SoftScore::of(-10), None));
    assert!(!acceptor.is_accepted(&SoftScore::of(-10), &SoftScore::of(-15), None));
}

#[test]
fn tabu_search_blocks_recent_entities_and_allows_aspiration() {
    let mut acceptor =
        TabuSearchAcceptor::<DummySolution>::new(policy(Some(3), None, None, None, true));
    let first = signature(0, "worker", &[7], &[], &[10], &[11]);
    let second = signature(0, "worker", &[7], &[], &[12], &[13]);

    acceptor.phase_started(&SoftScore::of(-10));
    acceptor.step_ended(&SoftScore::of(-9), Some(&first));

    assert!(!acceptor.is_accepted(&SoftScore::of(-9), &SoftScore::of(-9), Some(&second),));

    assert!(acceptor.is_accepted(&SoftScore::of(-9), &SoftScore::of(-5), Some(&second),));
}

#[test]
fn tabu_search_blocks_recent_values() {
    let mut acceptor =
        TabuSearchAcceptor::<DummySolution>::new(policy(None, Some(2), None, None, false));
    let first = signature(0, "worker", &[], &[42], &[10], &[11]);
    let second = signature(0, "worker", &[], &[42], &[12], &[13]);

    acceptor.phase_started(&SoftScore::of(-10));
    acceptor.step_ended(&SoftScore::of(-9), Some(&first));

    assert!(!acceptor.is_accepted(&SoftScore::of(-9), &SoftScore::of(-8), Some(&second),));
}

#[test]
fn tabu_search_blocks_exact_move_and_undo_move() {
    let mut acceptor =
        TabuSearchAcceptor::<DummySolution>::new(policy(None, None, Some(2), Some(2), false));
    let committed = signature(0, "worker", &[], &[], &[10, 20], &[30, 40]);
    let exact_repeat = signature(0, "worker", &[], &[], &[10, 20], &[99]);
    let undo_repeat = signature(0, "worker", &[], &[], &[30, 40], &[10, 20]);

    acceptor.phase_started(&SoftScore::of(-10));
    acceptor.step_ended(&SoftScore::of(-9), Some(&committed));

    assert!(!acceptor.is_accepted(&SoftScore::of(-9), &SoftScore::of(-8), Some(&exact_repeat),));
    assert!(!acceptor.is_accepted(&SoftScore::of(-9), &SoftScore::of(-8), Some(&undo_repeat),));
}

#[test]
fn tabu_search_undo_memory_matches_candidate_move_identity() {
    let mut acceptor =
        TabuSearchAcceptor::<DummySolution>::new(policy(None, None, None, Some(2), false));
    let committed = signature(0, "worker", &[], &[], &[10], &[20]);
    let reverse = signature(0, "worker", &[], &[], &[20], &[10]);
    let unrelated_with_same_undo_id = signature(0, "worker", &[], &[], &[30], &[20]);

    acceptor.phase_started(&SoftScore::of(-10));
    acceptor.step_ended(&SoftScore::of(-9), Some(&committed));

    assert!(!acceptor.is_accepted(&SoftScore::of(-9), &SoftScore::of(-8), Some(&reverse)));
    assert!(acceptor.is_accepted(
        &SoftScore::of(-9),
        &SoftScore::of(-8),
        Some(&unrelated_with_same_undo_id)
    ));
}

#[test]
fn tabu_search_requires_signatures() {
    let mut acceptor =
        TabuSearchAcceptor::<DummySolution>::new(policy(Some(1), None, None, None, true));
    acceptor.phase_started(&SoftScore::of(-10));

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        acceptor.is_accepted(&SoftScore::of(-10), &SoftScore::of(-9), None)
    }));

    assert!(result.is_err());
    assert!(acceptor.requires_move_signatures());
}

#[test]
fn tabu_search_clears_memories_on_phase_end() {
    let mut acceptor =
        TabuSearchAcceptor::<DummySolution>::new(policy(Some(1), Some(1), Some(1), Some(1), false));
    let sig = signature(0, "worker", &[1], &[2], &[3], &[4]);

    acceptor.phase_started(&SoftScore::of(-10));
    acceptor.step_ended(&SoftScore::of(-9), Some(&sig));
    assert!(!acceptor.is_accepted(&SoftScore::of(-9), &SoftScore::of(-8), Some(&sig)));

    acceptor.phase_ended();
    acceptor.phase_started(&SoftScore::of(-10));

    assert!(acceptor.is_accepted(&SoftScore::of(-10), &SoftScore::of(-9), Some(&sig)));
}

#[test]
fn tabu_search_move_only_policy_blocks_recent_exact_move() {
    let mut acceptor = TabuSearchAcceptor::<DummySolution>::new(TabuSearchPolicy::move_only(10));
    let committed = signature(0, "worker", &[], &[], &[10, 20], &[30, 40]);
    let repeated = signature(0, "worker", &[], &[], &[10, 20], &[99]);

    acceptor.phase_started(&SoftScore::of(-10));
    acceptor.step_ended(&SoftScore::of(-9), Some(&committed));

    assert!(!acceptor.is_accepted(&SoftScore::of(-9), &SoftScore::of(-9), Some(&repeated)));
}

#[test]
fn tabu_search_scoped_tokens_do_not_collide_across_variables() {
    let mut acceptor =
        TabuSearchAcceptor::<DummySolution>::new(policy(Some(2), Some(2), None, None, false));
    let first = signature(0, "worker_a", &[7], &[42], &[10], &[11]);
    let second = signature(0, "worker_b", &[7], &[42], &[12], &[13]);

    acceptor.phase_started(&SoftScore::of(-10));
    acceptor.step_ended(&SoftScore::of(-9), Some(&first));

    assert!(acceptor.is_accepted(&SoftScore::of(-9), &SoftScore::of(-8), Some(&second)));
}
