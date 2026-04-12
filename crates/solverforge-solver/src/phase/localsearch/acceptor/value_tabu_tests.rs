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
fn test_new_acceptor() {
    let acceptor = ValueTabuAcceptor::new(5);
    assert!(!acceptor.is_value_tabu(42));
}

#[test]
fn test_record_and_check() {
    let mut acceptor = ValueTabuAcceptor::new(5);
    Acceptor::<TestSolution>::phase_started(&mut acceptor, &SoftScore::of(0));

    acceptor.record_value_assignment(42);
    Acceptor::<TestSolution>::step_ended(&mut acceptor, &SoftScore::of(0));

    assert!(acceptor.is_value_tabu(42));
    assert!(!acceptor.is_value_tabu(99));
}

#[test]
fn test_tabu_expiration() {
    let mut acceptor = ValueTabuAcceptor::new(2);
    Acceptor::<TestSolution>::phase_started(&mut acceptor, &SoftScore::of(0));

    acceptor.record_value_assignment(1);
    Acceptor::<TestSolution>::step_ended(&mut acceptor, &SoftScore::of(0));
    assert!(acceptor.is_value_tabu(1));

    Acceptor::<TestSolution>::step_started(&mut acceptor);
    acceptor.record_value_assignment(2);
    Acceptor::<TestSolution>::step_ended(&mut acceptor, &SoftScore::of(0));
    assert!(acceptor.is_value_tabu(1));
    assert!(acceptor.is_value_tabu(2));

    Acceptor::<TestSolution>::step_started(&mut acceptor);
    acceptor.record_value_assignment(3);
    Acceptor::<TestSolution>::step_ended(&mut acceptor, &SoftScore::of(0));

    assert!(!acceptor.is_value_tabu(1));
    assert!(acceptor.is_value_tabu(2));
    assert!(acceptor.is_value_tabu(3));
}

#[test]
fn test_accepts_improving_move() {
    let mut acceptor = ValueTabuAcceptor::new(5);
    let last_score = SoftScore::of(-10);
    let move_score = SoftScore::of(-5);
    assert!(Acceptor::<TestSolution>::is_accepted(
        &mut acceptor,
        &last_score,
        &move_score
    ));
}

#[test]
fn test_accepts_equal_move() {
    let mut acceptor = ValueTabuAcceptor::new(5);
    let score = SoftScore::of(-10);
    assert!(Acceptor::<TestSolution>::is_accepted(
        &mut acceptor,
        &score,
        &score
    ));
}

#[test]
fn test_rejects_worsening_move() {
    let mut acceptor = ValueTabuAcceptor::new(5);
    let last_score = SoftScore::of(-5);
    let move_score = SoftScore::of(-10);
    assert!(!Acceptor::<TestSolution>::is_accepted(
        &mut acceptor,
        &last_score,
        &move_score
    ));
}

#[test]
fn test_phase_clears_tabu() {
    let mut acceptor = ValueTabuAcceptor::new(5);
    Acceptor::<TestSolution>::phase_started(&mut acceptor, &SoftScore::of(0));

    acceptor.record_value_assignment(42);
    Acceptor::<TestSolution>::step_ended(&mut acceptor, &SoftScore::of(0));
    assert!(acceptor.is_value_tabu(42));

    Acceptor::<TestSolution>::phase_ended(&mut acceptor);
    assert!(!acceptor.is_value_tabu(42));
}
