use super::*;
use crate::test_utils::{create_scope, create_scope_with_score, TestSolution};
use solverforge_core::score::SoftScore;
use std::thread::sleep;

#[test]
fn test_not_terminated_during_grace_period() {
    let termination =
        DiminishedReturnsTermination::<TestSolution>::new(Duration::from_millis(100), 0.0);

    let scope = create_scope_with_score(SoftScore::of(-100));
    assert!(!termination.is_terminated(&scope));
}

#[test]
fn test_terminates_with_zero_improvement() {
    let termination =
        DiminishedReturnsTermination::<TestSolution>::new(Duration::from_millis(500), 0.1);

    let scope = create_scope_with_score(SoftScore::of(-100));

    assert!(!termination.is_terminated(&scope));
    sleep(Duration::from_millis(50));
    assert!(!termination.is_terminated(&scope));

    sleep(Duration::from_millis(500));
    assert!(termination.is_terminated(&scope));
}

#[test]
fn test_not_terminated_with_sufficient_improvement() {
    let termination =
        DiminishedReturnsTermination::<TestSolution>::new(Duration::from_millis(50), 10.0);

    let mut scope = create_scope_with_score(SoftScore::of(-100));

    assert!(!termination.is_terminated(&scope));

    sleep(Duration::from_millis(60));

    scope.set_best_solution(
        TestSolution {
            score: Some(SoftScore::of(0)),
        },
        SoftScore::of(0),
    );
    assert!(!termination.is_terminated(&scope));
}

#[test]
fn test_no_score_does_not_terminate() {
    let termination =
        DiminishedReturnsTermination::<TestSolution>::new(Duration::from_millis(10), 0.0);

    let scope = create_scope();

    sleep(Duration::from_millis(20));
    assert!(!termination.is_terminated(&scope));
}
