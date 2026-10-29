use super::*;

#[test]
fn test_best_score_forager_never_quits_early() {
    let mut forager = BestScoreForager::<DummySolution>::new(false);
    <BestScoreForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::step_started(
        &mut forager,
        zero(),
        zero(),
        0,
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
    let mut forager = FirstBestScoreImprovingForager::<DummySolution>::new(false);
    <FirstBestScoreImprovingForager<DummySolution> as LocalSearchForager<
        DummySolution,
        TestMove,
    >>::step_started(&mut forager, best, zero(), 0);

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
    let mut forager = FirstLastStepScoreImprovingForager::<DummySolution>::new(false);
    <FirstLastStepScoreImprovingForager<DummySolution> as LocalSearchForager<
        DummySolution,
        TestMove,
    >>::step_started(&mut forager, zero(), last_step, 0);

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

#[test]
fn first_last_step_improving_falls_back_to_accepted_count_limit() {
    let last_step = SoftScore::of(-10);
    let mut forager = FirstLastStepScoreImprovingForager::<DummySolution>::new(false)
        .with_accepted_count_limit(2);
    <FirstLastStepScoreImprovingForager<DummySolution> as LocalSearchForager<
        DummySolution,
        TestMove,
    >>::step_started(&mut forager, zero(), last_step, 0);

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
    >>::add_move_index(&mut forager, CandidateId::new(1), SoftScore::of(-12));
    assert!(
        <FirstLastStepScoreImprovingForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::is_quit_early(&forager)
    );

    <FirstLastStepScoreImprovingForager<DummySolution> as LocalSearchForager<
        DummySolution,
        TestMove,
    >>::add_move_index(&mut forager, CandidateId::new(2), SoftScore::of(-5));

    let (index, score) =
        <FirstLastStepScoreImprovingForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::pick_move_index(&mut forager)
        .unwrap();
    assert_eq!(index.index(), 1);
    assert_eq!(score, SoftScore::of(-12));
}
