//! BendableScore - Compile-time configurable multi-level score
//!
//! Uses const generics for zero-erasure. Level counts are determined at compile time.

use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Neg, Sub};

use super::traits::Score;
use super::ScoreLevel;

/// A score with a configurable number of hard and soft levels.
///
/// Level counts are const generic parameters, enabling Copy and zero heap allocation.
///
/// # Type Parameters
///
/// * `H` - Number of hard score levels
/// * `S` - Number of soft score levels
///
/// # Examples
///
/// ```
/// use solverforge_core::score::{BendableScore, Score};
///
/// // Create a score with 2 hard levels and 3 soft levels
/// let score: BendableScore<2, 3> = BendableScore::of([-1, -2], [-10, -20, -30]);
///
/// assert_eq!(score.hard_levels_count(), 2);
/// assert_eq!(score.soft_levels_count(), 3);
/// assert!(!score.is_feasible());  // Negative hard scores
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct BendableScore<const H: usize, const S: usize> {
    hard: [i64; H],
    soft: [i64; S],
}

impl<const H: usize, const S: usize> BendableScore<H, S> {
    /// Creates a score from its hard levels and soft levels, each ordered from
    /// highest to lowest priority.
    pub const fn of(hard: [i64; H], soft: [i64; S]) -> Self {
        BendableScore { hard, soft }
    }

    /// Returns the score with every level at zero.
    pub const fn zero() -> Self {
        BendableScore {
            hard: [0; H],
            soft: [0; S],
        }
    }

    /// Returns the number of hard levels, `H`.
    pub const fn hard_levels_count(&self) -> usize {
        H
    }

    /// Returns the number of soft levels, `S`.
    pub const fn soft_levels_count(&self) -> usize {
        S
    }

    /// Returns the hard score at the given level.
    ///
    /// # Panics
    /// Panics if the level is out of bounds.
    pub const fn hard_score(&self, level: usize) -> i64 {
        self.hard[level]
    }

    /// Returns the soft score at the given level.
    ///
    /// # Panics
    /// Panics if the level is out of bounds.
    pub const fn soft_score(&self, level: usize) -> i64 {
        self.soft[level]
    }

    /// Returns all hard levels, highest priority first.
    pub const fn hard_scores(&self) -> &[i64; H] {
        &self.hard
    }

    /// Returns all soft levels, highest priority first.
    pub const fn soft_scores(&self) -> &[i64; S] {
        &self.soft
    }

    /// Returns a score of `1` at hard level `level` and zero elsewhere.
    ///
    /// # Panics
    /// Panics if `level >= H`.
    pub const fn one_hard(level: usize) -> Self {
        let mut hard = [0; H];
        hard[level] = 1;
        BendableScore { hard, soft: [0; S] }
    }

    /// Returns a score of `1` at soft level `level` and zero elsewhere.
    ///
    /// # Panics
    /// Panics if `level >= S`.
    pub const fn one_soft(level: usize) -> Self {
        let mut soft = [0; S];
        soft[level] = 1;
        BendableScore { hard: [0; H], soft }
    }
}

impl<const H: usize, const S: usize> Default for BendableScore<H, S> {
    fn default() -> Self {
        Self::zero()
    }
}

impl<const H: usize, const S: usize> Score for BendableScore<H, S> {
    fn is_feasible(&self) -> bool {
        self.hard.iter().all(|&s| s >= 0)
    }

    fn zero() -> Self {
        BendableScore::zero()
    }

    fn levels_count() -> usize {
        H + S
    }

    fn level_number(&self, index: usize) -> i64 {
        if index < H {
            self.hard[index]
        } else if index < H + S {
            self.soft[index - H]
        } else {
            panic!(
                "BendableScore<{}, {}> has {} levels, got index {}",
                H,
                S,
                H + S,
                index
            )
        }
    }

    fn from_level_numbers(levels: &[i64]) -> Self {
        assert!(levels.len() >= H + S, "Not enough levels provided");
        let mut hard = [0; H];
        let mut soft = [0; S];
        hard.copy_from_slice(&levels[..H]);
        soft.copy_from_slice(&levels[H..H + S]);
        BendableScore { hard, soft }
    }

    #[track_caller]
    fn multiply(&self, multiplicand: f64) -> Self {
        let (product, overflow) = self.overflowing_multiply(multiplicand);
        if overflow {
            super::float_overflow(multiplicand);
        }
        product
    }

    #[inline]
    fn overflowing_multiply(&self, multiplicand: f64) -> (Self, bool) {
        let mut overflow = !multiplicand.is_finite();
        let product = BendableScore {
            hard: self
                .hard
                .map(|level| super::float_level(level, multiplicand, &mut overflow)),
            soft: self
                .soft
                .map(|level| super::float_level(level, multiplicand, &mut overflow)),
        };
        (product, overflow)
    }

    #[track_caller]
    fn divide(&self, divisor: f64) -> Self {
        let factor = 1.0 / divisor;
        let mut overflow = !divisor.is_finite() || divisor == 0.0;
        let quotient = BendableScore {
            hard: self
                .hard
                .map(|level| super::float_level(level, factor, &mut overflow)),
            soft: self
                .soft
                .map(|level| super::float_level(level, factor, &mut overflow)),
        };
        if overflow {
            super::float_overflow(divisor);
        }
        quotient
    }

    #[inline]
    fn scale(&self, factor: i64) -> Self {
        BendableScore {
            hard: self.hard.map(|level| super::scale_level(level, factor)),
            soft: self.soft.map(|level| super::scale_level(level, factor)),
        }
    }

    #[inline]
    fn overflowing_scale(&self, factor: i64) -> (Self, bool) {
        let mut overflow = false;
        let mut scale =
            |level: i64| super::track_overflow(level.overflowing_mul(factor), &mut overflow);
        let hard = self.hard.map(&mut scale);
        let soft = self.soft.map(&mut scale);
        (BendableScore { hard, soft }, overflow)
    }

    #[track_caller]
    fn abs(&self) -> Self {
        Self::checked(
            self.levels_overflowing(*self, |level, _, overflow| {
                super::track_overflow(level.overflowing_abs(), overflow)
            }),
            "absolute value",
        )
    }

    #[inline]
    fn overflowing_add(self, other: Self) -> (Self, bool) {
        self.levels_overflowing(other, super::add_level)
    }

    #[inline]
    fn overflowing_sub(self, other: Self) -> (Self, bool) {
        self.levels_overflowing(other, super::sub_level)
    }

    #[inline]
    fn overflowing_neg(self) -> (Self, bool) {
        self.levels_overflowing(self, |level, _, overflow| super::neg_level(level, overflow))
    }

    fn level_label(index: usize) -> ScoreLevel {
        if index < H {
            ScoreLevel::Hard
        } else if index < H + S {
            ScoreLevel::Soft
        } else {
            panic!(
                "BendableScore<{}, {}> has {} levels, got index {}",
                H,
                S,
                H + S,
                index
            )
        }
    }

    #[inline]
    fn to_scalar(&self) -> f64 {
        let total_levels = H + S;
        let mut scalar = 0.0;
        for (i, &v) in self.hard.iter().enumerate() {
            let weight = 10_f64.powi(((total_levels - 1 - i) * 6) as i32);
            scalar += v as f64 * weight;
        }
        for (i, &v) in self.soft.iter().enumerate() {
            let weight = 10_f64.powi(((S - 1 - i) * 6) as i32);
            scalar += v as f64 * weight;
        }
        scalar
    }
}

impl<const H: usize, const S: usize> Ord for BendableScore<H, S> {
    fn cmp(&self, other: &Self) -> Ordering {
        // Compare hard scores first (highest priority first)
        for i in 0..H {
            match self.hard[i].cmp(&other.hard[i]) {
                Ordering::Equal => continue,
                ord => return ord,
            }
        }

        // Then compare soft scores
        for i in 0..S {
            match self.soft[i].cmp(&other.soft[i]) {
                Ordering::Equal => continue,
                ord => return ord,
            }
        }

        Ordering::Equal
    }
}

impl<const H: usize, const S: usize> PartialOrd for BendableScore<H, S> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<const H: usize, const S: usize> BendableScore<H, S> {
    /// Applies `operation` level by level; it records an overflow in its last
    /// argument (see `add_level`).
    #[inline]
    fn levels_overflowing(
        self,
        other: Self,
        operation: impl Fn(i64, i64, &mut bool) -> i64,
    ) -> (Self, bool) {
        let mut overflow = false;
        let mut hard = [0; H];
        for (slot, (left, right)) in hard.iter_mut().zip(self.hard.iter().zip(&other.hard)) {
            *slot = operation(*left, *right, &mut overflow);
        }
        let mut soft = [0; S];
        for (slot, (left, right)) in soft.iter_mut().zip(self.soft.iter().zip(&other.soft)) {
            *slot = operation(*left, *right, &mut overflow);
        }
        (BendableScore { hard, soft }, overflow)
    }

    /// Panics once if the operation overflowed; see `arithmetic_overflow`.
    #[inline]
    #[track_caller]
    fn checked((score, overflow): (Self, bool), name: &'static str) -> Self {
        if overflow {
            super::arithmetic_overflow(name);
        }
        score
    }
}

impl<const H: usize, const S: usize> Add for BendableScore<H, S> {
    type Output = Self;

    #[inline]
    #[track_caller]
    fn add(self, other: Self) -> Self {
        Self::checked(Score::overflowing_add(self, other), "addition")
    }
}

impl<const H: usize, const S: usize> Sub for BendableScore<H, S> {
    type Output = Self;

    #[inline]
    #[track_caller]
    fn sub(self, other: Self) -> Self {
        Self::checked(Score::overflowing_sub(self, other), "subtraction")
    }
}

impl<const H: usize, const S: usize> Neg for BendableScore<H, S> {
    type Output = Self;

    #[inline]
    #[track_caller]
    fn neg(self) -> Self {
        Self::checked(Score::overflowing_neg(self), "negation")
    }
}

impl<const H: usize, const S: usize> fmt::Debug for BendableScore<H, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BendableScore(hard: {:?}, soft: {:?})",
            self.hard, self.soft
        )
    }
}

impl<const H: usize, const S: usize> fmt::Display for BendableScore<H, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hard_str: Vec<String> = self.hard.iter().map(|s| s.to_string()).collect();
        let soft_str: Vec<String> = self.soft.iter().map(|s| s.to_string()).collect();

        write!(
            f,
            "[{}]hard/[{}]soft",
            hard_str.join("/"),
            soft_str.join("/")
        )
    }
}
