//! Tests for score types.

use super::*;

// ============================================================================
// SimpleScore Tests
// ============================================================================

mod simple_score {
    use super::*;

    #[test]
    fn test_creation() {
        let score = SimpleScore::of(-5);
        assert_eq!(score.score(), -5);
    }

    #[test]
    fn test_feasibility() {
        assert!(SimpleScore::of(0).is_feasible());
        assert!(SimpleScore::of(10).is_feasible());
        assert!(!SimpleScore::of(-1).is_feasible());
    }

    #[test]
    fn test_comparison() {
        let s1 = SimpleScore::of(-10);
        let s2 = SimpleScore::of(-5);
        let s3 = SimpleScore::of(0);

        assert!(s3 > s2);
        assert!(s2 > s1);
        assert!(s1 < s2);
    }

    #[test]
    fn test_arithmetic() {
        let s1 = SimpleScore::of(10);
        let s2 = SimpleScore::of(3);

        assert_eq!(s1 + s2, SimpleScore::of(13));
        assert_eq!(s1 - s2, SimpleScore::of(7));
        assert_eq!(-s1, SimpleScore::of(-10));
    }

    #[test]
    fn test_multiply_divide() {
        let score = SimpleScore::of(10);

        assert_eq!(score.multiply(2.0), SimpleScore::of(20));
        assert_eq!(score.divide(2.0), SimpleScore::of(5));
    }

    #[test]
    fn test_parse() {
        assert_eq!(SimpleScore::parse("42").unwrap(), SimpleScore::of(42));
        assert_eq!(SimpleScore::parse("-10").unwrap(), SimpleScore::of(-10));
        assert_eq!(SimpleScore::parse("0init").unwrap(), SimpleScore::of(0));
    }

    #[test]
    fn test_level_numbers() {
        let score = SimpleScore::of(-5);
        assert_eq!(score.to_level_numbers(), vec![-5]);
        assert_eq!(SimpleScore::from_level_numbers(&[-5]), score);
    }
}

// ============================================================================
// HardSoftScore Tests
// ============================================================================

mod hard_soft_score {
    use super::*;

    #[test]
    fn test_creation() {
        let score = HardSoftScore::of(-2, -100);
        assert_eq!(score.hard(), -2);
        assert_eq!(score.soft(), -100);
    }

    #[test]
    fn test_feasibility() {
        assert!(HardSoftScore::of(0, -1000).is_feasible());
        assert!(HardSoftScore::of(10, -50).is_feasible());
        assert!(!HardSoftScore::of(-1, 0).is_feasible());
    }

    #[test]
    fn test_comparison() {
        // Infeasible vs feasible
        let infeasible = HardSoftScore::of(-1, 0);
        let feasible = HardSoftScore::of(0, -1000);
        assert!(feasible > infeasible);

        // Same hard, different soft
        let s1 = HardSoftScore::of(0, -100);
        let s2 = HardSoftScore::of(0, -50);
        assert!(s2 > s1);

        // Different hard
        let s3 = HardSoftScore::of(-2, 0);
        let s4 = HardSoftScore::of(-1, -1000);
        assert!(s4 > s3);
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
}

// ============================================================================
// HardMediumSoftScore Tests
// ============================================================================

mod hard_medium_soft_score {
    use super::*;

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

        // Medium dominates soft
        let s3 = HardMediumSoftScore::of(0, -10, 0);
        let s4 = HardMediumSoftScore::of(0, -5, -1000);
        assert!(s4 > s3);

        // Soft comparison when others equal
        let s5 = HardMediumSoftScore::of(0, 0, -100);
        let s6 = HardMediumSoftScore::of(0, 0, -50);
        assert!(s6 > s5);
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
}

// ============================================================================
// HardSoftDecimalScore Tests
// ============================================================================

mod hard_soft_decimal_score {
    use super::*;

    #[test]
    fn test_creation_unscaled() {
        let score = HardSoftDecimalScore::of(-2, -100);
        assert_eq!(score.hard_scaled(), -200000);
        assert_eq!(score.soft_scaled(), -10000000);
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

        let s1 = HardSoftDecimalScore::of(0, -100);
        let s2 = HardSoftDecimalScore::of(0, -50);
        assert!(s2 > s1);

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
}

// ============================================================================
// BendableScore Tests
// ============================================================================

mod bendable_score {
    use super::*;

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
        // First hard level dominates
        let s1: BendableScore<2, 1> = BendableScore::of([-1, 0], [0]);
        let s2: BendableScore<2, 1> = BendableScore::of([0, -100], [-1000]);
        assert!(s2 > s1);

        // Second hard level matters when first is equal
        let s3: BendableScore<2, 1> = BendableScore::of([0, -10], [0]);
        let s4: BendableScore<2, 1> = BendableScore::of([0, -5], [-100]);
        assert!(s4 > s3);
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
}
