use crate::score::*;

#[test]
fn test_creation() {
    let score = HardMediumSoftScore::of(-2, -10, -100);
    assert_eq!(score.hard(), -2);
    assert_eq!(score.medium(), -10);
    assert_eq!(score.soft(), -100);
}

#[test]
fn test_feasibility() {
    assert!(HardMediumSoftScore::of(0, -100, -1000).is_feasible());
    assert!(!HardMediumSoftScore::of(-1, 0, 0).is_feasible());
}

#[test]
fn test_comparison() {
    // Hard dominates
    let s1 = HardMediumSoftScore::of(-1, 0, 0);
    let s2 = HardMediumSoftScore::of(0, -1000, -1000);
    assert!(s2 > s1);
    assert!(s2.is_better_than(&s1));
    assert!(s1.is_worse_than(&s2));

    // Medium dominates soft
    let s3 = HardMediumSoftScore::of(0, -10, 0);
    let s4 = HardMediumSoftScore::of(0, -5, -1000);
    assert!(s4 > s3);

    // Soft comparison when others equal
    let s5 = HardMediumSoftScore::of(0, 0, -100);
    let s6 = HardMediumSoftScore::of(0, 0, -50);
    assert!(s6 > s5);
    assert!(s6.is_equal_to(&HardMediumSoftScore::of(0, 0, -50)));
}

#[test]
fn test_arithmetic() {
    let s1 = HardMediumSoftScore::of(-1, -10, -100);
    let s2 = HardMediumSoftScore::of(-1, -5, -50);

    assert_eq!(s1 + s2, HardMediumSoftScore::of(-2, -15, -150));
    assert_eq!(s1 - s2, HardMediumSoftScore::of(0, -5, -50));
    assert_eq!(-s1, HardMediumSoftScore::of(1, 10, 100));
}

#[test]
fn test_parse() {
    assert_eq!(
        HardMediumSoftScore::parse("0hard/-10medium/-100soft").unwrap(),
        HardMediumSoftScore::of(0, -10, -100)
    );
}

#[test]
fn test_display() {
    let score = HardMediumSoftScore::of(-1, -10, -100);
    assert_eq!(format!("{}", score), "-1hard/-10medium/-100soft");
}

#[test]
fn test_level_label() {
    assert_eq!(HardMediumSoftScore::level_label(0), ScoreLevel::Hard);
    assert_eq!(HardMediumSoftScore::level_label(1), ScoreLevel::Medium);
    assert_eq!(HardMediumSoftScore::level_label(2), ScoreLevel::Soft);
}
