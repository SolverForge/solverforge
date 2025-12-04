use super::Score;
use crate::SolverForgeError;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Neg, Sub};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct BendableScore {
    pub hard_scores: Vec<i64>,
    pub soft_scores: Vec<i64>,
}

impl BendableScore {
    pub fn of(hard_scores: Vec<i64>, soft_scores: Vec<i64>) -> Self {
        Self {
            hard_scores,
            soft_scores,
        }
    }

    pub fn zero(hard_levels: usize, soft_levels: usize) -> Self {
        Self {
            hard_scores: vec![0; hard_levels],
            soft_scores: vec![0; soft_levels],
        }
    }

    pub fn of_hard(hard_level: usize, hard_levels: usize, soft_levels: usize, score: i64) -> Self {
        let mut hard_scores = vec![0; hard_levels];
        if hard_level < hard_levels {
            hard_scores[hard_level] = score;
        }
        Self {
            hard_scores,
            soft_scores: vec![0; soft_levels],
        }
    }

    pub fn of_soft(soft_level: usize, hard_levels: usize, soft_levels: usize, score: i64) -> Self {
        let mut soft_scores = vec![0; soft_levels];
        if soft_level < soft_levels {
            soft_scores[soft_level] = score;
        }
        Self {
            hard_scores: vec![0; hard_levels],
            soft_scores,
        }
    }

    pub fn hard_levels_size(&self) -> usize {
        self.hard_scores.len()
    }

    pub fn soft_levels_size(&self) -> usize {
        self.soft_scores.len()
    }

    pub fn parse(text: &str) -> Result<Self, SolverForgeError> {
        let text = text.trim();

        // Format: [h0/h1/.../hn]hard/[s0/s1/.../sm]soft
        let hard_end = text.find("]hard/[").ok_or_else(|| {
            SolverForgeError::Serialization(format!("Invalid BendableScore format: {}", text))
        })?;

        let hard_part = &text[1..hard_end];
        let soft_start = hard_end + 7;
        let soft_end = text.len() - 5;
        let soft_part = &text[soft_start..soft_end];

        let hard_scores = if hard_part.is_empty() {
            vec![]
        } else {
            hard_part
                .split('/')
                .map(|s| s.trim().parse::<i64>())
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| {
                    SolverForgeError::Serialization(format!("Invalid hard score: {}", e))
                })?
        };

        let soft_scores = if soft_part.is_empty() {
            vec![]
        } else {
            soft_part
                .split('/')
                .map(|s| s.trim().parse::<i64>())
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| {
                    SolverForgeError::Serialization(format!("Invalid soft score: {}", e))
                })?
        };

        Ok(Self {
            hard_scores,
            soft_scores,
        })
    }
}

impl Score for BendableScore {
    fn is_feasible(&self) -> bool {
        self.hard_scores.iter().all(|&s| s >= 0)
    }

    fn is_solution_initialized(&self) -> bool {
        true
    }

    fn zero() -> Self {
        Self {
            hard_scores: vec![],
            soft_scores: vec![],
        }
    }

    fn negate(&self) -> Self {
        Self {
            hard_scores: self.hard_scores.iter().map(|&s| -s).collect(),
            soft_scores: self.soft_scores.iter().map(|&s| -s).collect(),
        }
    }

    fn add(&self, other: &Self) -> Self {
        Self {
            hard_scores: self
                .hard_scores
                .iter()
                .zip(other.hard_scores.iter())
                .map(|(&a, &b)| a + b)
                .collect(),
            soft_scores: self
                .soft_scores
                .iter()
                .zip(other.soft_scores.iter())
                .map(|(&a, &b)| a + b)
                .collect(),
        }
    }

    fn subtract(&self, other: &Self) -> Self {
        Self {
            hard_scores: self
                .hard_scores
                .iter()
                .zip(other.hard_scores.iter())
                .map(|(&a, &b)| a - b)
                .collect(),
            soft_scores: self
                .soft_scores
                .iter()
                .zip(other.soft_scores.iter())
                .map(|(&a, &b)| a - b)
                .collect(),
        }
    }
}

impl PartialOrd for BendableScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BendableScore {
    fn cmp(&self, other: &Self) -> Ordering {
        for (a, b) in self.hard_scores.iter().zip(other.hard_scores.iter()) {
            match a.cmp(b) {
                Ordering::Equal => continue,
                ord => return ord,
            }
        }
        for (a, b) in self.soft_scores.iter().zip(other.soft_scores.iter()) {
            match a.cmp(b) {
                Ordering::Equal => continue,
                ord => return ord,
            }
        }
        Ordering::Equal
    }
}

impl fmt::Display for BendableScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hard = self
            .hard_scores
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join("/");
        let soft = self
            .soft_scores
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join("/");
        write!(f, "[{}]hard/[{}]soft", hard, soft)
    }
}

impl Add for BendableScore {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Score::add(&self, &other)
    }
}

impl Sub for BendableScore {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Score::subtract(&self, &other)
    }
}

impl Neg for BendableScore {
    type Output = Self;
    fn neg(self) -> Self {
        Score::negate(&self)
    }
}

// BendableDecimalScore

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct BendableDecimalScore {
    pub hard_scores: Vec<Decimal>,
    pub soft_scores: Vec<Decimal>,
}

impl BendableDecimalScore {
    pub fn of(hard_scores: Vec<Decimal>, soft_scores: Vec<Decimal>) -> Self {
        Self {
            hard_scores,
            soft_scores,
        }
    }

    pub fn zero(hard_levels: usize, soft_levels: usize) -> Self {
        Self {
            hard_scores: vec![Decimal::ZERO; hard_levels],
            soft_scores: vec![Decimal::ZERO; soft_levels],
        }
    }

    pub fn hard_levels_size(&self) -> usize {
        self.hard_scores.len()
    }

    pub fn soft_levels_size(&self) -> usize {
        self.soft_scores.len()
    }

    pub fn parse(text: &str) -> Result<Self, SolverForgeError> {
        let text = text.trim();

        let hard_end = text.find("]hard/[").ok_or_else(|| {
            SolverForgeError::Serialization(format!(
                "Invalid BendableDecimalScore format: {}",
                text
            ))
        })?;

        let hard_part = &text[1..hard_end];
        let soft_start = hard_end + 7;
        let soft_end = text.len() - 5;
        let soft_part = &text[soft_start..soft_end];

        let hard_scores = if hard_part.is_empty() {
            vec![]
        } else {
            hard_part
                .split('/')
                .map(|s| s.trim().parse::<Decimal>())
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| {
                    SolverForgeError::Serialization(format!("Invalid hard score: {}", e))
                })?
        };

        let soft_scores = if soft_part.is_empty() {
            vec![]
        } else {
            soft_part
                .split('/')
                .map(|s| s.trim().parse::<Decimal>())
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| {
                    SolverForgeError::Serialization(format!("Invalid soft score: {}", e))
                })?
        };

        Ok(Self {
            hard_scores,
            soft_scores,
        })
    }
}

impl Score for BendableDecimalScore {
    fn is_feasible(&self) -> bool {
        self.hard_scores.iter().all(|&s| s >= Decimal::ZERO)
    }

    fn is_solution_initialized(&self) -> bool {
        true
    }

    fn zero() -> Self {
        Self {
            hard_scores: vec![],
            soft_scores: vec![],
        }
    }

    fn negate(&self) -> Self {
        Self {
            hard_scores: self.hard_scores.iter().map(|&s| -s).collect(),
            soft_scores: self.soft_scores.iter().map(|&s| -s).collect(),
        }
    }

    fn add(&self, other: &Self) -> Self {
        Self {
            hard_scores: self
                .hard_scores
                .iter()
                .zip(other.hard_scores.iter())
                .map(|(&a, &b)| a + b)
                .collect(),
            soft_scores: self
                .soft_scores
                .iter()
                .zip(other.soft_scores.iter())
                .map(|(&a, &b)| a + b)
                .collect(),
        }
    }

    fn subtract(&self, other: &Self) -> Self {
        Self {
            hard_scores: self
                .hard_scores
                .iter()
                .zip(other.hard_scores.iter())
                .map(|(&a, &b)| a - b)
                .collect(),
            soft_scores: self
                .soft_scores
                .iter()
                .zip(other.soft_scores.iter())
                .map(|(&a, &b)| a - b)
                .collect(),
        }
    }
}

impl PartialOrd for BendableDecimalScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BendableDecimalScore {
    fn cmp(&self, other: &Self) -> Ordering {
        for (a, b) in self.hard_scores.iter().zip(other.hard_scores.iter()) {
            match a.cmp(b) {
                Ordering::Equal => continue,
                ord => return ord,
            }
        }
        for (a, b) in self.soft_scores.iter().zip(other.soft_scores.iter()) {
            match a.cmp(b) {
                Ordering::Equal => continue,
                ord => return ord,
            }
        }
        Ordering::Equal
    }
}

impl fmt::Display for BendableDecimalScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hard = self
            .hard_scores
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join("/");
        let soft = self
            .soft_scores
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join("/");
        write!(f, "[{}]hard/[{}]soft", hard, soft)
    }
}

impl Add for BendableDecimalScore {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Score::add(&self, &other)
    }
}

impl Sub for BendableDecimalScore {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Score::subtract(&self, &other)
    }
}

impl Neg for BendableDecimalScore {
    type Output = Self;
    fn neg(self) -> Self {
        Score::negate(&self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod bendable {
        use super::*;

        #[test]
        fn test_of() {
            let score = BendableScore::of(vec![-1, 0], vec![10, 20, 30]);
            assert_eq!(score.hard_scores, vec![-1, 0]);
            assert_eq!(score.soft_scores, vec![10, 20, 30]);
        }

        #[test]
        fn test_zero() {
            let score = BendableScore::zero(2, 3);
            assert_eq!(score.hard_scores, vec![0, 0]);
            assert_eq!(score.soft_scores, vec![0, 0, 0]);
        }

        #[test]
        fn test_levels_size() {
            let score = BendableScore::of(vec![1, 2], vec![3, 4, 5]);
            assert_eq!(score.hard_levels_size(), 2);
            assert_eq!(score.soft_levels_size(), 3);
        }

        #[test]
        fn test_is_feasible() {
            assert!(BendableScore::of(vec![0, 0], vec![-100]).is_feasible());
            assert!(BendableScore::of(vec![1, 0], vec![-100]).is_feasible());
            assert!(!BendableScore::of(vec![-1, 0], vec![100]).is_feasible());
            assert!(!BendableScore::of(vec![0, -1], vec![100]).is_feasible());
        }

        #[test]
        fn test_comparison() {
            assert!(
                BendableScore::of(vec![1, 0], vec![0]) > BendableScore::of(vec![0, 100], vec![100])
            );
            assert!(
                BendableScore::of(vec![0, 1], vec![0]) > BendableScore::of(vec![0, 0], vec![100])
            );
            assert!(
                BendableScore::of(vec![0, 0], vec![10]) > BendableScore::of(vec![0, 0], vec![5])
            );
        }

        #[test]
        fn test_arithmetic() {
            let a = BendableScore::of(vec![-2, 1], vec![10, 20]);
            let b = BendableScore::of(vec![-1, 1], vec![5, 10]);

            let sum = a.clone() + b.clone();
            assert_eq!(sum.hard_scores, vec![-3, 2]);
            assert_eq!(sum.soft_scores, vec![15, 30]);

            let diff = a.clone() - b;
            assert_eq!(diff.hard_scores, vec![-1, 0]);
            assert_eq!(diff.soft_scores, vec![5, 10]);

            let neg = -a;
            assert_eq!(neg.hard_scores, vec![2, -1]);
            assert_eq!(neg.soft_scores, vec![-10, -20]);
        }

        #[test]
        fn test_parse() {
            let score = BendableScore::parse("[-1/0]hard/[10/20/30]soft").unwrap();
            assert_eq!(score.hard_scores, vec![-1, 0]);
            assert_eq!(score.soft_scores, vec![10, 20, 30]);

            let empty = BendableScore::parse("[]hard/[]soft").unwrap();
            assert!(empty.hard_scores.is_empty());
            assert!(empty.soft_scores.is_empty());
        }

        #[test]
        fn test_display() {
            let score = BendableScore::of(vec![-1, 0], vec![10, 20]);
            assert_eq!(format!("{}", score), "[-1/0]hard/[10/20]soft");
        }

        #[test]
        fn test_json_serialization() {
            let score = BendableScore::of(vec![-1, 0], vec![10, 20]);
            let json = serde_json::to_string(&score).unwrap();
            let parsed: BendableScore = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, score);
        }
    }

    mod bendable_decimal {
        use super::*;

        #[test]
        fn test_of() {
            let score = BendableDecimalScore::of(
                vec![Decimal::new(-10, 1), Decimal::ZERO],
                vec![Decimal::new(100, 1)],
            );
            assert_eq!(score.hard_scores.len(), 2);
            assert_eq!(score.soft_scores.len(), 1);
        }

        #[test]
        fn test_is_feasible() {
            assert!(
                BendableDecimalScore::of(vec![Decimal::ZERO], vec![Decimal::new(-100, 0)])
                    .is_feasible()
            );
            assert!(!BendableDecimalScore::of(vec![Decimal::new(-1, 0)], vec![]).is_feasible());
        }

        #[test]
        fn test_parse() {
            let score = BendableDecimalScore::parse("[-1.5/0]hard/[10.25]soft").unwrap();
            assert_eq!(score.hard_scores, vec![Decimal::new(-15, 1), Decimal::ZERO]);
            assert_eq!(score.soft_scores, vec![Decimal::new(1025, 2)]);
        }

        #[test]
        fn test_display() {
            let score = BendableDecimalScore::of(
                vec![Decimal::new(-15, 1), Decimal::ZERO],
                vec![Decimal::new(1025, 2)],
            );
            assert_eq!(format!("{}", score), "[-1.5/0]hard/[10.25]soft");
        }
    }
}
