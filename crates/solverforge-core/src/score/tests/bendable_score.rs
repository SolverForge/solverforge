use crate::score::*;

#[test]
fn test_creation() {
    let score: BendableScore<2, 3> = BendableScore::of([-1, -2], [-10, -20, -30]);
    assert_eq!(score.hard_levels_count(), 2);
    assert_eq!(score.soft_levels_count(), 3);
    assert_eq!(score.hard_score(0), -1);
    assert_eq!(score.hard_score(1), -2);
    assert_eq!(score.soft_score(2), -30);
}

#[test]
fn test_feasibility() {
    let feasible: BendableScore<2, 2> = BendableScore::of([0, 0], [-10, -20]);
    let infeasible: BendableScore<2, 2> = BendableScore::of([0, -1], [0, 0]);

    assert!(feasible.is_feasible());
    assert!(!infeasible.is_feasible());
}

#[test]
fn test_comparison() {
    use std::cmp::Ordering;

    // First hard level dominates
    let s1: BendableScore<2, 1> = BendableScore::of([-1, 0], [0]);
    let s2: BendableScore<2, 1> = BendableScore::of([0, -100], [-1000]);
    assert!(s2 > s1);
    assert!(s2.is_better_than(&s1));
    assert!(s1.is_worse_than(&s2));
    assert_eq!(s2.compare(&s1), Ordering::Greater);

    // Second hard level matters when first is equal
    let s3: BendableScore<2, 1> = BendableScore::of([0, -10], [0]);
    let s4: BendableScore<2, 1> = BendableScore::of([0, -5], [-100]);
    assert!(s4 > s3);
    assert!(s4.is_equal_to(&s4));
}

#[test]
fn test_arithmetic() {
    let s1: BendableScore<1, 2> = BendableScore::of([-1], [-10, -20]);
    let s2: BendableScore<1, 2> = BendableScore::of([-2], [-5, -10]);

    let sum = s1 + s2;
    assert_eq!(sum.hard_scores(), &[-3]);
    assert_eq!(sum.soft_scores(), &[-15, -30]);

    let neg = -s1;
    assert_eq!(neg.hard_scores(), &[1]);
    assert_eq!(neg.soft_scores(), &[10, 20]);
}

#[test]
fn test_copy() {
    let s1: BendableScore<1, 1> = BendableScore::of([-1], [-10]);
    let s2 = s1; // Copy
    assert_eq!(s1, s2); // s1 still valid
}

#[test]
fn test_level_label() {
    assert_eq!(BendableScore::<2, 3>::level_label(0), ScoreLevel::Hard);
    assert_eq!(BendableScore::<2, 3>::level_label(1), ScoreLevel::Hard);
    assert_eq!(BendableScore::<2, 3>::level_label(2), ScoreLevel::Soft);
    assert_eq!(BendableScore::<2, 3>::level_label(3), ScoreLevel::Soft);
    assert_eq!(BendableScore::<2, 3>::level_label(4), ScoreLevel::Soft);
}
