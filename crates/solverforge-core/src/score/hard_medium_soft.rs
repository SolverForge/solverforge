//! HardMediumSoftScore - Three-level score with hard, medium, and soft constraints

use std::cmp::Ordering;
use std::fmt;

use super::traits::Score;
use super::ScoreLevel;

/// A score with hard, medium, and soft constraint levels.
///
/// Hard constraints must be satisfied for feasibility.
/// Medium constraints have higher priority than soft constraints.
/// Soft constraints are the lowest priority optimization objectives.
///
/// Comparison order: hard > medium > soft
///
/// # Examples
///
/// ```
/// use solverforge_core::HardMediumSoftScore;
///
/// let score1 = HardMediumSoftScore::of(0, -10, -100);
/// let score2 = HardMediumSoftScore::of(0, -5, -200);
///
/// // Better medium score wins even with worse soft score
/// assert!(score2 > score1);
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct HardMediumSoftScore {
    hard: i64,
    medium: i64,
    soft: i64,
}

impl HardMediumSoftScore {
    /// The zero score.
    pub const ZERO: HardMediumSoftScore = HardMediumSoftScore {
        hard: 0,
        medium: 0,
        soft: 0,
    };

    /// One hard constraint penalty.
    pub const ONE_HARD: HardMediumSoftScore = HardMediumSoftScore {
        hard: 1,
        medium: 0,
        soft: 0,
    };

    /// One medium constraint penalty.
    pub const ONE_MEDIUM: HardMediumSoftScore = HardMediumSoftScore {
        hard: 0,
        medium: 1,
        soft: 0,
    };

    /// One soft constraint penalty.
    pub const ONE_SOFT: HardMediumSoftScore = HardMediumSoftScore {
        hard: 0,
        medium: 0,
        soft: 1,
    };

    /// Creates a new HardMediumSoftScore.
    #[inline]
    pub const fn of(hard: i64, medium: i64, soft: i64) -> Self {
        HardMediumSoftScore { hard, medium, soft }
    }

    /// Creates a score with only a hard component.
    #[inline]
    pub const fn of_hard(hard: i64) -> Self {
        HardMediumSoftScore {
            hard,
            medium: 0,
            soft: 0,
        }
    }

    /// Creates a score with only a medium component.
    #[inline]
    pub const fn of_medium(medium: i64) -> Self {
        HardMediumSoftScore {
            hard: 0,
            medium,
            soft: 0,
        }
    }

    /// Creates a score with only a soft component.
    #[inline]
    pub const fn of_soft(soft: i64) -> Self {
        HardMediumSoftScore {
            hard: 0,
            medium: 0,
            soft,
        }
    }

    /// Returns the hard score component.
    #[inline]
    pub const fn hard(&self) -> i64 {
        self.hard
    }

    /// Returns the medium score component.
    #[inline]
    pub const fn medium(&self) -> i64 {
        self.medium
    }

    /// Returns the soft score component.
    #[inline]
    pub const fn soft(&self) -> i64 {
        self.soft
    }
}

impl Score for HardMediumSoftScore {
    #[inline]
    fn is_feasible(&self) -> bool {
        self.hard >= 0
    }

    #[inline]
    fn zero() -> Self {
        HardMediumSoftScore::ZERO
    }

    #[inline]
    fn levels_count() -> usize {
        3
    }

    fn to_level_numbers(&self) -> Vec<i64> {
        vec![self.hard, self.medium, self.soft]
    }

    fn from_level_numbers(levels: &[i64]) -> Self {
        assert_eq!(
            levels.len(),
            3,
            "HardMediumSoftScore requires exactly 3 levels"
        );
        HardMediumSoftScore::of(levels[0], levels[1], levels[2])
    }

    impl_score_scale!(HardMediumSoftScore { hard, medium, soft } => of);

    fn level_label(index: usize) -> ScoreLevel {
        match index {
            0 => ScoreLevel::Hard,
            1 => ScoreLevel::Medium,
            2 => ScoreLevel::Soft,
            _ => panic!("HardMediumSoftScore has 3 levels, got index {}", index),
        }
    }
}

impl Ord for HardMediumSoftScore {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.hard.cmp(&other.hard) {
            Ordering::Equal => match self.medium.cmp(&other.medium) {
                Ordering::Equal => self.soft.cmp(&other.soft),
                other => other,
            },
            other => other,
        }
    }
}

impl_score_ops!(HardMediumSoftScore { hard, medium, soft } => of);

impl fmt::Debug for HardMediumSoftScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "HardMediumSoftScore({}, {}, {})",
            self.hard, self.medium, self.soft
        )
    }
}

impl fmt::Display for HardMediumSoftScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}hard/{}medium/{}soft",
            self.hard, self.medium, self.soft
        )
    }
}

impl_score_parse!(HardMediumSoftScore { hard => "hard", medium => "medium", soft => "soft" } => of);
