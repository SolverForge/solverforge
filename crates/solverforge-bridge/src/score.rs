//! Dynamic score support for binding models.

use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Neg, Sub};

use solverforge_core::score::{ParseableScore, Score, ScoreLevel, ScoreParseError};

/// Declared host-language score family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DynamicScoreFamily {
    Soft,
    HardSoft,
    HardMediumSoft,
}

/// Fixed hard/medium/soft dynamic score used by host-language bindings.
///
/// The static `Score` trait requires a static level count. The bridge therefore
/// uses one concrete three-level representation and maps lower-dimensional
/// families into it. Binding crates must validate that a solve uses one
/// declared family consistently.
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct DynamicScore {
    pub hard: i64,
    pub medium: i64,
    pub soft: i64,
}

impl DynamicScore {
    pub const ZERO: Self = Self {
        hard: 0,
        medium: 0,
        soft: 0,
    };

    pub const fn of(hard: i64, medium: i64, soft: i64) -> Self {
        Self { hard, medium, soft }
    }

    pub const fn soft(soft: i64) -> Self {
        Self::of(0, 0, soft)
    }

    pub const fn hard_soft(hard: i64, soft: i64) -> Self {
        Self::of(hard, 0, soft)
    }

    pub const fn hard_medium_soft(hard: i64, medium: i64, soft: i64) -> Self {
        Self::of(hard, medium, soft)
    }

    pub fn family_levels(self, family: DynamicScoreFamily) -> Vec<i64> {
        match family {
            DynamicScoreFamily::Soft => vec![self.soft],
            DynamicScoreFamily::HardSoft => vec![self.hard, self.soft],
            DynamicScoreFamily::HardMediumSoft => vec![self.hard, self.medium, self.soft],
        }
    }
}

impl fmt::Debug for DynamicScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DynamicScore({}, {}, {})",
            self.hard, self.medium, self.soft
        )
    }
}

impl fmt::Display for DynamicScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}hard/{}medium/{}soft",
            self.hard, self.medium, self.soft
        )
    }
}

impl Ord for DynamicScore {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.hard, self.medium, self.soft).cmp(&(other.hard, other.medium, other.soft))
    }
}

impl PartialOrd for DynamicScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Add for DynamicScore {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::of(
            self.hard + rhs.hard,
            self.medium + rhs.medium,
            self.soft + rhs.soft,
        )
    }
}

impl Sub for DynamicScore {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::of(
            self.hard - rhs.hard,
            self.medium - rhs.medium,
            self.soft - rhs.soft,
        )
    }
}

impl Neg for DynamicScore {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::of(-self.hard, -self.medium, -self.soft)
    }
}

impl Score for DynamicScore {
    fn is_feasible(&self) -> bool {
        self.hard >= 0
    }

    fn zero() -> Self {
        Self::ZERO
    }

    fn levels_count() -> usize {
        3
    }

    fn level_number(&self, index: usize) -> i64 {
        match index {
            0 => self.hard,
            1 => self.medium,
            2 => self.soft,
            _ => panic!("DynamicScore has 3 levels, got index {index}"),
        }
    }

    fn from_level_numbers(levels: &[i64]) -> Self {
        match levels {
            [soft] => Self::soft(*soft),
            [hard, soft] => Self::hard_soft(*hard, *soft),
            [hard, medium, soft] => Self::hard_medium_soft(*hard, *medium, *soft),
            _ => panic!("DynamicScore requires 1, 2, or 3 levels"),
        }
    }

    fn multiply(&self, multiplicand: f64) -> Self {
        Self::of(
            (self.hard as f64 * multiplicand).round() as i64,
            (self.medium as f64 * multiplicand).round() as i64,
            (self.soft as f64 * multiplicand).round() as i64,
        )
    }

    fn divide(&self, divisor: f64) -> Self {
        self.multiply(1.0 / divisor)
    }

    fn abs(&self) -> Self {
        Self::of(self.hard.abs(), self.medium.abs(), self.soft.abs())
    }

    fn to_scalar(&self) -> f64 {
        self.hard as f64 * 1_000_000.0 + self.medium as f64 * 1_000.0 + self.soft as f64
    }

    fn level_label(index: usize) -> ScoreLevel {
        match index {
            0 => ScoreLevel::Hard,
            1 => ScoreLevel::Medium,
            2 => ScoreLevel::Soft,
            _ => panic!("DynamicScore has 3 levels, got index {index}"),
        }
    }
}

impl ParseableScore for DynamicScore {
    fn parse(s: &str) -> Result<Self, ScoreParseError> {
        let mut hard = 0;
        let mut medium = 0;
        let mut soft = 0;
        for part in s.split('/') {
            if let Some(raw) = part.strip_suffix("hard") {
                hard = raw.parse().map_err(|_| ScoreParseError {
                    message: format!("invalid hard score `{raw}`"),
                })?;
            } else if let Some(raw) = part.strip_suffix("medium") {
                medium = raw.parse().map_err(|_| ScoreParseError {
                    message: format!("invalid medium score `{raw}`"),
                })?;
            } else if let Some(raw) = part.strip_suffix("soft") {
                soft = raw.parse().map_err(|_| ScoreParseError {
                    message: format!("invalid soft score `{raw}`"),
                })?;
            } else if let Ok(value) = part.parse::<i64>() {
                soft = value;
            } else {
                return Err(ScoreParseError {
                    message: format!("invalid dynamic score `{s}`"),
                });
            }
        }
        Ok(Self::of(hard, medium, soft))
    }

    fn to_string_repr(&self) -> String {
        self.to_string()
    }
}
