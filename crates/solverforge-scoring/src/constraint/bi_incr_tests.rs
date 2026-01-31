//! Tests for zero-erasure incremental bi-constraint.

use super::IncrementalBiConstraint;
use crate::api::constraint_set::{ConstraintSet, IncrementalConstraint};
use solverforge_core::score::SimpleScore;
use solverforge_core::{ConstraintRef, ImpactType};

#[derive(Clone, Hash, PartialEq, Eq, Debug)]
struct Queen {
    row: i64,
    col: i64,
}

#[derive(Clone)]
struct NQueensSolution {
    queens: Vec<Queen>,
}

#[test]
fn test_evaluate_no_conflicts() {
    let constraint = IncrementalBiConstraint::new(
        ConstraintRef::new("", "Row conflict"),
        ImpactType::Penalty,
        |s: &NQueensSolution| s.queens.as_slice(),
        |q: &Queen| q.row, // Key by row for grouping
        |_s: &NQueensSolution, a: &Queen, b: &Queen| a.col < b.col, // Filter: only ordered pairs
        |_a: &Queen, _b: &Queen| SimpleScore::of(1),
        false,
    );

    let solution = NQueensSolution {
        queens: vec![
            Queen { row: 0, col: 0 },
            Queen { row: 1, col: 1 },
            Queen { row: 2, col: 2 },
        ],
    };

    // Each row has exactly one queen, no pairs exist within same row
    assert_eq!(constraint.evaluate(&solution), SimpleScore::of(0));
    assert_eq!(constraint.match_count(&solution), 0);
}

#[test]
fn test_evaluate_with_conflicts() {
    let constraint = IncrementalBiConstraint::new(
        ConstraintRef::new("", "Row conflict"),
        ImpactType::Penalty,
        |s: &NQueensSolution| s.queens.as_slice(),
        |q: &Queen| q.row,
        |_s: &NQueensSolution, a: &Queen, b: &Queen| a.col < b.col,
        |_a: &Queen, _b: &Queen| SimpleScore::of(1),
        false,
    );

    let solution = NQueensSolution {
        queens: vec![
            Queen { row: 0, col: 0 },
            Queen { row: 0, col: 1 }, // Same row as queen 0
            Queen { row: 2, col: 2 },
        ],
    };

    assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-1));
    assert_eq!(constraint.match_count(&solution), 1);
}

#[test]
fn test_incremental_insert() {
    let mut constraint = IncrementalBiConstraint::new(
        ConstraintRef::new("", "Row conflict"),
        ImpactType::Penalty,
        |s: &NQueensSolution| s.queens.as_slice(),
        |q: &Queen| q.row,
        |_s: &NQueensSolution, a: &Queen, b: &Queen| a.col < b.col,
        |_a: &Queen, _b: &Queen| SimpleScore::of(1),
        false,
    );

    let solution = NQueensSolution {
        queens: vec![
            Queen { row: 0, col: 0 },
            Queen { row: 0, col: 1 },
            Queen { row: 2, col: 2 },
        ],
    };

    // Initialize to build index
    constraint.initialize(&solution);
    constraint.reset();

    // Insert first queen - no matches yet
    let delta = constraint.on_insert(&solution, 0, 0);
    assert_eq!(delta, SimpleScore::of(0));

    // Insert second queen - matches with first (same row)
    let delta = constraint.on_insert(&solution, 1, 0);
    assert_eq!(delta, SimpleScore::of(-1));

    // Insert third queen - no new matches (different row)
    let delta = constraint.on_insert(&solution, 2, 0);
    assert_eq!(delta, SimpleScore::of(0));
}

#[test]
fn test_incremental_retract() {
    let mut constraint = IncrementalBiConstraint::new(
        ConstraintRef::new("", "Row conflict"),
        ImpactType::Penalty,
        |s: &NQueensSolution| s.queens.as_slice(),
        |q: &Queen| q.row,
        |_s: &NQueensSolution, a: &Queen, b: &Queen| a.col < b.col,
        |_a: &Queen, _b: &Queen| SimpleScore::of(1),
        false,
    );

    let solution = NQueensSolution {
        queens: vec![Queen { row: 0, col: 0 }, Queen { row: 0, col: 1 }],
    };

    // Initialize and insert both queens
    constraint.initialize(&solution);
    constraint.reset();
    constraint.on_insert(&solution, 0, 0);
    constraint.on_insert(&solution, 1, 0);

    // Retract first queen - removes the match
    let delta = constraint.on_retract(&solution, 0, 0);
    assert_eq!(delta, SimpleScore::of(1)); // Reverses penalty
}

#[test]
fn test_reward_type() {
    let constraint = IncrementalBiConstraint::new(
        ConstraintRef::new("", "Adjacent queens"),
        ImpactType::Reward,
        |s: &NQueensSolution| s.queens.as_slice(),
        |q: &Queen| q.row, // Group by row
        |_s: &NQueensSolution, a: &Queen, b: &Queen| a.col < b.col && (a.col - b.col).abs() == 1,
        |_a: &Queen, _b: &Queen| SimpleScore::of(2),
        false,
    );

    let solution = NQueensSolution {
        queens: vec![
            Queen { row: 0, col: 0 },
            Queen { row: 0, col: 1 }, // Same row, adjacent column
        ],
    };

    assert_eq!(constraint.evaluate(&solution), SimpleScore::of(2));
}

#[test]
fn test_dynamic_weight() {
    let constraint = IncrementalBiConstraint::new(
        ConstraintRef::new("", "Column distance"),
        ImpactType::Penalty,
        |s: &NQueensSolution| s.queens.as_slice(),
        |q: &Queen| q.row,
        |_s: &NQueensSolution, a: &Queen, b: &Queen| a.col < b.col,
        |a: &Queen, b: &Queen| SimpleScore::of((b.col - a.col).abs()),
        false,
    );

    let solution = NQueensSolution {
        queens: vec![
            Queen { row: 0, col: 0 },
            Queen { row: 0, col: 3 }, // Same row, 3 columns apart
        ],
    };

    assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-3));
}

#[test]
fn test_multiple_conflicts() {
    let constraint = IncrementalBiConstraint::new(
        ConstraintRef::new("", "Row conflict"),
        ImpactType::Penalty,
        |s: &NQueensSolution| s.queens.as_slice(),
        |q: &Queen| q.row,
        |_s: &NQueensSolution, a: &Queen, b: &Queen| a.col < b.col,
        |_a: &Queen, _b: &Queen| SimpleScore::of(1),
        false,
    );

    let solution = NQueensSolution {
        queens: vec![
            Queen { row: 0, col: 0 },
            Queen { row: 0, col: 1 }, // Conflicts with queen 0
            Queen { row: 0, col: 2 }, // Conflicts with queens 0 and 1
        ],
    };

    // 3 queens on same row = 3 conflicts: (0,1), (0,2), (1,2)
    assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-3));
    assert_eq!(constraint.match_count(&solution), 3);
}

#[test]
fn test_reset() {
    let mut constraint = IncrementalBiConstraint::new(
        ConstraintRef::new("", "Row conflict"),
        ImpactType::Penalty,
        |s: &NQueensSolution| s.queens.as_slice(),
        |q: &Queen| q.row,
        |_s: &NQueensSolution, a: &Queen, b: &Queen| a.col < b.col,
        |_a: &Queen, _b: &Queen| SimpleScore::of(1),
        false,
    );

    let solution = NQueensSolution {
        queens: vec![Queen { row: 0, col: 0 }, Queen { row: 0, col: 1 }],
    };

    constraint.initialize(&solution);
    constraint.reset();
    constraint.on_insert(&solution, 0, 0);
    constraint.on_insert(&solution, 1, 0);

    constraint.reset();

    // After reset, inserting should produce no delta (no prior state)
    let delta = constraint.on_insert(&solution, 0, 0);
    assert_eq!(delta, SimpleScore::of(0));
}

#[test]
fn test_in_constraint_set() {
    let c1 = IncrementalBiConstraint::new(
        ConstraintRef::new("", "Row conflict"),
        ImpactType::Penalty,
        |s: &NQueensSolution| s.queens.as_slice(),
        |q: &Queen| q.row,
        |_s: &NQueensSolution, a: &Queen, b: &Queen| a.col < b.col,
        |_a: &Queen, _b: &Queen| SimpleScore::of(1),
        false,
    );

    let constraints = (c1,);
    let solution = NQueensSolution {
        queens: vec![
            Queen { row: 0, col: 0 },
            Queen { row: 0, col: 1 },
            Queen { row: 2, col: 2 },
        ],
    };

    assert_eq!(constraints.evaluate_all(&solution), SimpleScore::of(-1));
}

#[test]
fn test_out_of_bounds() {
    let mut constraint = IncrementalBiConstraint::new(
        ConstraintRef::new("", "Row conflict"),
        ImpactType::Penalty,
        |s: &NQueensSolution| s.queens.as_slice(),
        |q: &Queen| q.row,
        |_s: &NQueensSolution, a: &Queen, b: &Queen| a.col < b.col,
        |_a: &Queen, _b: &Queen| SimpleScore::of(1),
        false,
    );

    let solution = NQueensSolution {
        queens: vec![Queen { row: 0, col: 0 }],
    };

    constraint.initialize(&solution);

    // Out of bounds returns zero
    assert_eq!(constraint.on_insert(&solution, 100, 0), SimpleScore::of(0));
    assert_eq!(constraint.on_retract(&solution, 100, 0), SimpleScore::of(0));
}
