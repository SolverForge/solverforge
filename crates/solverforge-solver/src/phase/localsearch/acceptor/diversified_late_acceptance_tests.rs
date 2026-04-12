use super::*;
use solverforge_core::score::SoftScore;

#[derive(Clone)]
struct TestSolution;

impl PlanningSolution for TestSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        None
    }

    fn set_score(&mut self, _: Option<Self::Score>) {}
}

#[test]
fn test_accepts_improving_moves() {
    let mut acceptor = DiversifiedLateAcceptanceAcceptor::<TestSolution>::new(5, 0.1);
    acceptor.phase_started(&SoftScore::of(-100));

    assert!(acceptor.is_accepted(&SoftScore::of(-100), &SoftScore::of(-90)));
}

#[test]
fn test_accepts_late_equal() {
    let mut acceptor = DiversifiedLateAcceptanceAcceptor::<TestSolution>::new(3, 0.1);
    acceptor.phase_started(&SoftScore::of(-100));

    assert!(acceptor.is_accepted(&SoftScore::of(-90), &SoftScore::of(-100)));
}

#[test]
fn test_diversification_accepts_within_tolerance() {
    let mut acceptor = DiversifiedLateAcceptanceAcceptor::<TestSolution>::new(3, 0.1);
    acceptor.phase_started(&SoftScore::of(-100));

    acceptor.step_ended(&SoftScore::of(-80));
    acceptor.step_ended(&SoftScore::of(-70));
    acceptor.step_ended(&SoftScore::of(-60));

    assert!(acceptor.is_accepted(&SoftScore::of(-60), &SoftScore::of(-65)));
}

#[test]
fn test_rejects_outside_tolerance() {
    let mut acceptor = DiversifiedLateAcceptanceAcceptor::<TestSolution>::new(3, 0.05);
    acceptor.phase_started(&SoftScore::of(-100));

    acceptor.step_ended(&SoftScore::of(-40));
    acceptor.step_ended(&SoftScore::of(-40));
    acceptor.step_ended(&SoftScore::of(-40));

    assert!(!acceptor.is_accepted(&SoftScore::of(-40), &SoftScore::of(-50)));
}

#[test]
fn test_history_cycles() {
    let mut acceptor = DiversifiedLateAcceptanceAcceptor::<TestSolution>::new(3, 0.1);
    acceptor.phase_started(&SoftScore::of(-100));

    acceptor.step_ended(&SoftScore::of(-80));
    acceptor.step_ended(&SoftScore::of(-70));
    acceptor.step_ended(&SoftScore::of(-60));

    assert!(acceptor.is_accepted(&SoftScore::of(-60), &SoftScore::of(-75)));
}
