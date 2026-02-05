//! Tests for BalanceConstraint.

use crate::api::constraint_set::IncrementalConstraint;
use crate::constraint::balance::BalanceConstraint;
use crate::stream::filter::TrueFilter;
use solverforge_core::score::SimpleScore;
use solverforge_core::{ConstraintRef, ImpactType};

#[derive(Clone)]
struct Shift {
    employee_id: Option<usize>,
}

#[derive(Clone)]
struct Solution {
    shifts: Vec<Shift>,
}

#[test]
fn test_balance_evaluate_equal_distribution() {
    let constraint = BalanceConstraint::new(
        ConstraintRef::new("", "Balance"),
        ImpactType::Penalty,
        |s: &Solution| &s.shifts,
        TrueFilter,
        |shift: &Shift| shift.employee_id,
        SimpleScore::of(1000), // 1000 per unit std_dev
        false,
    );

    // Equal distribution: 2 shifts each
    let solution = Solution {
        shifts: vec![
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(1),
            },
            Shift {
                employee_id: Some(1),
            },
        ],
    };

    // Mean = 2, all counts = 2, variance = 0, std_dev = 0
    assert_eq!(constraint.evaluate(&solution), SimpleScore::of(0));
}

#[test]
fn test_balance_evaluate_unequal_distribution() {
    let constraint = BalanceConstraint::new(
        ConstraintRef::new("", "Balance"),
        ImpactType::Penalty,
        |s: &Solution| &s.shifts,
        TrueFilter,
        |shift: &Shift| shift.employee_id,
        SimpleScore::of(1000), // 1000 per unit std_dev
        false,
    );

    // Unequal: employee 0 has 3, employee 1 has 1
    let solution = Solution {
        shifts: vec![
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(1),
            },
        ],
    };

    // Mean = 2, variance = ((3-2)² + (1-2)²) / 2 = 1, std_dev = 1.0
    // base_score * std_dev = 1000 * 1.0 = 1000, negated = -1000
    assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-1000));
}

#[test]
fn test_balance_filters_unassigned() {
    let constraint = BalanceConstraint::new(
        ConstraintRef::new("", "Balance"),
        ImpactType::Penalty,
        |s: &Solution| &s.shifts,
        TrueFilter,
        |shift: &Shift| shift.employee_id,
        SimpleScore::of(1000),
        false,
    );

    // Employee 0: 2, Employee 1: 2, plus unassigned (ignored)
    let solution = Solution {
        shifts: vec![
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(1),
            },
            Shift {
                employee_id: Some(1),
            },
            Shift { employee_id: None },
        ],
    };

    // Balanced, std_dev = 0
    assert_eq!(constraint.evaluate(&solution), SimpleScore::of(0));
}

#[test]
fn test_balance_incremental() {
    let mut constraint = BalanceConstraint::new(
        ConstraintRef::new("", "Balance"),
        ImpactType::Penalty,
        |s: &Solution| &s.shifts,
        TrueFilter,
        |shift: &Shift| shift.employee_id,
        SimpleScore::of(1000),
        false,
    );

    let solution = Solution {
        shifts: vec![
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(1),
            },
            Shift {
                employee_id: Some(1),
            },
        ],
    };

    // Initialize with balanced state (std_dev = 0)
    let initial = constraint.initialize(&solution);
    assert_eq!(initial, SimpleScore::of(0));

    // Retract one shift from employee 0
    let delta = constraint.on_retract(&solution, 0, 0);
    // Now: employee 0 has 1, employee 1 has 2
    // Mean = 1.5, variance = (0.25 + 0.25) / 2 = 0.25, std_dev = 0.5
    // Score = -1000 * 0.5 = -500
    assert_eq!(delta, SimpleScore::of(-500));

    // Insert it back
    let delta = constraint.on_insert(&solution, 0, 0);
    // Back to balanced: delta = +500
    assert_eq!(delta, SimpleScore::of(500));
}

#[test]
fn test_balance_empty_solution() {
    let constraint = BalanceConstraint::new(
        ConstraintRef::new("", "Balance"),
        ImpactType::Penalty,
        |s: &Solution| &s.shifts,
        TrueFilter,
        |shift: &Shift| shift.employee_id,
        SimpleScore::of(1000),
        false,
    );

    let solution = Solution { shifts: vec![] };
    assert_eq!(constraint.evaluate(&solution), SimpleScore::of(0));
}

#[test]
fn test_balance_single_employee() {
    let constraint = BalanceConstraint::new(
        ConstraintRef::new("", "Balance"),
        ImpactType::Penalty,
        |s: &Solution| &s.shifts,
        TrueFilter,
        |shift: &Shift| shift.employee_id,
        SimpleScore::of(1000),
        false,
    );

    // Single employee with 5 shifts - no variance possible
    let solution = Solution {
        shifts: vec![
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(0),
            },
        ],
    };

    // With only one group, variance = 0
    assert_eq!(constraint.evaluate(&solution), SimpleScore::of(0));
}

#[test]
fn test_balance_reward() {
    let constraint = BalanceConstraint::new(
        ConstraintRef::new("", "Balance reward"),
        ImpactType::Reward,
        |s: &Solution| &s.shifts,
        TrueFilter,
        |shift: &Shift| shift.employee_id,
        SimpleScore::of(1000),
        false,
    );

    let solution = Solution {
        shifts: vec![
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(1),
            },
        ],
    };

    // std_dev = 1.0, reward = +1000
    assert_eq!(constraint.evaluate(&solution), SimpleScore::of(1000));
}
