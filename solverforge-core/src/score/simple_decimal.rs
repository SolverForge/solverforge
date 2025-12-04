use super::Score;
use crate::SolverForgeError;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Neg, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SimpleDecimalScore {
    #[serde(with = "crate::value::decimal_serde")]
    pub score: Decimal,
}

impl SimpleDecimalScore {
    pub fn of(score: Decimal) -> Self {
        Self { score }
    }

    pub fn of_i64(score: i64) -> Self {
        Self {
            score: Decimal::from(score),
        }
    }

    pub fn zero() -> Self {
        Self {
            score: Decimal::ZERO,
        }
    }

    pub fn one() -> Self {
        Self {
            score: Decimal::ONE,
        }
    }

    pub fn parse(text: &str) -> Result<Self, SolverForgeError> {
        let text = text.trim();
        let score = text.parse::<Decimal>().map_err(|e| {
            SolverForgeError::Serialization(format!("Invalid SimpleDecimalScore: {}", e))
        })?;
        Ok(Self { score })
    }
}

impl Score for SimpleDecimalScore {
    fn is_feasible(&self) -> bool {
        self.score >= Decimal::ZERO
    }

    fn is_solution_initialized(&self) -> bool {
        true
    }

    fn zero() -> Self {
        Self {
            score: Decimal::ZERO,
        }
    }

    fn negate(&self) -> Self {
        Self { score: -self.score }
    }

    fn add(&self, other: &Self) -> Self {
        Self {
            score: self.score + other.score,
        }
    }

    fn subtract(&self, other: &Self) -> Self {
        Self {
            score: self.score - other.score,
        }
    }
}

impl PartialOrd for SimpleDecimalScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SimpleDecimalScore {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score.cmp(&other.score)
    }
}

impl fmt::Display for SimpleDecimalScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.score)
    }
}

impl Add for SimpleDecimalScore {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Score::add(&self, &other)
    }
}

impl Sub for SimpleDecimalScore {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Score::subtract(&self, &other)
    }
}

impl Neg for SimpleDecimalScore {
    type Output = Self;
    fn neg(self) -> Self {
        Score::negate(&self)
    }
}

impl Default for SimpleDecimalScore {
    fn default() -> Self {
        Self::zero()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_of() {
        let score = SimpleDecimalScore::of(Decimal::new(314, 2));
        assert_eq!(score.score, Decimal::new(314, 2));
    }

    #[test]
    fn test_of_i64() {
        let score = SimpleDecimalScore::of_i64(42);
        assert_eq!(score.score, Decimal::from(42));
    }

    #[test]
    fn test_is_feasible() {
        assert!(SimpleDecimalScore::of(Decimal::ZERO).is_feasible());
        assert!(SimpleDecimalScore::of(Decimal::new(1, 0)).is_feasible());
        assert!(!SimpleDecimalScore::of(Decimal::new(-1, 0)).is_feasible());
    }

    #[test]
    fn test_arithmetic() {
        let a = SimpleDecimalScore::of(Decimal::new(100, 1));
        let b = SimpleDecimalScore::of(Decimal::new(30, 1));

        assert_eq!((a + b).score, Decimal::new(130, 1));
        assert_eq!((a - b).score, Decimal::new(70, 1));
        assert_eq!((-a).score, Decimal::new(-100, 1));
    }

    #[test]
    fn test_comparison() {
        assert!(
            SimpleDecimalScore::of(Decimal::new(100, 1))
                > SimpleDecimalScore::of(Decimal::new(50, 1))
        );
        assert!(
            SimpleDecimalScore::of(Decimal::new(-50, 1)) < SimpleDecimalScore::of(Decimal::ZERO)
        );
    }

    #[test]
    fn test_parse() {
        assert_eq!(
            SimpleDecimalScore::parse("3.14").unwrap().score,
            Decimal::new(314, 2)
        );
        assert_eq!(
            SimpleDecimalScore::parse("-10").unwrap().score,
            Decimal::new(-10, 0)
        );
        assert!(SimpleDecimalScore::parse("invalid").is_err());
    }

    #[test]
    fn test_display() {
        assert_eq!(
            format!("{}", SimpleDecimalScore::of(Decimal::new(314, 2))),
            "3.14"
        );
    }
}
