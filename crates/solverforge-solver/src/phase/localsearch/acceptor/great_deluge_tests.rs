use super::*;
use solverforge_core::score::SoftScore;

#[derive(Clone)]
struct TestSolution {
    score: Option<SoftScore>,
}

impl PlanningSolution for TestSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

#[test]
fn test_accepts_improving_moves() {
    let mut acceptor = GreatDelugeAcceptor::<TestSolution>::new(0.001);
    acceptor.phase_started(&SoftScore::of(-100));

    assert!(acceptor.is_accepted(&SoftScore::of(-100), &SoftScore::of(-50)));
}

#[test]
fn test_accepts_above_water_level() {
    let mut acceptor = GreatDelugeAcceptor::<TestSolution>::new(0.001);
    acceptor.phase_started(&SoftScore::of(-100));

    assert!(acceptor.is_accepted(&SoftScore::of(-100), &SoftScore::of(-100)));
    assert!(acceptor.is_accepted(&SoftScore::of(-100), &SoftScore::of(-90)));
}

#[test]
fn test_rejects_below_water_level() {
    let mut acceptor = GreatDelugeAcceptor::<TestSolution>::new(0.001);
    acceptor.phase_started(&SoftScore::of(-100));

    assert!(!acceptor.is_accepted(&SoftScore::of(-100), &SoftScore::of(-110)));
}

#[test]
fn test_water_rises_over_time() {
    let mut acceptor = GreatDelugeAcceptor::<TestSolution>::new(0.1);
    acceptor.phase_started(&SoftScore::of(-100));

    assert!(acceptor.is_accepted(&SoftScore::of(-100), &SoftScore::of(-100)));
    assert!(!acceptor.is_accepted(&SoftScore::of(-100), &SoftScore::of(-101)));

    acceptor.step_ended(&SoftScore::of(-100));
    assert!(acceptor.is_accepted(&SoftScore::of(-90), &SoftScore::of(-90)));
    assert!(!acceptor.is_accepted(&SoftScore::of(-90), &SoftScore::of(-91)));

    acceptor.step_ended(&SoftScore::of(-90));
    assert!(acceptor.is_accepted(&SoftScore::of(-80), &SoftScore::of(-80)));
    assert!(!acceptor.is_accepted(&SoftScore::of(-80), &SoftScore::of(-81)));
}

#[test]
fn test_phase_reset() {
    let mut acceptor = GreatDelugeAcceptor::<TestSolution>::new(0.1);
    acceptor.phase_started(&SoftScore::of(-100));
    acceptor.step_ended(&SoftScore::of(-100));
    acceptor.phase_ended();

    acceptor.phase_started(&SoftScore::of(-50));
    assert!(acceptor.is_accepted(&SoftScore::of(-50), &SoftScore::of(-50)));
    assert!(!acceptor.is_accepted(&SoftScore::of(-50), &SoftScore::of(-51)));
}
