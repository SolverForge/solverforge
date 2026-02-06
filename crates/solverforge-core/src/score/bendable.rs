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
    /// Creates a new BendableScore with the given hard and soft score arrays.
    pub const fn of(hard: [i64; H], soft: [i64; S]) -> Self {
        BendableScore { hard, soft }
    }

    /// Creates a zero score.
    pub const fn zero() -> Self {
        BendableScore {
            hard: [0; H],
            soft: [0; S],
        }
    }

    /// Returns the number of hard score levels.
    pub const fn hard_levels_count(&self) -> usize {
        H
    }

    /// Returns the number of soft score levels.
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

    /// Returns all hard scores as a slice.
    pub const fn hard_scores(&self) -> &[i64; H] {
        &self.hard
    }

    /// Returns all soft scores as a slice.
    pub const fn soft_scores(&self) -> &[i64; S] {
        &self.soft
    }

    /// Creates a score with a single hard level penalty at the given index.
    pub const fn one_hard(level: usize) -> Self {
        let mut hard = [0; H];
        hard[level] = 1;
        BendableScore { hard, soft: [0; S] }
    }

    /// Creates a score with a single soft level penalty at the given index.
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

    fn to_level_numbers(&self) -> Vec<i64> {
        let mut levels = Vec::with_capacity(H + S);
        levels.extend_from_slice(&self.hard);
        levels.extend_from_slice(&self.soft);
        levels
    }

    fn from_level_numbers(levels: &[i64]) -> Self {
        assert!(levels.len() >= H + S, "Not enough levels provided");
        let mut hard = [0; H];
        let mut soft = [0; S];
        hard.copy_from_slice(&levels[..H]);
        soft.copy_from_slice(&levels[H..H + S]);
        BendableScore { hard, soft }
    }

    fn multiply(&self, multiplicand: f64) -> Self {
        let mut hard = [0; H];
        let mut soft = [0; S];
        for (i, item) in hard.iter_mut().enumerate().take(H) {
            *item = (self.hard[i] as f64 * multiplicand).round() as i64;
        }
        for (i, item) in soft.iter_mut().enumerate().take(S) {
            *item = (self.soft[i] as f64 * multiplicand).round() as i64;
        }
        BendableScore { hard, soft }
    }

    fn divide(&self, divisor: f64) -> Self {
        let mut hard = [0; H];
        let mut soft = [0; S];
        for (i, item) in hard.iter_mut().enumerate().take(H) {
            *item = (self.hard[i] as f64 / divisor).round() as i64;
        }
        for (i, item) in soft.iter_mut().enumerate().take(S) {
            *item = (self.soft[i] as f64 / divisor).round() as i64;
        }
        BendableScore { hard, soft }
    }

    fn abs(&self) -> Self {
        let mut hard = [0; H];
        let mut soft = [0; S];
        for (i, item) in hard.iter_mut().enumerate().take(H) {
            *item = self.hard[i].abs();
        }
        for (i, item) in soft.iter_mut().enumerate().take(S) {
            *item = self.soft[i].abs();
        }
        BendableScore { hard, soft }
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

impl<const H: usize, const S: usize> Add for BendableScore<H, S> {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let mut hard = [0; H];
        let mut soft = [0; S];
        for (i, item) in hard.iter_mut().enumerate().take(H) {
            *item = self.hard[i] + other.hard[i];
        }
        for (i, item) in soft.iter_mut().enumerate().take(S) {
            *item = self.soft[i] + other.soft[i];
        }
        BendableScore { hard, soft }
    }
}

impl<const H: usize, const S: usize> Sub for BendableScore<H, S> {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        let mut hard = [0; H];
        let mut soft = [0; S];
        for (i, item) in hard.iter_mut().enumerate().take(H) {
            *item = self.hard[i] - other.hard[i];
        }
        for (i, item) in soft.iter_mut().enumerate().take(S) {
            *item = self.soft[i] - other.soft[i];
        }
        BendableScore { hard, soft }
    }
}

impl<const H: usize, const S: usize> Neg for BendableScore<H, S> {
    type Output = Self;

    fn neg(self) -> Self {
        let mut hard = [0; H];
        let mut soft = [0; S];
        for (i, item) in hard.iter_mut().enumerate().take(H) {
            *item = -self.hard[i];
        }
        for (i, item) in soft.iter_mut().enumerate().take(S) {
            *item = -self.soft[i];
        }
        BendableScore { hard, soft }
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
