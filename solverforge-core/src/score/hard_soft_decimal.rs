use super::Score;
use crate::SolverForgeError;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Neg, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HardSoftDecimalScore {
    #[serde(with = "crate::value::decimal_serde")]
    pub hard_score: Decimal,
    #[serde(with = "crate::value::decimal_serde")]
    pub soft_score: Decimal,
}

impl HardSoftDecimalScore {
    pub fn of(hard_score: Decimal, soft_score: Decimal) -> Self {
        Self {
            hard_score,
            soft_score,
        }
    }

    pub fn of_i64(hard_score: i64, soft_score: i64) -> Self {
        Self {
            hard_score: Decimal::from(hard_score),
            soft_score: Decimal::from(soft_score),
        }
    }

    pub fn of_hard(hard_score: Decimal) -> Self {
        Self {
            hard_score,
            soft_score: Decimal::ZERO,
        }
    }

    pub fn of_soft(soft_score: Decimal) -> Self {
        Self {
            hard_score: Decimal::ZERO,
            soft_score,
        }
    }

    pub fn zero() -> Self {
        Self {
            hard_score: Decimal::ZERO,
            soft_score: Decimal::ZERO,
        }
    }

    pub fn one_hard() -> Self {
        Self {
            hard_score: Decimal::ONE,
            soft_score: Decimal::ZERO,
        }
    }

    pub fn one_soft() -> Self {
        Self {
            hard_score: Decimal::ZERO,
            soft_score: Decimal::ONE,
        }
    }

    pub fn parse(text: &str) -> Result<Self, SolverForgeError> {
        let text = text.trim();
        let parts: Vec<&str> = text.split('/').collect();
        if parts.len() != 2 {
            return Err(SolverForgeError::Serialization(format!(
                "Invalid HardSoftDecimalScore format: expected 'hard/soft', got '{}'",
                text
            )));
        }

        let hard = parts[0]
            .trim()
            .trim_end_matches("hard")
            .trim()
            .parse::<Decimal>()
            .map_err(|e| SolverForgeError::Serialization(format!("Invalid hard score: {}", e)))?;

        let soft = parts[1]
            .trim()
            .trim_end_matches("soft")
            .trim()
            .parse::<Decimal>()
            .map_err(|e| SolverForgeError::Serialization(format!("Invalid soft score: {}", e)))?;

        Ok(Self {
            hard_score: hard,
            soft_score: soft,
        })
    }
}

impl Score for HardSoftDecimalScore {
    fn is_feasible(&self) -> bool {
        self.hard_score >= Decimal::ZERO
    }

    fn is_solution_initialized(&self) -> bool {
        true
    }

    fn zero() -> Self {
        Self {
            hard_score: Decimal::ZERO,
            soft_score: Decimal::ZERO,
        }
    }

    fn negate(&self) -> Self {
        Self {
            hard_score: -self.hard_score,
            soft_score: -self.soft_score,
        }
    }

    fn add(&self, other: &Self) -> Self {
        Self {
            hard_score: self.hard_score + other.hard_score,
            soft_score: self.soft_score + other.soft_score,
        }
    }

    fn subtract(&self, other: &Self) -> Self {
        Self {
            hard_score: self.hard_score - other.hard_score,
            soft_score: self.soft_score - other.soft_score,
        }
    }
}

impl PartialOrd for HardSoftDecimalScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HardSoftDecimalScore {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.hard_score.cmp(&other.hard_score) {
            Ordering::Equal => self.soft_score.cmp(&other.soft_score),
            ord => ord,
        }
    }
}

impl fmt::Display for HardSoftDecimalScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}hard/{}soft", self.hard_score, self.soft_score)
    }
}

impl Add for HardSoftDecimalScore {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Score::add(&self, &other)
    }
}

impl Sub for HardSoftDecimalScore {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Score::subtract(&self, &other)
    }
}

impl Neg for HardSoftDecimalScore {
    type Output = Self;
    fn neg(self) -> Self {
        Score::negate(&self)
    }
}

impl Default for HardSoftDecimalScore {
    fn default() -> Self {
        Self::zero()
    }
}

// HardMediumSoftDecimalScore

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HardMediumSoftDecimalScore {
    #[serde(with = "crate::value::decimal_serde")]
    pub hard_score: Decimal,
    #[serde(with = "crate::value::decimal_serde")]
    pub medium_score: Decimal,
    #[serde(with = "crate::value::decimal_serde")]
    pub soft_score: Decimal,
}

impl HardMediumSoftDecimalScore {
    pub fn of(hard_score: Decimal, medium_score: Decimal, soft_score: Decimal) -> Self {
        Self {
            hard_score,
            medium_score,
            soft_score,
        }
    }

    pub fn of_i64(hard_score: i64, medium_score: i64, soft_score: i64) -> Self {
        Self {
            hard_score: Decimal::from(hard_score),
            medium_score: Decimal::from(medium_score),
            soft_score: Decimal::from(soft_score),
        }
    }

    pub fn zero() -> Self {
        Self {
            hard_score: Decimal::ZERO,
            medium_score: Decimal::ZERO,
            soft_score: Decimal::ZERO,
        }
    }

    pub fn one_hard() -> Self {
        Self {
            hard_score: Decimal::ONE,
            medium_score: Decimal::ZERO,
            soft_score: Decimal::ZERO,
        }
    }

    pub fn one_medium() -> Self {
        Self {
            hard_score: Decimal::ZERO,
            medium_score: Decimal::ONE,
            soft_score: Decimal::ZERO,
        }
    }

    pub fn one_soft() -> Self {
        Self {
            hard_score: Decimal::ZERO,
            medium_score: Decimal::ZERO,
            soft_score: Decimal::ONE,
        }
    }

    pub fn parse(text: &str) -> Result<Self, SolverForgeError> {
        let text = text.trim();
        let parts: Vec<&str> = text.split('/').collect();
        if parts.len() != 3 {
            return Err(SolverForgeError::Serialization(format!(
                "Invalid HardMediumSoftDecimalScore format: expected 'hard/medium/soft', got '{}'",
                text
            )));
        }

        let hard = parts[0]
            .trim()
            .trim_end_matches("hard")
            .trim()
            .parse::<Decimal>()
            .map_err(|e| SolverForgeError::Serialization(format!("Invalid hard score: {}", e)))?;

        let medium = parts[1]
            .trim()
            .trim_end_matches("medium")
            .trim()
            .parse::<Decimal>()
            .map_err(|e| SolverForgeError::Serialization(format!("Invalid medium score: {}", e)))?;

        let soft = parts[2]
            .trim()
            .trim_end_matches("soft")
            .trim()
            .parse::<Decimal>()
            .map_err(|e| SolverForgeError::Serialization(format!("Invalid soft score: {}", e)))?;

        Ok(Self {
            hard_score: hard,
            medium_score: medium,
            soft_score: soft,
        })
    }
}

impl Score for HardMediumSoftDecimalScore {
    fn is_feasible(&self) -> bool {
        self.hard_score >= Decimal::ZERO
    }

    fn is_solution_initialized(&self) -> bool {
        true
    }

    fn zero() -> Self {
        Self {
            hard_score: Decimal::ZERO,
            medium_score: Decimal::ZERO,
            soft_score: Decimal::ZERO,
        }
    }

    fn negate(&self) -> Self {
        Self {
            hard_score: -self.hard_score,
            medium_score: -self.medium_score,
            soft_score: -self.soft_score,
        }
    }

    fn add(&self, other: &Self) -> Self {
        Self {
            hard_score: self.hard_score + other.hard_score,
            medium_score: self.medium_score + other.medium_score,
            soft_score: self.soft_score + other.soft_score,
        }
    }

    fn subtract(&self, other: &Self) -> Self {
        Self {
            hard_score: self.hard_score - other.hard_score,
            medium_score: self.medium_score - other.medium_score,
            soft_score: self.soft_score - other.soft_score,
        }
    }
}

impl PartialOrd for HardMediumSoftDecimalScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HardMediumSoftDecimalScore {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.hard_score.cmp(&other.hard_score) {
            Ordering::Equal => match self.medium_score.cmp(&other.medium_score) {
                Ordering::Equal => self.soft_score.cmp(&other.soft_score),
                ord => ord,
            },
            ord => ord,
        }
    }
}

impl fmt::Display for HardMediumSoftDecimalScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}hard/{}medium/{}soft",
            self.hard_score, self.medium_score, self.soft_score
        )
    }
}

impl Add for HardMediumSoftDecimalScore {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Score::add(&self, &other)
    }
}

impl Sub for HardMediumSoftDecimalScore {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Score::subtract(&self, &other)
    }
}

impl Neg for HardMediumSoftDecimalScore {
    type Output = Self;
    fn neg(self) -> Self {
        Score::negate(&self)
    }
}

impl Default for HardMediumSoftDecimalScore {
    fn default() -> Self {
        Self::zero()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod hard_soft_decimal {
        use super::*;

        #[test]
        fn test_of() {
            let score = HardSoftDecimalScore::of(Decimal::new(-50, 1), Decimal::new(100, 1));
            assert_eq!(score.hard_score, Decimal::new(-50, 1));
            assert_eq!(score.soft_score, Decimal::new(100, 1));
        }

        #[test]
        fn test_is_feasible() {
            assert!(HardSoftDecimalScore::of(Decimal::ZERO, Decimal::new(-100, 0)).is_feasible());
            assert!(
                !HardSoftDecimalScore::of(Decimal::new(-1, 0), Decimal::new(100, 0)).is_feasible()
            );
        }

        #[test]
        fn test_comparison() {
            let a = HardSoftDecimalScore::of(Decimal::ZERO, Decimal::new(10, 0));
            let b = HardSoftDecimalScore::of(Decimal::new(-1, 0), Decimal::new(100, 0));
            assert!(a > b);
        }

        #[test]
        fn test_arithmetic() {
            let a = HardSoftDecimalScore::of(Decimal::new(-2, 0), Decimal::new(10, 0));
            let b = HardSoftDecimalScore::of(Decimal::new(-1, 0), Decimal::new(5, 0));

            let sum = a + b;
            assert_eq!(sum.hard_score, Decimal::new(-3, 0));
            assert_eq!(sum.soft_score, Decimal::new(15, 0));
        }

        #[test]
        fn test_parse() {
            let score = HardSoftDecimalScore::parse("-5.5hard/10.25soft").unwrap();
            assert_eq!(score.hard_score, Decimal::new(-55, 1));
            assert_eq!(score.soft_score, Decimal::new(1025, 2));
        }

        #[test]
        fn test_display() {
            let score = HardSoftDecimalScore::of(Decimal::new(-55, 1), Decimal::new(1025, 2));
            assert_eq!(format!("{}", score), "-5.5hard/10.25soft");
        }
    }

    mod hard_medium_soft_decimal {
        use super::*;

        #[test]
        fn test_of() {
            let score = HardMediumSoftDecimalScore::of(
                Decimal::new(-50, 1),
                Decimal::new(30, 1),
                Decimal::new(100, 1),
            );
            assert_eq!(score.hard_score, Decimal::new(-50, 1));
            assert_eq!(score.medium_score, Decimal::new(30, 1));
            assert_eq!(score.soft_score, Decimal::new(100, 1));
        }

        #[test]
        fn test_is_feasible() {
            assert!(HardMediumSoftDecimalScore::of(
                Decimal::ZERO,
                Decimal::new(-100, 0),
                Decimal::new(-100, 0)
            )
            .is_feasible());
            assert!(!HardMediumSoftDecimalScore::of(
                Decimal::new(-1, 0),
                Decimal::new(100, 0),
                Decimal::new(100, 0)
            )
            .is_feasible());
        }

        #[test]
        fn test_comparison() {
            let a = HardMediumSoftDecimalScore::of(Decimal::ZERO, Decimal::ONE, Decimal::ZERO);
            let b =
                HardMediumSoftDecimalScore::of(Decimal::ZERO, Decimal::ZERO, Decimal::new(100, 0));
            assert!(a > b);
        }

        #[test]
        fn test_parse() {
            let score = HardMediumSoftDecimalScore::parse("-5.5hard/3.0medium/10.25soft").unwrap();
            assert_eq!(score.hard_score, Decimal::new(-55, 1));
            assert_eq!(score.medium_score, Decimal::new(30, 1));
            assert_eq!(score.soft_score, Decimal::new(1025, 2));
        }
    }
}
