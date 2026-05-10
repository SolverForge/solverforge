use std::fmt;
use std::ops::{Add, Neg, Sub};

use solverforge_core::score::{HardSoftScore, Score, ScoreLevel};

use super::collection_extract::vec as source_vec;
use super::{fixed_weight, hard_weight, ConstraintFactory};
use crate::api::constraint_set::{ConstraintSet, IncrementalConstraint};

#[derive(Clone, Debug)]
struct Item {
    active: bool,
}

#[derive(Clone)]
struct Plan {
    items: Vec<Item>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
struct CustomScore {
    hard: i64,
    soft: i64,
}

impl CustomScore {
    const fn of(hard: i64, soft: i64) -> Self {
        Self { hard, soft }
    }
}

impl fmt::Display for CustomScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}hard/{}soft", self.hard, self.soft)
    }
}

impl Add for CustomScore {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::of(self.hard + rhs.hard, self.soft + rhs.soft)
    }
}

impl Sub for CustomScore {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::of(self.hard - rhs.hard, self.soft - rhs.soft)
    }
}

impl Neg for CustomScore {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::of(-self.hard, -self.soft)
    }
}

impl Score for CustomScore {
    fn is_feasible(&self) -> bool {
        self.hard >= 0
    }

    fn zero() -> Self {
        Self::of(0, 0)
    }

    fn levels_count() -> usize {
        2
    }

    fn level_number(&self, index: usize) -> i64 {
        match index {
            0 => self.hard,
            1 => self.soft,
            _ => panic!("CustomScore has 2 levels, got index {index}"),
        }
    }

    fn from_level_numbers(levels: &[i64]) -> Self {
        assert_eq!(levels.len(), 2);
        Self::of(levels[0], levels[1])
    }

    fn multiply(&self, multiplicand: f64) -> Self {
        Self::of(
            (self.hard as f64 * multiplicand).round() as i64,
            (self.soft as f64 * multiplicand).round() as i64,
        )
    }

    fn divide(&self, divisor: f64) -> Self {
        Self::of(
            (self.hard as f64 / divisor).round() as i64,
            (self.soft as f64 / divisor).round() as i64,
        )
    }

    fn abs(&self) -> Self {
        Self::of(self.hard.abs(), self.soft.abs())
    }

    fn to_scalar(&self) -> f64 {
        self.hard as f64 * 1_000_000.0 + self.soft as f64
    }

    fn level_label(index: usize) -> ScoreLevel {
        match index {
            0 => ScoreLevel::Hard,
            1 => ScoreLevel::Soft,
            _ => panic!("CustomScore has 2 levels, got index {index}"),
        }
    }
}

fn sample_plan() -> Plan {
    Plan {
        items: vec![Item { active: true }, Item { active: true }],
    }
}

#[test]
fn fixed_weight_supports_custom_scores() {
    let constraint = ConstraintFactory::<Plan, CustomScore>::new()
        .for_each(source_vec(|plan: &Plan| &plan.items))
        .filter(|item: &Item| item.active)
        .penalize(fixed_weight(CustomScore::of(0, 2)))
        .named("custom fixed score");

    assert_eq!(constraint.evaluate(&sample_plan()), CustomScore::of(0, -4));
    assert!(!constraint.is_hard());
}

#[test]
fn dynamic_hard_soft_weights_are_non_hard_metadata_by_default() {
    let constraint = ConstraintFactory::<Plan, HardSoftScore>::new()
        .for_each(source_vec(|plan: &Plan| &plan.items))
        .penalize(|_item: &Item| HardSoftScore::of_soft(1))
        .named("dynamic soft score");

    assert_eq!(
        constraint.evaluate(&sample_plan()),
        HardSoftScore::of_soft(-2)
    );
    assert!(!constraint.is_hard());
}

#[test]
fn hard_weight_marks_dynamic_weights_as_hard_metadata() {
    let constraint = ConstraintFactory::<Plan, HardSoftScore>::new()
        .for_each(source_vec(|plan: &Plan| &plan.items))
        .penalize(hard_weight(|_item: &Item| HardSoftScore::of_hard(1)))
        .named("explicit hard score");

    assert_eq!(
        constraint.evaluate(&sample_plan()),
        HardSoftScore::of_hard(-2)
    );
    assert!(constraint.is_hard());
}

#[test]
fn fixed_soft_and_dynamic_soft_metadata_deduplicate_without_conflict() {
    let fixed = ConstraintFactory::<Plan, HardSoftScore>::new()
        .for_each(source_vec(|plan: &Plan| &plan.items))
        .penalize(HardSoftScore::of_soft(1))
        .named("same soft score");
    let dynamic = ConstraintFactory::<Plan, HardSoftScore>::new()
        .for_each(source_vec(|plan: &Plan| &plan.items))
        .penalize(|_item: &Item| HardSoftScore::of_soft(1))
        .named("same soft score");

    let constraints = (fixed, dynamic);
    let metadata = constraints.constraint_metadata();

    assert_eq!(metadata.len(), 1);
    assert_eq!(metadata[0].name(), "same soft score");
    assert!(!metadata[0].is_hard);
}
