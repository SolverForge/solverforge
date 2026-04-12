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
    let acceptor = MoveTabuAcceptor::new(5);
    assert!(!acceptor.is_move_tabu(42));
    assert!(acceptor.aspiration_enabled());
}

#[test]
fn test_without_aspiration() {
    let acceptor = MoveTabuAcceptor::without_aspiration(5);
    assert!(!acceptor.aspiration_enabled());
}

#[test]
fn test_record_and_check() {
    let mut acceptor = MoveTabuAcceptor::new(5);
    Acceptor::<TestSolution>::phase_started(&mut acceptor, &SoftScore::of(0));

    acceptor.record_move(42);
    Acceptor::<TestSolution>::step_ended(&mut acceptor, &SoftScore::of(0));

    assert!(acceptor.is_move_tabu(42));
    assert!(!acceptor.is_move_tabu(99));
}

#[test]
fn test_tabu_expiration() {
    let mut acceptor = MoveTabuAcceptor::new(2);
    Acceptor::<TestSolution>::phase_started(&mut acceptor, &SoftScore::of(0));

    acceptor.record_move(1);
    Acceptor::<TestSolution>::step_ended(&mut acceptor, &SoftScore::of(0));
    assert!(acceptor.is_move_tabu(1));

    Acceptor::<TestSolution>::step_started(&mut acceptor);
    acceptor.record_move(2);
    Acceptor::<TestSolution>::step_ended(&mut acceptor, &SoftScore::of(0));
    assert!(acceptor.is_move_tabu(1));
    assert!(acceptor.is_move_tabu(2));

    Acceptor::<TestSolution>::step_started(&mut acceptor);
    acceptor.record_move(3);
    Acceptor::<TestSolution>::step_ended(&mut acceptor, &SoftScore::of(0));

    assert!(!acceptor.is_move_tabu(1));
    assert!(acceptor.is_move_tabu(2));
    assert!(acceptor.is_move_tabu(3));
}

#[test]
fn test_accepts_improving_move() {
    let mut acceptor = MoveTabuAcceptor::new(5);
    let last_score = SoftScore::of(-10);
    let move_score = SoftScore::of(-5);
    assert!(Acceptor::<TestSolution>::is_accepted(
        &mut acceptor,
        &last_score,
        &move_score
    ));
}

#[test]
fn test_phase_clears_tabu() {
    let mut acceptor = MoveTabuAcceptor::new(5);
    Acceptor::<TestSolution>::phase_started(&mut acceptor, &SoftScore::of(0));

    acceptor.record_move(42);
    Acceptor::<TestSolution>::step_ended(&mut acceptor, &SoftScore::of(0));
    assert!(acceptor.is_move_tabu(42));

    Acceptor::<TestSolution>::phase_ended(&mut acceptor);
    assert!(!acceptor.is_move_tabu(42));
}

#[test]
fn test_no_move_recorded_no_tabu() {
    let mut acceptor = MoveTabuAcceptor::new(5);
    Acceptor::<TestSolution>::phase_started(&mut acceptor, &SoftScore::of(0));

    Acceptor::<TestSolution>::step_ended(&mut acceptor, &SoftScore::of(0));

    assert!(!acceptor.is_move_tabu(42));
}
