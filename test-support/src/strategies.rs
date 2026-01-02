//! proptest strategies for property-based testing.
//!
//! These strategies generate random but valid domain objects for testing
//! invariants like serialization roundtrips and arithmetic properties.

use proptest::prelude::*;
use solverforge_core::score::{BendableScore, HardMediumSoftScore, HardSoftScore, SimpleScore};

// =============================================================================
// SCORE STRATEGIES
// =============================================================================

/// Generates arbitrary SimpleScore values.
pub fn simple_score() -> impl Strategy<Value = SimpleScore> {
    any::<i64>().prop_map(SimpleScore::of)
}

/// Generates arbitrary HardSoftScore values.
pub fn hard_soft_score() -> impl Strategy<Value = HardSoftScore> {
    (any::<i64>(), any::<i64>()).prop_map(|(h, s)| HardSoftScore::of(h, s))
}

/// Generates feasible HardSoftScore (hard >= 0).
pub fn feasible_hard_soft_score() -> impl Strategy<Value = HardSoftScore> {
    (0i64..=i64::MAX, any::<i64>()).prop_map(|(h, s)| HardSoftScore::of(h, s))
}

/// Generates arbitrary HardMediumSoftScore values.
pub fn hard_medium_soft_score() -> impl Strategy<Value = HardMediumSoftScore> {
    (any::<i64>(), any::<i64>(), any::<i64>())
        .prop_map(|(h, m, s)| HardMediumSoftScore::of(h, m, s))
}

/// Generates BendableScore with specified hard and soft level counts.
pub fn bendable_score(
    hard_levels: usize,
    soft_levels: usize,
) -> impl Strategy<Value = BendableScore> {
    (
        proptest::collection::vec(any::<i64>(), hard_levels),
        proptest::collection::vec(any::<i64>(), soft_levels),
    )
        .prop_map(|(h, s)| BendableScore::of(h, s))
}

// =============================================================================
// COMMON PROPTEST HELPERS
// =============================================================================

/// Asserts two scores are equal, with better error messages.
#[macro_export]
macro_rules! assert_score_eq {
    ($left:expr, $right:expr) => {
        prop_assert_eq!(
            $left.to_string(),
            $right.to_string(),
            "Scores differ: {} != {}",
            $left,
            $right
        );
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn hard_soft_score_generates_valid(score in hard_soft_score()) {
            // Smoke test: strategy produces parseable scores
            let s = score.to_string();
            prop_assert!(s.contains("hard") || s.contains('/'));
        }

        #[test]
        fn feasible_hard_soft_has_nonnegative_hard(score in feasible_hard_soft_score()) {
            prop_assert!(score.hard_score >= 0);
        }
    }
}
