use super::*;

use crate::builder::AnyForager;
use crate::heuristic::r#move::ChangeMove;
use solverforge_core::score::SoftScore;

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

type TestMove = ChangeMove<DummySolution, i32>;

#[test]
fn any_forager_forwards_stream_horizon() {
    let accepted_count = AnyForager::AcceptedCount(AcceptedCountForager::<DummySolution>::new(4));
    let first_accepted = AnyForager::FirstAccepted(FirstAcceptedForager::<DummySolution>::new());
    let best_score = AnyForager::BestScore(BestScoreForager::<DummySolution>::new());
    let first_best =
        AnyForager::BestScoreImproving(FirstBestScoreImprovingForager::<DummySolution>::new());
    let first_last = AnyForager::LastStepScoreImproving(FirstLastStepScoreImprovingForager::<
        DummySolution,
    >::new());

    assert_eq!(
        <AnyForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::accepted_count_limit(&accepted_count),
        Some(4)
    );
    assert_eq!(
        <AnyForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::accepted_count_limit(&first_accepted),
        Some(1)
    );
    assert_eq!(
        <AnyForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::accepted_count_limit(&best_score),
        None
    );
    assert_eq!(
        <AnyForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::accepted_count_limit(&first_best),
        None
    );
    assert_eq!(
        <AnyForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::accepted_count_limit(&first_last),
        None
    );
}
