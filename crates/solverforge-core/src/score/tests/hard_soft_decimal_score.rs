use crate::score::*;

#[test]
fn test_creation_unscaled() {
    let score = HardSoftDecimalScore::of(-2, -100);
    assert_eq!(score.hard_scaled(), -200000);
    assert_eq!(score.soft_scaled(), -10000000);

    // Wire hard_score() / soft_score() / has_hard_component()
    assert_eq!(
        score.hard_score(),
        HardSoftDecimalScore::of_hard_scaled(-200000)
    );
    assert_eq!(
        score.soft_score(),
        HardSoftDecimalScore::of_soft_scaled(-10000000)
    );
    assert!(score.has_hard_component());
    assert!(!HardSoftDecimalScore::ZERO.has_hard_component());
}

#[test]
fn test_creation_scaled() {
    let score = HardSoftDecimalScore::of_scaled(-30500, -208250);
    assert_eq!(score.hard_scaled(), -30500);
    assert_eq!(score.soft_scaled(), -208250);
}

#[test]
fn test_feasibility() {
    assert!(HardSoftDecimalScore::of(0, -1000).is_feasible());
    assert!(HardSoftDecimalScore::of(10, -50).is_feasible());
    assert!(!HardSoftDecimalScore::of(-1, 0).is_feasible());
    assert!(!HardSoftDecimalScore::of_scaled(-1, 0).is_feasible());
}

#[test]
fn test_comparison() {
    let infeasible = HardSoftDecimalScore::of(-1, 0);
    let feasible = HardSoftDecimalScore::of(0, -1000);
    assert!(feasible > infeasible);
    assert!(feasible.is_better_than(&infeasible));

    let s1 = HardSoftDecimalScore::of(0, -100);
    let s2 = HardSoftDecimalScore::of(0, -50);
    assert!(s2 > s1);
    assert!(s1.is_worse_than(&s2));

    let s3 = HardSoftDecimalScore::of(-2, 0);
    let s4 = HardSoftDecimalScore::of(-1, -1000);
    assert!(s4 > s3);
}

#[test]
fn test_arithmetic() {
    let s1 = HardSoftDecimalScore::of(-1, -100);
    let s2 = HardSoftDecimalScore::of(-1, -50);

    assert_eq!(s1 + s2, HardSoftDecimalScore::of(-2, -150));
    assert_eq!(s1 - s2, HardSoftDecimalScore::of(0, -50));
    assert_eq!(-s1, HardSoftDecimalScore::of(1, 100));
}

#[test]
fn test_arithmetic_scaled() {
    let s1 = HardSoftDecimalScore::of_scaled(-1500, -100500);
    let s2 = HardSoftDecimalScore::of_scaled(-500, -50250);

    let sum = s1 + s2;
    assert_eq!(sum.hard_scaled(), -2000);
    assert_eq!(sum.soft_scaled(), -150750);
}

#[test]
fn test_parse_integer() {
    let score = HardSoftDecimalScore::parse("0hard/-100soft").unwrap();
    assert_eq!(score.hard_scaled(), 0);
    assert_eq!(score.soft_scaled(), -10000000);
}

#[test]
fn test_parse_decimal() {
    let score = HardSoftDecimalScore::parse("-30.5hard/-208.25soft").unwrap();
    assert_eq!(score.hard_scaled(), -3050000);
    assert_eq!(score.soft_scaled(), -20825000);
}

#[test]
fn test_display() {
    let score = HardSoftDecimalScore::of_scaled(-3050000, -20825000);
    assert_eq!(format!("{}", score), "-30.5hard/-208.25soft");
}

#[test]
fn test_display_integer() {
    let score = HardSoftDecimalScore::of(-2, -100);
    assert_eq!(format!("{}", score), "-2hard/-100soft");
}

#[test]
fn test_display_zero() {
    let score = HardSoftDecimalScore::ZERO;
    assert_eq!(format!("{}", score), "0hard/0soft");
}

#[test]
fn test_level_numbers() {
    let score = HardSoftDecimalScore::of_scaled(-2000, -50000);
    assert_eq!(score.to_level_numbers(), vec![-2000, -50000]);
    assert_eq!(
        HardSoftDecimalScore::from_level_numbers(&[-2000, -50000]),
        score
    );
}

#[test]
fn test_level_label() {
    assert_eq!(HardSoftDecimalScore::level_label(0), ScoreLevel::Hard);
    assert_eq!(HardSoftDecimalScore::level_label(1), ScoreLevel::Soft);
}
