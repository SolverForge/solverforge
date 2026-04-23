use super::*;

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

fn zero() -> SoftScore {
    SoftScore::of(0)
}

#[test]
fn test_accepted_count_forager_never_quits_early() {
    let mut forager = AcceptedCountForager::<DummySolution>::new(3);
    <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::step_started(
        &mut forager,
        zero(),
        zero(),
    );

    <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(
        &mut forager,
        0,
        SoftScore::of(-10),
    );
    assert!(
        !<AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::is_quit_early(
            &forager,
        )
    );

    <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(
        &mut forager,
        1,
        SoftScore::of(-5),
    );
    assert!(
        !<AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::is_quit_early(
            &forager,
        )
    );

    <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(
        &mut forager,
        2,
        SoftScore::of(-8),
    );
    assert!(
        !<AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::is_quit_early(
            &forager,
        )
    );
}

#[test]
fn test_accepted_count_forager_retains_best_n_moves() {
    let mut forager = AcceptedCountForager::<DummySolution>::new(2);
    <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::step_started(
        &mut forager,
        zero(),
        zero(),
    );

    <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(
        &mut forager,
        0,
        SoftScore::of(-10),
    );
    <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(
        &mut forager,
        1,
        SoftScore::of(-5),
    );
    <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(
        &mut forager,
        2,
        SoftScore::of(-8),
    );
    <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(
        &mut forager,
        3,
        SoftScore::of(-1),
    );

    let (index, score) = <AcceptedCountForager<DummySolution> as LocalSearchForager<
        DummySolution,
        TestMove,
    >>::pick_move_index(&mut forager)
    .unwrap();
    assert_eq!(index, 3);
    assert_eq!(score, SoftScore::of(-1));
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
        0,
        SoftScore::of(-10),
    );
    <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(
        &mut forager,
        1,
        SoftScore::of(-5),
    );
    <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(
        &mut forager,
        2,
        SoftScore::of(-8),
    );

    let (index, score) = <AcceptedCountForager<DummySolution> as LocalSearchForager<
        DummySolution,
        TestMove,
    >>::pick_move_index(&mut forager)
    .unwrap();
    assert_eq!(index, 1);
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
        0,
        SoftScore::of(-10),
    );
    assert!(
        <FirstAcceptedForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::is_quit_early(
            &forager,
        )
    );

    <FirstAcceptedForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(
        &mut forager,
        1,
        SoftScore::of(-5),
    );

    let (index, score) = <FirstAcceptedForager<DummySolution> as LocalSearchForager<
        DummySolution,
        TestMove,
    >>::pick_move_index(&mut forager)
    .unwrap();
    assert_eq!(index, 0);
    assert_eq!(score, SoftScore::of(-10));
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
        0,
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
        0,
        SoftScore::of(-5),
    );
    assert!(!<BestScoreForager<DummySolution> as LocalSearchForager<
        DummySolution,
        TestMove,
    >>::is_quit_early(&forager,));

    <BestScoreForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(
        &mut forager,
        1,
        SoftScore::of(-1),
    );
    let (index, score) = <BestScoreForager<DummySolution> as LocalSearchForager<
        DummySolution,
        TestMove,
    >>::pick_move_index(&mut forager)
    .unwrap();
    assert_eq!(index, 1);
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
    >>::add_move_index(&mut forager, 0, SoftScore::of(-15));
    assert!(
        !<FirstBestScoreImprovingForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::is_quit_early(&forager)
    );

    <FirstBestScoreImprovingForager<DummySolution> as LocalSearchForager<
        DummySolution,
        TestMove,
    >>::add_move_index(&mut forager, 1, SoftScore::of(-5));
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
    assert_eq!(index, 1);
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
    >>::add_move_index(&mut forager, 0, SoftScore::of(-15));
    assert!(
        !<FirstLastStepScoreImprovingForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::is_quit_early(&forager)
    );

    <FirstLastStepScoreImprovingForager<DummySolution> as LocalSearchForager<
        DummySolution,
        TestMove,
    >>::add_move_index(&mut forager, 1, SoftScore::of(-5));
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
    assert_eq!(index, 1);
    assert_eq!(score, SoftScore::of(-5));
}
