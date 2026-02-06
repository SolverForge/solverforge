use crate::score::*;

#[test]
fn test_creation() {
    let score = HardSoftScore::of(-2, -100);
    assert_eq!(score.hard(), -2);
    assert_eq!(score.soft(), -100);

    // Wire hard_score() / soft_score() extraction methods
    assert_eq!(score.hard_score(), HardSoftScore::of_hard(-2));
    assert_eq!(score.soft_score(), HardSoftScore::of_soft(-100));
}

#[test]
fn test_feasibility() {
    assert!(HardSoftScore::of(0, -1000).is_feasible());
    assert!(HardSoftScore::of(10, -50).is_feasible());
    assert!(!HardSoftScore::of(-1, 0).is_feasible());
}

#[test]
fn test_comparison() {
    use std::cmp::Ordering;

    // Infeasible vs feasible
    let infeasible = HardSoftScore::of(-1, 0);
    let feasible = HardSoftScore::of(0, -1000);
    assert!(feasible > infeasible);
    assert!(feasible.is_better_than(&infeasible));
    assert!(infeasible.is_worse_than(&feasible));

    // Same hard, different soft
    let s1 = HardSoftScore::of(0, -100);
    let s2 = HardSoftScore::of(0, -50);
    assert!(s2 > s1);
    assert_eq!(s2.compare(&s1), Ordering::Greater);

    // Different hard
    let s3 = HardSoftScore::of(-2, 0);
    let s4 = HardSoftScore::of(-1, -1000);
    assert!(s4 > s3);

    // Equality
    let eq1 = HardSoftScore::of(0, -50);
    assert!(eq1.is_equal_to(&s2));
}

#[test]
fn test_arithmetic() {
    let s1 = HardSoftScore::of(-1, -100);
    let s2 = HardSoftScore::of(-1, -50);

    assert_eq!(s1 + s2, HardSoftScore::of(-2, -150));
    assert_eq!(s1 - s2, HardSoftScore::of(0, -50));
    assert_eq!(-s1, HardSoftScore::of(1, 100));
}

#[test]
fn test_parse() {
    assert_eq!(
        HardSoftScore::parse("0hard/-100soft").unwrap(),
        HardSoftScore::of(0, -100)
    );
    assert_eq!(
        HardSoftScore::parse("-1hard/0soft").unwrap(),
        HardSoftScore::of(-1, 0)
    );
}

#[test]
fn test_display() {
    let score = HardSoftScore::of(-1, -100);
    assert_eq!(format!("{}", score), "-1hard/-100soft");
}

#[test]
fn test_level_numbers() {
    let score = HardSoftScore::of(-2, -50);
    assert_eq!(score.to_level_numbers(), vec![-2, -50]);
    assert_eq!(HardSoftScore::from_level_numbers(&[-2, -50]), score);
}

#[test]
fn test_level_label() {
    assert_eq!(HardSoftScore::level_label(0), ScoreLevel::Hard);
    assert_eq!(HardSoftScore::level_label(1), ScoreLevel::Soft);
}
