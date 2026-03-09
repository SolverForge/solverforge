use crate::score::*;

#[test]
fn test_creation() {
    let score = SoftScore::of(-5);
    assert_eq!(score.score(), -5);
    assert_eq!(SoftScore::ONE, SoftScore::of(1));
}

#[test]
fn test_feasibility() {
    assert!(SoftScore::of(0).is_feasible());
    assert!(SoftScore::of(10).is_feasible());
    assert!(!SoftScore::of(-1).is_feasible());
}

#[test]
fn test_comparison() {
    use std::cmp::Ordering;

    let s1 = SoftScore::of(-10);
    let s2 = SoftScore::of(-5);
    let s3 = SoftScore::of(0);

    assert!(s3 > s2);
    assert!(s2 > s1);
    assert!(s1 < s2);

    // Wire Score convenience methods
    assert!(s3.is_better_than(&s2));
    assert!(s1.is_worse_than(&s2));
    assert!(s2.is_equal_to(&s2));
    assert_eq!(s3.compare(&s1), Ordering::Greater);
}

#[test]
fn test_arithmetic() {
    let s1 = SoftScore::of(10);
    let s2 = SoftScore::of(3);

    assert_eq!(s1 + s2, SoftScore::of(13));
    assert_eq!(s1 - s2, SoftScore::of(7));
    assert_eq!(-s1, SoftScore::of(-10));
}

#[test]
fn test_multiply_divide() {
    let score = SoftScore::of(10);

    assert_eq!(score.multiply(2.0), SoftScore::of(20));
    assert_eq!(score.divide(2.0), SoftScore::of(5));
}

#[test]
fn test_parse() {
    assert_eq!(SoftScore::parse("42").unwrap(), SoftScore::of(42));
    assert_eq!(SoftScore::parse("-10").unwrap(), SoftScore::of(-10));
    assert_eq!(SoftScore::parse("0init").unwrap(), SoftScore::of(0));
}

#[test]
fn test_level_numbers() {
    let score = SoftScore::of(-5);
    assert_eq!(score.to_level_numbers(), vec![-5]);
    assert_eq!(SoftScore::from_level_numbers(&[-5]), score);
}

#[test]
fn test_level_label() {
    assert_eq!(SoftScore::level_label(0), ScoreLevel::Soft);
}
