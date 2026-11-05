use solverforge_config::{ForagerConfig, ScoreTieBreak};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::SoftScore;

use crate::builder::{AnyForager, CompiledForager};
use crate::heuristic::r#move::ChangeMove;
use crate::heuristic::selector::move_selector::CandidateId;
use crate::phase::localsearch::LocalSearchForager;

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

fn build(config: ForagerConfig, score_tie_break: ScoreTieBreak) -> AnyForager<Plan> {
    CompiledForager::compile(&config)
        .expect("the forager config is valid")
        .build(score_tie_break)
}

#[test]
fn first_accepted_takes_the_first_candidate() {
    let mut forager = build(ForagerConfig::FirstAccepted {}, ScoreTieBreak::First);
    assert_eq!(forage(&mut forager, -10, -10, 0, &[-12, -5, -1]), (0, 1));
}

#[test]
fn first_best_score_improving_waits_for_a_new_best() {
    // -15 improves the last step (-20) but not the best (-10); -5 improves both.
    let mut forager = build(
        ForagerConfig::FirstBestScoreImproving {},
        ScoreTieBreak::First,
    );
    assert_eq!(forage(&mut forager, -10, -20, 0, &[-15, -5, -1]), (1, 2));
}

#[test]
fn first_last_step_score_improving_stops_on_beating_the_last_step() {
    let mut forager = build(
        ForagerConfig::FirstLastStepScoreImproving {},
        ScoreTieBreak::First,
    );
    assert_eq!(forage(&mut forager, -10, -20, 0, &[-15, -5, -1]), (0, 1));
}

#[test]
fn a_random_score_tie_break_spreads_picks_over_equal_candidates() {
    let mut random = build(ForagerConfig::BestScore {}, ScoreTieBreak::Random);
    assert!(tie_picks(&mut random).len() > 1);

    let mut first = build(ForagerConfig::BestScore {}, ScoreTieBreak::First);
    assert_eq!(tie_picks(&mut first), [0].into());
}
