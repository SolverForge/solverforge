use super::Score;
use crate::SolverForgeError;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Neg, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SimpleScore {
    pub score: i64,
}

impl SimpleScore {
    pub const ZERO: SimpleScore = SimpleScore { score: 0 };
    pub const ONE: SimpleScore = SimpleScore { score: 1 };

    pub fn of(score: i64) -> Self {
        Self { score }
    }

    pub fn parse(text: &str) -> Result<Self, SolverForgeError> {
        let text = text.trim();
        let score = text
            .parse::<i64>()
            .map_err(|e| SolverForgeError::Serialization(format!("Invalid SimpleScore: {}", e)))?;
        Ok(Self { score })
    }
}

impl Score for SimpleScore {
    fn is_feasible(&self) -> bool {
        self.score >= 0
    }

    fn is_solution_initialized(&self) -> bool {
        true
    }

    fn zero() -> Self {
        Self::ZERO
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

impl PartialOrd for SimpleScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SimpleScore {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score.cmp(&other.score)
    }
}

impl fmt::Display for SimpleScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.score)
    }
}

impl Add for SimpleScore {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Score::add(&self, &other)
    }
}

impl Sub for SimpleScore {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Score::subtract(&self, &other)
    }
}

impl Neg for SimpleScore {
    type Output = Self;
    fn neg(self) -> Self {
        Score::negate(&self)
    }
}

impl Default for SimpleScore {
    fn default() -> Self {
        Self::ZERO
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_of() {
        assert_eq!(SimpleScore::of(42).score, 42);
        assert_eq!(SimpleScore::of(-10).score, -10);
    }

    #[test]
    fn test_constants() {
        assert_eq!(SimpleScore::ZERO.score, 0);
        assert_eq!(SimpleScore::ONE.score, 1);
    }

    #[test]
    fn test_is_feasible() {
        assert!(SimpleScore::of(0).is_feasible());
        assert!(SimpleScore::of(10).is_feasible());
        assert!(!SimpleScore::of(-1).is_feasible());
    }

    #[test]
    fn test_arithmetic() {
        let a = SimpleScore::of(10);
        let b = SimpleScore::of(3);

        assert_eq!(a + b, SimpleScore::of(13));
        assert_eq!(a - b, SimpleScore::of(7));
        assert_eq!(-a, SimpleScore::of(-10));
    }

    #[test]
    fn test_comparison() {
        assert!(SimpleScore::of(10) > SimpleScore::of(5));
        assert!(SimpleScore::of(-5) < SimpleScore::of(0));
        assert!(SimpleScore::of(5) == SimpleScore::of(5));
    }

    #[test]
    fn test_parse() {
        assert_eq!(SimpleScore::parse("42").unwrap(), SimpleScore::of(42));
        assert_eq!(SimpleScore::parse("-10").unwrap(), SimpleScore::of(-10));
        assert_eq!(SimpleScore::parse("  0  ").unwrap(), SimpleScore::ZERO);
        assert!(SimpleScore::parse("invalid").is_err());
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", SimpleScore::of(42)), "42");
        assert_eq!(format!("{}", SimpleScore::of(-10)), "-10");
    }

    #[test]
    fn test_json_serialization() {
        let score = SimpleScore::of(42);
        let json = serde_json::to_string(&score).unwrap();
        assert_eq!(json, r#"{"score":42}"#);

        let parsed: SimpleScore = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, score);
    }
}
