use super::Score;
use crate::SolverForgeError;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Neg, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HardSoftScore {
    pub hard_score: i64,
    pub soft_score: i64,
}

impl HardSoftScore {
    pub const ZERO: HardSoftScore = HardSoftScore {
        hard_score: 0,
        soft_score: 0,
    };
    pub const ONE_HARD: HardSoftScore = HardSoftScore {
        hard_score: 1,
        soft_score: 0,
    };
    pub const ONE_SOFT: HardSoftScore = HardSoftScore {
        hard_score: 0,
        soft_score: 1,
    };

    pub fn of(hard_score: i64, soft_score: i64) -> Self {
        Self {
            hard_score,
            soft_score,
        }
    }

    pub fn of_hard(hard_score: i64) -> Self {
        Self {
            hard_score,
            soft_score: 0,
        }
    }

    pub fn of_soft(soft_score: i64) -> Self {
        Self {
            hard_score: 0,
            soft_score,
        }
    }

    pub fn parse(text: &str) -> Result<Self, SolverForgeError> {
        let text = text.trim();
        let parts: Vec<&str> = text.split('/').collect();
        if parts.len() != 2 {
            return Err(SolverForgeError::Serialization(format!(
                "Invalid HardSoftScore format: expected 'hard/soft', got '{}'",
                text
            )));
        }

        let hard = parts[0]
            .trim()
            .trim_end_matches("hard")
            .trim()
            .parse::<i64>()
            .map_err(|e| SolverForgeError::Serialization(format!("Invalid hard score: {}", e)))?;

        let soft = parts[1]
            .trim()
            .trim_end_matches("soft")
            .trim()
            .parse::<i64>()
            .map_err(|e| SolverForgeError::Serialization(format!("Invalid soft score: {}", e)))?;

        Ok(Self {
            hard_score: hard,
            soft_score: soft,
        })
    }
}

impl Score for HardSoftScore {
    fn is_feasible(&self) -> bool {
        self.hard_score >= 0
    }

    fn is_solution_initialized(&self) -> bool {
        true
    }

    fn zero() -> Self {
        Self::ZERO
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

impl PartialOrd for HardSoftScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HardSoftScore {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.hard_score.cmp(&other.hard_score) {
            Ordering::Equal => self.soft_score.cmp(&other.soft_score),
            ord => ord,
        }
    }
}

impl fmt::Display for HardSoftScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}hard/{}soft", self.hard_score, self.soft_score)
    }
}

impl Add for HardSoftScore {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Score::add(&self, &other)
    }
}

impl Sub for HardSoftScore {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Score::subtract(&self, &other)
    }
}

impl Neg for HardSoftScore {
    type Output = Self;
    fn neg(self) -> Self {
        Score::negate(&self)
    }
}

impl Default for HardSoftScore {
    fn default() -> Self {
        Self::ZERO
    }
}

// HardMediumSoftScore

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HardMediumSoftScore {
    pub hard_score: i64,
    pub medium_score: i64,
    pub soft_score: i64,
}

impl HardMediumSoftScore {
    pub const ZERO: HardMediumSoftScore = HardMediumSoftScore {
        hard_score: 0,
        medium_score: 0,
        soft_score: 0,
    };
    pub const ONE_HARD: HardMediumSoftScore = HardMediumSoftScore {
        hard_score: 1,
        medium_score: 0,
        soft_score: 0,
    };
    pub const ONE_MEDIUM: HardMediumSoftScore = HardMediumSoftScore {
        hard_score: 0,
        medium_score: 1,
        soft_score: 0,
    };
    pub const ONE_SOFT: HardMediumSoftScore = HardMediumSoftScore {
        hard_score: 0,
        medium_score: 0,
        soft_score: 1,
    };

    pub fn of(hard_score: i64, medium_score: i64, soft_score: i64) -> Self {
        Self {
            hard_score,
            medium_score,
            soft_score,
        }
    }

    pub fn of_hard(hard_score: i64) -> Self {
        Self {
            hard_score,
            medium_score: 0,
            soft_score: 0,
        }
    }

    pub fn of_medium(medium_score: i64) -> Self {
        Self {
            hard_score: 0,
            medium_score,
            soft_score: 0,
        }
    }

    pub fn of_soft(soft_score: i64) -> Self {
        Self {
            hard_score: 0,
            medium_score: 0,
            soft_score,
        }
    }

    pub fn parse(text: &str) -> Result<Self, SolverForgeError> {
        let text = text.trim();
        let parts: Vec<&str> = text.split('/').collect();
        if parts.len() != 3 {
            return Err(SolverForgeError::Serialization(format!(
                "Invalid HardMediumSoftScore format: expected 'hard/medium/soft', got '{}'",
                text
            )));
        }

        let hard = parts[0]
            .trim()
            .trim_end_matches("hard")
            .trim()
            .parse::<i64>()
            .map_err(|e| SolverForgeError::Serialization(format!("Invalid hard score: {}", e)))?;

        let medium = parts[1]
            .trim()
            .trim_end_matches("medium")
            .trim()
            .parse::<i64>()
            .map_err(|e| SolverForgeError::Serialization(format!("Invalid medium score: {}", e)))?;

        let soft = parts[2]
            .trim()
            .trim_end_matches("soft")
            .trim()
            .parse::<i64>()
            .map_err(|e| SolverForgeError::Serialization(format!("Invalid soft score: {}", e)))?;

        Ok(Self {
            hard_score: hard,
            medium_score: medium,
            soft_score: soft,
        })
    }
}

impl Score for HardMediumSoftScore {
    fn is_feasible(&self) -> bool {
        self.hard_score >= 0
    }

    fn is_solution_initialized(&self) -> bool {
        true
    }

    fn zero() -> Self {
        Self::ZERO
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

impl PartialOrd for HardMediumSoftScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HardMediumSoftScore {
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

impl fmt::Display for HardMediumSoftScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}hard/{}medium/{}soft",
            self.hard_score, self.medium_score, self.soft_score
        )
    }
}

impl Add for HardMediumSoftScore {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Score::add(&self, &other)
    }
}

impl Sub for HardMediumSoftScore {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Score::subtract(&self, &other)
    }
}

impl Neg for HardMediumSoftScore {
    type Output = Self;
    fn neg(self) -> Self {
        Score::negate(&self)
    }
}

impl Default for HardMediumSoftScore {
    fn default() -> Self {
        Self::ZERO
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod hard_soft {
        use super::*;

        #[test]
        fn test_of() {
            let score = HardSoftScore::of(-5, 10);
            assert_eq!(score.hard_score, -5);
            assert_eq!(score.soft_score, 10);
        }

        #[test]
        fn test_constants() {
            assert_eq!(HardSoftScore::ZERO, HardSoftScore::of(0, 0));
            assert_eq!(HardSoftScore::ONE_HARD, HardSoftScore::of(1, 0));
            assert_eq!(HardSoftScore::ONE_SOFT, HardSoftScore::of(0, 1));
        }

        #[test]
        fn test_is_feasible() {
            assert!(HardSoftScore::of(0, -100).is_feasible());
            assert!(HardSoftScore::of(10, -50).is_feasible());
            assert!(!HardSoftScore::of(-1, 100).is_feasible());
        }

        #[test]
        fn test_comparison() {
            assert!(HardSoftScore::of(0, 0) > HardSoftScore::of(-1, 100));
            assert!(HardSoftScore::of(0, 10) > HardSoftScore::of(0, 5));
            assert!(HardSoftScore::of(1, 0) > HardSoftScore::of(0, 1000));
        }

        #[test]
        fn test_arithmetic() {
            let a = HardSoftScore::of(-2, 10);
            let b = HardSoftScore::of(-1, 5);

            assert_eq!(a + b, HardSoftScore::of(-3, 15));
            assert_eq!(a - b, HardSoftScore::of(-1, 5));
            assert_eq!(-a, HardSoftScore::of(2, -10));
        }

        #[test]
        fn test_parse() {
            assert_eq!(
                HardSoftScore::parse("0hard/0soft").unwrap(),
                HardSoftScore::ZERO
            );
            assert_eq!(
                HardSoftScore::parse("-5hard/10soft").unwrap(),
                HardSoftScore::of(-5, 10)
            );
            assert_eq!(
                HardSoftScore::parse("-5/-10").unwrap(),
                HardSoftScore::of(-5, -10)
            );
            assert!(HardSoftScore::parse("invalid").is_err());
        }

        #[test]
        fn test_display() {
            assert_eq!(format!("{}", HardSoftScore::of(-5, 10)), "-5hard/10soft");
        }

        #[test]
        fn test_json_serialization() {
            let score = HardSoftScore::of(-5, 10);
            let json = serde_json::to_string(&score).unwrap();
            let parsed: HardSoftScore = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, score);
        }
    }

    mod hard_medium_soft {
        use super::*;

        #[test]
        fn test_of() {
            let score = HardMediumSoftScore::of(-5, 3, 10);
            assert_eq!(score.hard_score, -5);
            assert_eq!(score.medium_score, 3);
            assert_eq!(score.soft_score, 10);
        }

        #[test]
        fn test_constants() {
            assert_eq!(HardMediumSoftScore::ZERO, HardMediumSoftScore::of(0, 0, 0));
            assert_eq!(
                HardMediumSoftScore::ONE_HARD,
                HardMediumSoftScore::of(1, 0, 0)
            );
            assert_eq!(
                HardMediumSoftScore::ONE_MEDIUM,
                HardMediumSoftScore::of(0, 1, 0)
            );
            assert_eq!(
                HardMediumSoftScore::ONE_SOFT,
                HardMediumSoftScore::of(0, 0, 1)
            );
        }

        #[test]
        fn test_is_feasible() {
            assert!(HardMediumSoftScore::of(0, -100, -100).is_feasible());
            assert!(!HardMediumSoftScore::of(-1, 100, 100).is_feasible());
        }

        #[test]
        fn test_comparison() {
            assert!(HardMediumSoftScore::of(0, 0, 0) > HardMediumSoftScore::of(-1, 100, 100));
            assert!(HardMediumSoftScore::of(0, 1, 0) > HardMediumSoftScore::of(0, 0, 100));
            assert!(HardMediumSoftScore::of(0, 0, 10) > HardMediumSoftScore::of(0, 0, 5));
        }

        #[test]
        fn test_arithmetic() {
            let a = HardMediumSoftScore::of(-2, 3, 10);
            let b = HardMediumSoftScore::of(-1, 1, 5);

            assert_eq!(a + b, HardMediumSoftScore::of(-3, 4, 15));
            assert_eq!(a - b, HardMediumSoftScore::of(-1, 2, 5));
            assert_eq!(-a, HardMediumSoftScore::of(2, -3, -10));
        }

        #[test]
        fn test_parse() {
            assert_eq!(
                HardMediumSoftScore::parse("0hard/0medium/0soft").unwrap(),
                HardMediumSoftScore::ZERO
            );
            assert_eq!(
                HardMediumSoftScore::parse("-5hard/3medium/10soft").unwrap(),
                HardMediumSoftScore::of(-5, 3, 10)
            );
            assert!(HardMediumSoftScore::parse("invalid").is_err());
        }

        #[test]
        fn test_display() {
            assert_eq!(
                format!("{}", HardMediumSoftScore::of(-5, 3, 10)),
                "-5hard/3medium/10soft"
            );
        }
    }
}
