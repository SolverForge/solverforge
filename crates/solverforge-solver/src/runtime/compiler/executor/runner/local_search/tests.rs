use solverforge_config::{AcceptorConfig, ScoreTieBreak, TabuSearchConfig};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::SoftScore;

use super::build_acceptor_and_forager;
use crate::builder::{AnyForager, CompiledAcceptor};
use crate::heuristic::r#move::ChangeMove;
use crate::heuristic::selector::move_selector::CandidateId;
use crate::phase::localsearch::LocalSearchForager;
use crate::runtime::compiler::default_local_search::{
    DefaultLocalSearchAcceptorPolicy, DefaultLocalSearchComponents, DefaultLocalSearchForagerPolicy,
};

#[derive(Clone, Debug)]
struct Plan {
    score: Option<SoftScore>,
}

impl PlanningSolution for Plan {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

type TestMove = ChangeMove<Plan, usize>;

/// Starts a step at `best` and `last_step`, offers `scores` in order until the
/// forager quits, and returns the picked candidate and how many were offered.
fn forage(
    forager: &mut AnyForager<Plan>,
    best: i64,
    last_step: i64,
    step_seed: u64,
    scores: &[i64],
) -> (usize, usize) {
    LocalSearchForager::<Plan, TestMove>::step_started(
        forager,
        SoftScore::of(best),
        SoftScore::of(last_step),
        step_seed,
    );
    let mut offered = 0;
    for (index, score) in scores.iter().enumerate() {
        offered += 1;
        LocalSearchForager::<Plan, TestMove>::add_move_index(
            forager,
            CandidateId::new(index),
            SoftScore::of(*score),
        );
        if LocalSearchForager::<Plan, TestMove>::is_quit_early(forager) {
            break;
        }
    }
    let (picked, _) = LocalSearchForager::<Plan, TestMove>::pick_move_index(forager)
        .expect("an offered candidate is picked");
    (picked.index(), offered)
}

/// The candidates a forager picks among four equally scored ones over 32 steps.
fn tie_picks(forager: &mut AnyForager<Plan>) -> std::collections::BTreeSet<usize> {
    (0..32)
        .map(|seed| forage(forager, -1, -1, seed, &[-1, -1, -1, -1]).0)
        .collect()
}

const COMPONENTS: DefaultLocalSearchComponents = DefaultLocalSearchComponents {
    acceptor: DefaultLocalSearchAcceptorPolicy::LateAcceptance { history_size: 400 },
    forager: DefaultLocalSearchForagerPolicy::AcceptedCount { limit: 4 },
};

#[test]
fn the_default_forager_follows_the_score_tie_break() {
    let (_, mut random) =
        build_acceptor_and_forager::<Plan>(COMPONENTS, None, None, ScoreTieBreak::Random, Some(7));
    assert!(tie_picks(&mut random).len() > 1);

    let (_, mut first) =
        build_acceptor_and_forager::<Plan>(COMPONENTS, None, None, ScoreTieBreak::First, Some(7));
    assert_eq!(tie_picks(&mut first), [0].into());
}

#[test]
fn the_tabu_search_default_forager_follows_the_score_tie_break() {
    let tabu =
        CompiledAcceptor::<Plan>::compile(&AcceptorConfig::TabuSearch(TabuSearchConfig::default()))
            .expect("the default tabu search config is valid");

    let (_, mut random) = build_acceptor_and_forager::<Plan>(
        COMPONENTS,
        Some(&tabu),
        None,
        ScoreTieBreak::Random,
        Some(7),
    );
    assert!(tie_picks(&mut random).len() > 1);

    let (_, mut first) = build_acceptor_and_forager::<Plan>(
        COMPONENTS,
        Some(&tabu),
        None,
        ScoreTieBreak::First,
        Some(7),
    );
    assert_eq!(tie_picks(&mut first), [0].into());
}
