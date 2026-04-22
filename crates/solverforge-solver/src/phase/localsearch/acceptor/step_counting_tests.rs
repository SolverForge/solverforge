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
    let mut acceptor = StepCountingHillClimbingAcceptor::<TestSolution>::new(5);
    acceptor.phase_started(&SoftScore::of(-100));

    assert!(acceptor.is_accepted(&SoftScore::of(-100), &SoftScore::of(-50), None));
}

#[test]
fn test_accepts_non_improving_within_limit() {
    let mut acceptor = StepCountingHillClimbingAcceptor::<TestSolution>::new(5);
    acceptor.phase_started(&SoftScore::of(-100));

    assert!(acceptor.is_accepted(&SoftScore::of(-100), &SoftScore::of(-110), None));
}

#[test]
fn test_rejects_after_limit_exceeded() {
    let mut acceptor = StepCountingHillClimbingAcceptor::<TestSolution>::new(3);
    acceptor.phase_started(&SoftScore::of(-100));

    assert!(acceptor.is_accepted(&SoftScore::of(-100), &SoftScore::of(-110), None));
    acceptor.step_ended(&SoftScore::of(-110), None);

    assert!(acceptor.is_accepted(&SoftScore::of(-110), &SoftScore::of(-120), None));
    acceptor.step_ended(&SoftScore::of(-120), None);

    assert!(acceptor.is_accepted(&SoftScore::of(-120), &SoftScore::of(-130), None));
    acceptor.step_ended(&SoftScore::of(-130), None);

    assert!(!acceptor.is_accepted(&SoftScore::of(-130), &SoftScore::of(-140), None));
}

#[test]
fn test_resets_on_improvement() {
    let mut acceptor = StepCountingHillClimbingAcceptor::<TestSolution>::new(3);
    acceptor.phase_started(&SoftScore::of(-100));

    acceptor.step_ended(&SoftScore::of(-110), None);
    acceptor.step_ended(&SoftScore::of(-120), None);
    assert_eq!(acceptor.steps_since_improvement, 2);

    acceptor.step_ended(&SoftScore::of(-50), None);
    assert_eq!(acceptor.steps_since_improvement, 0);

    acceptor.step_ended(&SoftScore::of(-60), None);
    acceptor.step_ended(&SoftScore::of(-70), None);
    assert!(acceptor.is_accepted(&SoftScore::of(-70), &SoftScore::of(-80), None));
}

#[test]
fn test_improving_always_accepted_even_after_limit() {
    let mut acceptor = StepCountingHillClimbingAcceptor::<TestSolution>::new(2);
    acceptor.phase_started(&SoftScore::of(-100));

    acceptor.step_ended(&SoftScore::of(-110), None);
    acceptor.step_ended(&SoftScore::of(-120), None);

    assert!(!acceptor.is_accepted(&SoftScore::of(-120), &SoftScore::of(-130), None));
    assert!(acceptor.is_accepted(&SoftScore::of(-120), &SoftScore::of(-50), None));
}

#[test]
fn test_phase_reset() {
    let mut acceptor = StepCountingHillClimbingAcceptor::<TestSolution>::new(3);
    acceptor.phase_started(&SoftScore::of(-100));
    acceptor.step_ended(&SoftScore::of(-110), None);
    acceptor.step_ended(&SoftScore::of(-120), None);
    acceptor.phase_ended();

    acceptor.phase_started(&SoftScore::of(-200));
    assert_eq!(acceptor.steps_since_improvement, 0);
}
