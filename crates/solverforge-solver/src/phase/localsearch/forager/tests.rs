use super::*;

use crate::heuristic::r#move::ChangeMove;
use crate::heuristic::selector::move_selector::CandidateId;
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

fn zero() -> SoftScore {
    SoftScore::of(0)
}

#[test]
fn test_accepted_count_forager_quits_at_limit() {
    let mut forager = AcceptedCountForager::<DummySolution>::new(3);
    <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::step_started(
        &mut forager,
        zero(),
        zero(),
    );

    <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(
        &mut forager,
        CandidateId::new(0),
        SoftScore::of(-10),
    );
    assert!(
        !<AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::is_quit_early(
            &forager,
        )
    );

    <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(
        &mut forager,
        CandidateId::new(1),
        SoftScore::of(-5),
    );
    assert!(
        !<AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::is_quit_early(
            &forager,
        )
    );

    <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(
        &mut forager,
        CandidateId::new(2),
        SoftScore::of(-8),
    );
    assert!(
        <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::is_quit_early(
            &forager,
        )
    );
}

#[test]
fn test_accepted_count_forager_picks_best_of_first_n_accepted_moves() {
    let mut forager = AcceptedCountForager::<DummySolution>::new(2);
    <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::step_started(
        &mut forager,
        zero(),
        zero(),
    );

    <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(
        &mut forager,
        CandidateId::new(0),
        SoftScore::of(-10),
    );
    <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(
        &mut forager,
        CandidateId::new(1),
        SoftScore::of(-5),
    );
    <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(
        &mut forager,
        CandidateId::new(2),
        SoftScore::of(-8),
    );
    <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(
        &mut forager,
        CandidateId::new(3),
        SoftScore::of(-1),
    );

    let (index, score) = <AcceptedCountForager<DummySolution> as LocalSearchForager<
        DummySolution,
        TestMove,
    >>::pick_move_index(&mut forager)
    .unwrap();
    assert_eq!(index.index(), 1);
    assert_eq!(score, SoftScore::of(-5));
}

#[test]
fn test_accepted_count_forager_ignores_candidates_after_limit() {
    let mut forager = AcceptedCountForager::<DummySolution>::new(1);
    <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::step_started(
        &mut forager,
        zero(),
        zero(),
    );

    <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(
        &mut forager,
        CandidateId::new(0),
        SoftScore::of(-10),
    );
    <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(
        &mut forager,
        CandidateId::new(1),
        SoftScore::of(-1),
    );

    let (index, score) = <AcceptedCountForager<DummySolution> as LocalSearchForager<
        DummySolution,
        TestMove,
    >>::pick_move_index(&mut forager)
    .unwrap();
    assert_eq!(index.index(), 0);
    assert_eq!(score, SoftScore::of(-10));
}

#[test]
fn test_accepted_count_forager_picks_best_index() {
    let mut forager = AcceptedCountForager::<DummySolution>::new(10);
    <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::step_started(
        &mut forager,
        zero(),
        zero(),
    );

    <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(
        &mut forager,
        CandidateId::new(0),
        SoftScore::of(-10),
    );
    <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(
        &mut forager,
        CandidateId::new(1),
        SoftScore::of(-5),
    );
    <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(
        &mut forager,
        CandidateId::new(2),
        SoftScore::of(-8),
    );

    let (index, score) = <AcceptedCountForager<DummySolution> as LocalSearchForager<
        DummySolution,
        TestMove,
    >>::pick_move_index(&mut forager)
    .unwrap();
    assert_eq!(index.index(), 1);
    assert_eq!(score, SoftScore::of(-5));
}

#[test]
fn test_accepted_count_forager_empty() {
    let mut forager = AcceptedCountForager::<DummySolution>::new(3);
    <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::step_started(
        &mut forager,
        zero(),
        zero(),
    );

    assert!(
        <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::pick_move_index(
            &mut forager,
        )
        .is_none()
    );
}

#[test]
fn test_first_accepted_forager() {
    let mut forager = FirstAcceptedForager::<DummySolution>::new();
    <FirstAcceptedForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::step_started(
        &mut forager,
        zero(),
        zero(),
    );

    assert!(
        !<FirstAcceptedForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::is_quit_early(
            &forager,
        )
    );

    <FirstAcceptedForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(
        &mut forager,
        CandidateId::new(0),
        SoftScore::of(-10),
    );
    assert!(
        <FirstAcceptedForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::is_quit_early(
            &forager,
        )
    );

    <FirstAcceptedForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(
        &mut forager,
        CandidateId::new(1),
        SoftScore::of(-5),
    );

    let (index, score) = <FirstAcceptedForager<DummySolution> as LocalSearchForager<
        DummySolution,
        TestMove,
    >>::pick_move_index(&mut forager)
    .unwrap();
    assert_eq!(index.index(), 0);
    assert_eq!(score, SoftScore::of(-10));
}

#[test]
fn test_accepted_count_one_matches_first_accepted_horizon() {
    let mut accepted_count = AcceptedCountForager::<DummySolution>::new(1);
    let mut first_accepted = FirstAcceptedForager::<DummySolution>::new();
    <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::step_started(
        &mut accepted_count,
        zero(),
        zero(),
    );
    <FirstAcceptedForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::step_started(
        &mut first_accepted,
        zero(),
        zero(),
    );

    <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(
        &mut accepted_count,
        CandidateId::new(0),
        SoftScore::of(-10),
    );
    <FirstAcceptedForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(
        &mut first_accepted,
        CandidateId::new(0),
        SoftScore::of(-10),
    );

    assert!(
        <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::is_quit_early(
            &accepted_count,
        )
    );
    assert!(
        <FirstAcceptedForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::is_quit_early(
            &first_accepted,
        )
    );

    assert_eq!(
        <AcceptedCountForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::pick_move_index(&mut accepted_count),
        <FirstAcceptedForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::pick_move_index(&mut first_accepted)
    );
}

#[test]
fn foragers_report_stream_horizon_only_for_finite_accepted_counts() {
    let accepted_count = AcceptedCountForager::<DummySolution>::new(4);
    let first_accepted = FirstAcceptedForager::<DummySolution>::new();
    let best_score = BestScoreForager::<DummySolution>::new();
    let first_best = FirstBestScoreImprovingForager::<DummySolution>::new();
    let first_last = FirstLastStepScoreImprovingForager::<DummySolution>::new();

    assert_eq!(
        <AcceptedCountForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::accepted_count_limit(&accepted_count),
        Some(4)
    );
    assert_eq!(
        <FirstAcceptedForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::accepted_count_limit(&first_accepted),
        Some(1)
    );
    assert_eq!(
        <BestScoreForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::accepted_count_limit(&best_score),
        None
    );
    assert_eq!(
        <FirstBestScoreImprovingForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::accepted_count_limit(&first_best),
        None
    );
    assert_eq!(
        <FirstLastStepScoreImprovingForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::accepted_count_limit(&first_last),
        None
    );
}

#[test]
fn test_forager_resets_on_step() {
    let mut forager = AcceptedCountForager::<DummySolution>::new(3);

    <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::step_started(
        &mut forager,
        zero(),
        zero(),
    );
    <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(
        &mut forager,
        CandidateId::new(0),
        SoftScore::of(-10),
    );

    <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::step_started(
        &mut forager,
        zero(),
        zero(),
    );
    assert!(
        <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::pick_move_index(
            &mut forager,
        )
        .is_none()
    );
}

#[test]
#[should_panic(expected = "accepted_count_limit must be > 0")]
fn test_accepted_count_forager_zero_panics() {
    let _ = AcceptedCountForager::<DummySolution>::new(0);
}

#[test]
fn test_best_score_forager_never_quits_early() {
    let mut forager = BestScoreForager::<DummySolution>::new();
    <BestScoreForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::step_started(
        &mut forager,
        zero(),
        zero(),
    );

    <BestScoreForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(
        &mut forager,
        CandidateId::new(0),
        SoftScore::of(-5),
    );
    assert!(!<BestScoreForager<DummySolution> as LocalSearchForager<
        DummySolution,
        TestMove,
    >>::is_quit_early(&forager,));

    <BestScoreForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(
        &mut forager,
        CandidateId::new(1),
        SoftScore::of(-1),
    );
    let (index, score) = <BestScoreForager<DummySolution> as LocalSearchForager<
        DummySolution,
        TestMove,
    >>::pick_move_index(&mut forager)
    .unwrap();
    assert_eq!(index.index(), 1);
    assert_eq!(score, SoftScore::of(-1));
}

#[test]
fn test_first_best_score_improving_quits_on_improvement() {
    let best = SoftScore::of(-10);
    let mut forager = FirstBestScoreImprovingForager::<DummySolution>::new();
    <FirstBestScoreImprovingForager<DummySolution> as LocalSearchForager<
        DummySolution,
        TestMove,
    >>::step_started(&mut forager, best, zero());

    <FirstBestScoreImprovingForager<DummySolution> as LocalSearchForager<
        DummySolution,
        TestMove,
    >>::add_move_index(&mut forager, CandidateId::new(0), SoftScore::of(-15));
    assert!(
        !<FirstBestScoreImprovingForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::is_quit_early(&forager)
    );

    <FirstBestScoreImprovingForager<DummySolution> as LocalSearchForager<
        DummySolution,
        TestMove,
    >>::add_move_index(&mut forager, CandidateId::new(1), SoftScore::of(-5));
    assert!(
        <FirstBestScoreImprovingForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::is_quit_early(&forager)
    );

    let (index, score) = <FirstBestScoreImprovingForager<DummySolution> as LocalSearchForager<
        DummySolution,
        TestMove,
    >>::pick_move_index(&mut forager)
    .unwrap();
    assert_eq!(index.index(), 1);
    assert_eq!(score, SoftScore::of(-5));
}

#[test]
fn test_first_last_step_improving_quits_on_improvement() {
    let last_step = SoftScore::of(-10);
    let mut forager = FirstLastStepScoreImprovingForager::<DummySolution>::new();
    <FirstLastStepScoreImprovingForager<DummySolution> as LocalSearchForager<
        DummySolution,
        TestMove,
    >>::step_started(&mut forager, zero(), last_step);

    <FirstLastStepScoreImprovingForager<DummySolution> as LocalSearchForager<
        DummySolution,
        TestMove,
    >>::add_move_index(&mut forager, CandidateId::new(0), SoftScore::of(-15));
    assert!(
        !<FirstLastStepScoreImprovingForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::is_quit_early(&forager)
    );

    <FirstLastStepScoreImprovingForager<DummySolution> as LocalSearchForager<
        DummySolution,
        TestMove,
    >>::add_move_index(&mut forager, CandidateId::new(1), SoftScore::of(-5));
    assert!(
        <FirstLastStepScoreImprovingForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::is_quit_early(&forager)
    );

    let (index, score) =
        <FirstLastStepScoreImprovingForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::pick_move_index(&mut forager)
        .unwrap();
    assert_eq!(index.index(), 1);
    assert_eq!(score, SoftScore::of(-5));
}
