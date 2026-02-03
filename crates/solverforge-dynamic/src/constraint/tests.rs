//! Tests for dynamic constraints.

use super::*;
use crate::constraint_set::DynamicConstraintSet;
use crate::descriptor::{
    DynamicDescriptor, EntityClassDef, FieldDef, FieldType, ValueRangeDef,
};
use crate::expr::Expr;
use crate::solution::{DynamicEntity, DynamicValue};
use solverforge_core::score::HardSoftScore;
use solverforge_core::{ConstraintRef, ImpactType};
use solverforge_scoring::api::constraint_set::IncrementalConstraint;
use solverforge_scoring::ConstraintSet;

fn make_nqueens_solution(rows: &[i64]) -> DynamicSolution {
    let mut desc = DynamicDescriptor::new();
    desc.add_entity_class(EntityClassDef::new(
        "Queen",
        vec![
            FieldDef::new("column", FieldType::I64),
            FieldDef::planning_variable("row", FieldType::I64, "rows"),
        ],
    ));
    desc.add_value_range("rows", ValueRangeDef::int_range(0, rows.len() as i64));

    let mut solution = DynamicSolution::new(desc);
    for (col, &row) in rows.iter().enumerate() {
        solution.add_entity(
            0,
            DynamicEntity::new(
                col as i64,
                vec![DynamicValue::I64(col as i64), DynamicValue::I64(row)],
            ),
        );
    }
    solution
}

#[test]
fn test_row_conflict_constraint() {
    // Two queens on the same row
    let solution = make_nqueens_solution(&[0, 0, 1, 2]);

    // Build constraint using StreamOp pipeline
    let ops = vec![
        StreamOp::ForEach { class_idx: 0 }, // Queen class
        StreamOp::Join {
            class_idx: 0, // Join with Queen (self-join)
            conditions: vec![Expr::eq(Expr::field(0, 1), Expr::field(1, 1))], // row == row
        },
        StreamOp::DistinctPair {
            ordering_expr: Expr::lt(Expr::field(0, 0), Expr::field(1, 0)), // column < column
        },
        StreamOp::Penalize {
            weight: HardSoftScore::of_hard(1),
        },
    ];

    let mut constraint = build_from_stream_ops(
        ConstraintRef::new("row_conflict"),
        ImpactType::Penalty,
        &ops,
        solution.descriptor().clone(),
    );

    // Initialize to compute matches and score
    let score = constraint.initialize(&solution);
    // Queens at columns 0 and 1 both have row=0, so 1 conflict
    assert_eq!(score, HardSoftScore::of_hard(-1));
}

#[test]
fn test_no_conflicts() {
    // No queens on the same row
    let solution = make_nqueens_solution(&[0, 1, 2, 3]);

    // Build constraint using StreamOp pipeline
    let ops = vec![
        StreamOp::ForEach { class_idx: 0 },
        StreamOp::Join {
            class_idx: 0,
            conditions: vec![Expr::eq(Expr::field(0, 1), Expr::field(1, 1))],
        },
        StreamOp::DistinctPair {
            ordering_expr: Expr::lt(Expr::field(0, 0), Expr::field(1, 0)),
        },
        StreamOp::Penalize {
            weight: HardSoftScore::of_hard(1),
        },
    ];

    let mut constraint = build_from_stream_ops(
        ConstraintRef::new("row_conflict"),
        ImpactType::Penalty,
        &ops,
        solution.descriptor().clone(),
    );

    // Initialize to compute matches and score
    let score = constraint.initialize(&solution);
    assert_eq!(score, HardSoftScore::ZERO);
}

#[test]
fn test_constraint_set() {
    let solution = make_nqueens_solution(&[0, 0, 2, 2]);

    // Build constraint using StreamOp pipeline
    let ops = vec![
        StreamOp::ForEach { class_idx: 0 },
        StreamOp::Join {
            class_idx: 0,
            conditions: vec![Expr::eq(Expr::field(0, 1), Expr::field(1, 1))],
        },
        StreamOp::DistinctPair {
            ordering_expr: Expr::lt(Expr::field(0, 0), Expr::field(1, 0)),
        },
        StreamOp::Penalize {
            weight: HardSoftScore::of_hard(1),
        },
    ];

    let constraint = build_from_stream_ops(
        ConstraintRef::new("row_conflict"),
        ImpactType::Penalty,
        &ops,
        solution.descriptor().clone(),
    );

    let mut constraint_set = DynamicConstraintSet::new();
    constraint_set.add(constraint);

    // Initialize to compute matches and score
    let score = constraint_set.initialize_all(&solution);
    // Two pairs: (0,1) both row=0, (2,3) both row=2
    assert_eq!(score, HardSoftScore::of_hard(-2));
}

#[test]
fn test_bi_self_join_incremental() {
    // Test incremental scoring with on_insert
    let mut solution = make_nqueens_solution(&[0, 1, 2]);

    // Build row conflict constraint
    let ops = vec![
        StreamOp::ForEach { class_idx: 0 },
        StreamOp::Join {
            class_idx: 0,
            conditions: vec![Expr::eq(Expr::field(0, 1), Expr::field(1, 1))],
        },
        StreamOp::DistinctPair {
            ordering_expr: Expr::lt(Expr::field(0, 0), Expr::field(1, 0)),
        },
        StreamOp::Penalize {
            weight: HardSoftScore::of_hard(1),
        },
    ];

    let mut constraint = build_from_stream_ops(
        ConstraintRef::new("row_conflict"),
        ImpactType::Penalty,
        &ops,
        solution.descriptor().clone(),
    );

    // Initialize: no conflicts (rows 0, 1, 2)
    let init_score = constraint.initialize(&solution);
    assert_eq!(init_score, HardSoftScore::ZERO);

    // Insert a new queen at column 3, row 1 (conflicts with queen at column 1)
    solution.add_entity(
        0,
        DynamicEntity::new(
            3,
            vec![DynamicValue::I64(3), DynamicValue::I64(1)],
        ),
    );
    let delta = constraint.on_insert(&solution, 3, 0);
    // New queen (col 3, row 1) conflicts with existing queen (col 1, row 1)
    assert_eq!(delta, HardSoftScore::of_hard(-1));

    // Full evaluation should match incremental result
    let full_score = constraint.evaluate(&solution);
    assert_eq!(full_score, HardSoftScore::of_hard(-1));

    // Insert another queen at column 4, row 1 (conflicts with col 1 and col 3)
    solution.add_entity(
        0,
        DynamicEntity::new(
            4,
            vec![DynamicValue::I64(4), DynamicValue::I64(1)],
        ),
    );
    let delta2 = constraint.on_insert(&solution, 4, 0);
    // New queen (col 4, row 1) conflicts with 2 existing queens (col 1, col 3)
    assert_eq!(delta2, HardSoftScore::of_hard(-2));

    // Full evaluation: 3 total conflicts (1-3, 1-4, 3-4)
    let full_score2 = constraint.evaluate(&solution);
    assert_eq!(full_score2, HardSoftScore::of_hard(-3));

    // Insert queen at column 5, row 0 (conflicts with col 0)
    solution.add_entity(
        0,
        DynamicEntity::new(
            5,
            vec![DynamicValue::I64(5), DynamicValue::I64(0)],
        ),
    );
    let delta3 = constraint.on_insert(&solution, 5, 0);
    // New queen (col 5, row 0) conflicts with 1 existing queen (col 0, row 0)
    assert_eq!(delta3, HardSoftScore::of_hard(-1));

    // Full evaluation: 4 total conflicts (0-5, 1-3, 1-4, 3-4)
    let full_score3 = constraint.evaluate(&solution);
    assert_eq!(full_score3, HardSoftScore::of_hard(-4));
}

#[test]
fn test_tri_self_join_incremental() {
    // Test incremental scoring for tri self-join constraint
    // Scenario: penalize triplets (a, b, c) of numbers where a + b = c
    // This tests the IncrementalTriConstraint wrapper with 3-entity tuples

    let mut descriptor = DynamicDescriptor::new();
    let number_class = descriptor.add_entity_class("Number".to_string(), vec!["value".to_string()]);

    // Initial solution: [1, 2, 3, 4]
    // Initial triplets where a + b = c: (1, 2, 3) only → 1 match
    let mut solution = DynamicSolution::new(descriptor.clone());
    solution.add_entity(number_class, DynamicEntity::new(0, vec![DynamicValue::I64(1)]));
    solution.add_entity(number_class, DynamicEntity::new(1, vec![DynamicValue::I64(2)]));
    solution.add_entity(number_class, DynamicEntity::new(2, vec![DynamicValue::I64(3)]));
    solution.add_entity(number_class, DynamicEntity::new(3, vec![DynamicValue::I64(4)]));

    // Build constraint: penalize triplets where a + b = c
    // ForEach → Join → Join → Filter(a + b = c) → Penalize
    let ops = vec![
        StreamOp::ForEach {
            class_idx: number_class,
        },
        StreamOp::Join {
            class_idx: number_class,
            conditions: vec![],
        },
        StreamOp::Join {
            class_idx: number_class,
            conditions: vec![],
        },
        StreamOp::Filter {
            predicate: Expr::eq(
                Expr::add(Expr::field(0, 0), Expr::field(1, 0)),
                Expr::field(2, 0),
            ),
        },
        StreamOp::Penalize {
            weight: HardSoftScore::of_hard(1),
        },
    ];

    let mut constraint = build_from_stream_ops(
        ConstraintRef::new("sum_triplet"),
        ImpactType::Penalty,
        &ops,
        solution.descriptor().clone(),
    );

    // Initialize
    let init_score = constraint.initialize(&solution);
    // Initial: (1, 2, 3) is the only triplet where a + b = c
    assert_eq!(init_score, HardSoftScore::of_hard(-1));
    let eval_score = constraint.evaluate(&solution);
    assert_eq!(eval_score, HardSoftScore::of_hard(-1));

    // Insert number 5 at index 4
    // New triplets: (1, 4, 5), (2, 3, 5)
    solution.add_entity(
        number_class,
        DynamicEntity::new(4, vec![DynamicValue::I64(5)]),
    );
    let delta1 = constraint.on_insert(&solution, 4, number_class);
    // Delta should be -2 (two new triplets formed)
    assert_eq!(delta1, HardSoftScore::of_hard(-2));

    // Full evaluation: (1, 2, 3), (1, 4, 5), (2, 3, 5) → 3 triplets
    let full_score1 = constraint.evaluate(&solution);
    assert_eq!(full_score1, HardSoftScore::of_hard(-3));

    // Insert number 6 at index 5
    // New triplets: (1, 5, 6), (2, 4, 6)
    solution.add_entity(
        number_class,
        DynamicEntity::new(5, vec![DynamicValue::I64(6)]),
    );
    let delta2 = constraint.on_insert(&solution, 5, number_class);
    // Delta should be -2 (two new triplets formed)
    assert_eq!(delta2, HardSoftScore::of_hard(-2));

    // Full evaluation: (1,2,3), (1,4,5), (2,3,5), (1,5,6), (2,4,6) → 5 triplets
    let full_score2 = constraint.evaluate(&solution);
    assert_eq!(full_score2, HardSoftScore::of_hard(-5));

    // Insert number 7 at index 6
    // New triplets: (1, 6, 7), (2, 5, 7), (3, 4, 7)
    solution.add_entity(
        number_class,
        DynamicEntity::new(6, vec![DynamicValue::I64(7)]),
    );
    let delta3 = constraint.on_insert(&solution, 6, number_class);
    // Delta should be -3 (three new triplets formed)
    assert_eq!(delta3, HardSoftScore::of_hard(-3));

    // Full evaluation: previous 5 + 3 new → 8 triplets total
    let full_score3 = constraint.evaluate(&solution);
    assert_eq!(full_score3, HardSoftScore::of_hard(-8));
}

#[test]
fn test_cross_bi_constraint() {
    // Test cross-join constraint between two different entity classes
    // Scenario: penalize shifts assigned to unavailable employees
    // Shift(employee_id) joins with Employee(id) where Employee.available = false

    let mut descriptor = DynamicDescriptor::new();

    // Define Shift entity class: [shift_id, employee_id]
    let shift_class = descriptor.add_entity_class(
        "Shift".to_string(),
        vec!["shift_id".to_string(), "employee_id".to_string()],
    );

    // Define Employee entity class: [employee_id, available]
    let employee_class = descriptor.add_entity_class(
        "Employee".to_string(),
        vec!["employee_id".to_string(), "available".to_string()],
    );

    // Create solution
    let mut solution = DynamicSolution::new(descriptor.clone());

    // Add employees: [employee_id, available]
    // Employee 1: available = true
    // Employee 2: available = false
    // Employee 3: available = true
    solution.add_entity(
        employee_class,
        DynamicEntity::new(
            0,
            vec![DynamicValue::I64(1), DynamicValue::Bool(true)],
        ),
    );
    solution.add_entity(
        employee_class,
        DynamicEntity::new(
            1,
            vec![DynamicValue::I64(2), DynamicValue::Bool(false)],
        ),
    );
    solution.add_entity(
        employee_class,
        DynamicEntity::new(
            2,
            vec![DynamicValue::I64(3), DynamicValue::Bool(true)],
        ),
    );

    // Add shifts: [shift_id, employee_id]
    // Shift 0 assigned to employee 1 (available) → no penalty
    // Shift 1 assigned to employee 2 (unavailable) → penalty
    solution.add_entity(
        shift_class,
        DynamicEntity::new(
            0,
            vec![DynamicValue::I64(100), DynamicValue::I64(1)],
        ),
    );
    solution.add_entity(
        shift_class,
        DynamicEntity::new(
            1,
            vec![DynamicValue::I64(101), DynamicValue::I64(2)],
        ),
    );

    // Build constraint: penalize shifts assigned to unavailable employees
    // ForEach Shift → Join Employee on shift.employee_id = employee.employee_id
    // → Filter employee.available = false → Penalize
    let ops = vec![
        StreamOp::ForEach {
            class_idx: shift_class,
        },
        StreamOp::Join {
            class_idx: employee_class,
            conditions: vec![Expr::eq(
                Expr::field(0, 1), // shift.employee_id (field index 1)
                Expr::field(1, 0), // employee.employee_id (field index 0)
            )],
        },
        StreamOp::Filter {
            predicate: Expr::eq(
                Expr::field(1, 1), // employee.available (field index 1)
                Expr::literal(DynamicValue::Bool(false)),
            ),
        },
        StreamOp::Penalize {
            weight: HardSoftScore::of_hard(10),
        },
    ];

    let mut constraint = build_from_stream_ops(
        ConstraintRef::new("unavailable_employee"),
        ImpactType::Penalty,
        &ops,
        solution.descriptor().clone(),
    );

    // Initialize
    let init_score = constraint.initialize(&solution);
    // Shift 1 assigned to employee 2 (unavailable) → 1 match → -10
    assert_eq!(init_score, HardSoftScore::of_hard(-10));

    // Verify full evaluation matches
    let eval_score = constraint.evaluate(&solution);
    assert_eq!(eval_score, HardSoftScore::of_hard(-10));

    // Insert a new shift assigned to employee 2 (unavailable)
    solution.add_entity(
        shift_class,
        DynamicEntity::new(
            2,
            vec![DynamicValue::I64(102), DynamicValue::I64(2)],
        ),
    );
    let delta = constraint.on_insert(&solution, 2, shift_class);
    // New shift assigned to unavailable employee → delta = -10
    assert_eq!(delta, HardSoftScore::of_hard(-10));

    // Full evaluation: 2 shifts assigned to unavailable employee → -20
    let full_score = constraint.evaluate(&solution);
    assert_eq!(full_score, HardSoftScore::of_hard(-20));

    // Insert a new shift assigned to employee 3 (available)
    solution.add_entity(
        shift_class,
        DynamicEntity::new(
            3,
            vec![DynamicValue::I64(103), DynamicValue::I64(3)],
        ),
    );
    let delta2 = constraint.on_insert(&solution, 3, shift_class);
    // New shift assigned to available employee → no penalty → delta = 0
    assert_eq!(delta2, HardSoftScore::ZERO);

    // Full evaluation: still 2 shifts assigned to unavailable employee → -20
    let full_score2 = constraint.evaluate(&solution);
    assert_eq!(full_score2, HardSoftScore::of_hard(-20));
}

#[test]
fn test_flattened_bi_constraint() {
    // Test flattened bi-constraint: A joins with B, B contains collection C
    // Scenario: penalize shifts assigned to employees on their unavailable days
    // Shift.employee_id joins with Employee.id, Employee has unavailable_days: Vec<i64>
    // Flatten: Employee → Vec<i64> (unavailable dates)
    // Index: (employee_id, date) for O(1) lookup
    // Lookup: Shift.day → check if (shift.employee_id, shift.day) exists

    let mut descriptor = DynamicDescriptor::new();

    // Define Shift entity class: [shift_id, employee_id, day]
    let shift_class = descriptor.add_entity_class(
        "Shift".to_string(),
        vec![
            "shift_id".to_string(),
            "employee_id".to_string(),
            "day".to_string(),
        ],
    );

    // Define Employee entity class: [employee_id, unavailable_days (Vec<i64>)]
    let employee_class = descriptor.add_entity_class(
        "Employee".to_string(),
        vec![
            "employee_id".to_string(),
            "unavailable_days".to_string(),
        ],
    );

    // Create solution
    let mut solution = DynamicSolution::new(descriptor.clone());

    // Add employees with unavailable days
    // Employee 1: unavailable on days [5, 10, 15]
    // Employee 2: unavailable on days [7, 14]
    solution.add_entity(
        employee_class,
        DynamicEntity::new(
            0,
            vec![
                DynamicValue::I64(1),
                DynamicValue::List(vec![
                    DynamicValue::I64(5),
                    DynamicValue::I64(10),
                    DynamicValue::I64(15),
                ]),
            ],
        ),
    );
    solution.add_entity(
        employee_class,
        DynamicEntity::new(
            1,
            vec![
                DynamicValue::I64(2),
                DynamicValue::List(vec![DynamicValue::I64(7), DynamicValue::I64(14)]),
            ],
        ),
    );

    // Add shifts: [shift_id, employee_id, day]
    // Shift 0: employee 1, day 5 → conflicts (employee 1 unavailable on day 5)
    // Shift 1: employee 1, day 12 → no conflict (employee 1 available on day 12)
    // Shift 2: employee 2, day 7 → conflicts (employee 2 unavailable on day 7)
    solution.add_entity(
        shift_class,
        DynamicEntity::new(
            0,
            vec![
                DynamicValue::I64(100),
                DynamicValue::I64(1),
                DynamicValue::I64(5),
            ],
        ),
    );
    solution.add_entity(
        shift_class,
        DynamicEntity::new(
            1,
            vec![
                DynamicValue::I64(101),
                DynamicValue::I64(1),
                DynamicValue::I64(12),
            ],
        ),
    );
    solution.add_entity(
        shift_class,
        DynamicEntity::new(
            2,
            vec![
                DynamicValue::I64(102),
                DynamicValue::I64(2),
                DynamicValue::I64(7),
            ],
        ),
    );

    // Build constraint: penalize shifts on unavailable days
    // ForEach Shift
    // → Join Employee on shift.employee_id = employee.employee_id
    // → FlattenLast employee.unavailable_days
    // → Filter shift.day = date (where date is from flattened unavailable_days)
    // → Penalize
    let ops = vec![
        StreamOp::ForEach {
            class_idx: shift_class,
        },
        StreamOp::Join {
            class_idx: employee_class,
            conditions: vec![Expr::eq(
                Expr::field(0, 1), // shift.employee_id (field index 1)
                Expr::field(1, 0), // employee.employee_id (field index 0)
            )],
        },
        StreamOp::FlattenLast {
            collection_expr: Expr::field(1, 1), // employee.unavailable_days (field index 1)
        },
        StreamOp::Filter {
            predicate: Expr::eq(
                Expr::field(0, 2), // shift.day (field index 2)
                Expr::param(2),    // flattened date item (Param(2) = C in flattened context)
            ),
        },
        StreamOp::Penalize {
            weight: HardSoftScore::of_hard(10),
        },
    ];

    let mut constraint = build_from_stream_ops(
        ConstraintRef::new("shift_on_unavailable_day"),
        ImpactType::Penalty,
        &ops,
        solution.descriptor().clone(),
    );

    // Initialize
    let init_score = constraint.initialize(&solution);
    // Shift 0 (employee 1, day 5) conflicts → 1 match
    // Shift 2 (employee 2, day 7) conflicts → 1 match
    // Total: 2 conflicts → -20
    assert_eq!(init_score, HardSoftScore::of_hard(-20));

    // Verify full evaluation matches
    let eval_score = constraint.evaluate(&solution);
    assert_eq!(eval_score, HardSoftScore::of_hard(-20));

    // Insert a new shift: employee 1, day 10 (conflicts - employee 1 unavailable on day 10)
    solution.add_entity(
        shift_class,
        DynamicEntity::new(
            3,
            vec![
                DynamicValue::I64(103),
                DynamicValue::I64(1),
                DynamicValue::I64(10),
            ],
        ),
    );
    let delta = constraint.on_insert(&solution, 3, shift_class);
    // New shift conflicts with employee 1's unavailable day 10 → delta = -10
    assert_eq!(delta, HardSoftScore::of_hard(-10));

    // Full evaluation: 3 conflicts → -30
    let full_score = constraint.evaluate(&solution);
    assert_eq!(full_score, HardSoftScore::of_hard(-30));

    // Insert a new shift: employee 1, day 20 (no conflict - employee 1 available on day 20)
    solution.add_entity(
        shift_class,
        DynamicEntity::new(
            4,
            vec![
                DynamicValue::I64(104),
                DynamicValue::I64(1),
                DynamicValue::I64(20),
            ],
        ),
    );
    let delta2 = constraint.on_insert(&solution, 4, shift_class);
    // New shift doesn't conflict → delta = 0
    assert_eq!(delta2, HardSoftScore::ZERO);

    // Full evaluation: still 3 conflicts → -30
    let full_score2 = constraint.evaluate(&solution);
    assert_eq!(full_score2, HardSoftScore::of_hard(-30));

    // Insert a new shift: employee 2, day 14 (conflicts - employee 2 unavailable on day 14)
    solution.add_entity(
        shift_class,
        DynamicEntity::new(
            5,
            vec![
                DynamicValue::I64(105),
                DynamicValue::I64(2),
                DynamicValue::I64(14),
            ],
        ),
    );
    let delta3 = constraint.on_insert(&solution, 5, shift_class);
    // New shift conflicts with employee 2's unavailable day 14 → delta = -10
    assert_eq!(delta3, HardSoftScore::of_hard(-10));

    // Full evaluation: 4 conflicts → -40
    let full_score3 = constraint.evaluate(&solution);
    assert_eq!(full_score3, HardSoftScore::of_hard(-40));
}

/// Test that incremental deltas match full recalculation across multiple constraint types.
///
/// This test creates constraints of different patterns (bi self-join, tri self-join,
/// cross-bi, flattened-bi) and verifies that:
/// 1. After initialize(), the incremental score matches evaluate()
/// 2. After each on_insert(), accumulated delta matches evaluate()
/// 3. After each on_retract(), accumulated delta matches evaluate()
///
/// This provides strong evidence that the incremental indexing is correct and
/// doesn't drift from the true score over multiple operations.
#[test]
fn test_incremental_delta_matches_full_recalculation() {
    // ======================
    // Test 1: Bi Self-Join
    // ======================
    {
        let mut solution = make_nqueens_solution(&[0, 1, 2]);

        let ops = vec![
            StreamOp::ForEach { class_idx: 0 },
            StreamOp::Join {
                class_idx: 0,
                conditions: vec![Expr::eq(Expr::field(0, 1), Expr::field(1, 1))],
            },
            StreamOp::DistinctPair {
                ordering_expr: Expr::lt(Expr::field(0, 0), Expr::field(1, 0)),
            },
            StreamOp::Penalize {
                weight: HardSoftScore::of_hard(1),
            },
        ];

        let mut constraint = build_from_stream_ops(
            ConstraintRef::new("bi_test"),
            ImpactType::Penalty,
            &ops,
            solution.descriptor().clone(),
        );

        // Initialize
        let init_score = constraint.initialize(&solution);
        let full_score = constraint.evaluate(&solution);
        assert_eq!(init_score, full_score, "Bi: Initialize delta != evaluate");

        // Insert entity → conflicts
        solution.add_entity(
            0,
            DynamicEntity::new(3, vec![DynamicValue::I64(3), DynamicValue::I64(1)]),
        );
        let delta1 = constraint.on_insert(&solution, 3, 0);
        let accumulated1 = init_score + delta1;
        let full1 = constraint.evaluate(&solution);
        assert_eq!(
            accumulated1, full1,
            "Bi: After insert, accumulated score != evaluate"
        );

        // Retract entity
        let retracted = solution.entities_by_class(0)[3].clone();
        solution.retract_entity(0, 3);
        let delta2 = constraint.on_retract(&retracted, 3, 0);
        let accumulated2 = accumulated1 + delta2;
        let full2 = constraint.evaluate(&solution);
        assert_eq!(
            accumulated2, full2,
            "Bi: After retract, accumulated score != evaluate"
        );
    }

    // ======================
    // Test 2: Tri Self-Join
    // ======================
    {
        let mut desc = DynamicDescriptor::new();
        desc.add_entity_class(EntityClassDef::new(
            "Number",
            vec![FieldDef::new("value", FieldType::I64)],
        ));

        let mut solution = DynamicSolution::new(desc);
        for val in [1, 2, 3, 4] {
            solution.add_entity(0, DynamicEntity::new(val, vec![DynamicValue::I64(val)]));
        }

        let ops = vec![
            StreamOp::ForEach { class_idx: 0 },
            StreamOp::Join {
                class_idx: 0,
                conditions: vec![],
            },
            StreamOp::Join {
                class_idx: 0,
                conditions: vec![],
            },
            StreamOp::Filter {
                predicate: Expr::eq(
                    Expr::add(Expr::field(0, 0), Expr::field(1, 0)),
                    Expr::field(2, 0),
                ),
            },
            StreamOp::Penalize {
                weight: HardSoftScore::of_hard(1),
            },
        ];

        let mut constraint = build_from_stream_ops(
            ConstraintRef::new("tri_test"),
            ImpactType::Penalty,
            &ops,
            solution.descriptor().clone(),
        );

        // Initialize
        let init_score = constraint.initialize(&solution);
        let full_score = constraint.evaluate(&solution);
        assert_eq!(init_score, full_score, "Tri: Initialize delta != evaluate");

        // Insert 5 → creates (1,4,5) and (2,3,5)
        solution.add_entity(0, DynamicEntity::new(5, vec![DynamicValue::I64(5)]));
        let delta1 = constraint.on_insert(&solution, 4, 0);
        let accumulated1 = init_score + delta1;
        let full1 = constraint.evaluate(&solution);
        assert_eq!(
            accumulated1, full1,
            "Tri: After insert 5, accumulated score != evaluate"
        );

        // Insert 6 → creates (1,5,6) and (2,4,6)
        solution.add_entity(0, DynamicEntity::new(6, vec![DynamicValue::I64(6)]));
        let delta2 = constraint.on_insert(&solution, 5, 0);
        let accumulated2 = accumulated1 + delta2;
        let full2 = constraint.evaluate(&solution);
        assert_eq!(
            accumulated2, full2,
            "Tri: After insert 6, accumulated score != evaluate"
        );
    }

    // ======================
    // Test 3: Cross-Bi
    // ======================
    {
        let mut desc = DynamicDescriptor::new();
        let shift_class = desc.add_entity_class(EntityClassDef::new(
            "Shift",
            vec![
                FieldDef::new("shift_id", FieldType::I64),
                FieldDef::planning_variable("employee_id", FieldType::I64, "employees"),
            ],
        ));
        let employee_class = desc.add_entity_class(EntityClassDef::new(
            "Employee",
            vec![
                FieldDef::new("employee_id", FieldType::I64),
                FieldDef::new("available", FieldType::Bool),
            ],
        ));
        desc.add_value_range("employees", ValueRangeDef::int_range(1, 4));

        let mut solution = DynamicSolution::new(desc);

        // Employees
        solution.add_entity(
            employee_class,
            DynamicEntity::new(0, vec![DynamicValue::I64(1), DynamicValue::Bool(true)]),
        );
        solution.add_entity(
            employee_class,
            DynamicEntity::new(1, vec![DynamicValue::I64(2), DynamicValue::Bool(false)]),
        );

        // Shifts
        solution.add_entity(
            shift_class,
            DynamicEntity::new(0, vec![DynamicValue::I64(100), DynamicValue::I64(1)]),
        );
        solution.add_entity(
            shift_class,
            DynamicEntity::new(1, vec![DynamicValue::I64(101), DynamicValue::I64(2)]),
        );

        let ops = vec![
            StreamOp::ForEach {
                class_idx: shift_class,
            },
            StreamOp::Join {
                class_idx: employee_class,
                conditions: vec![Expr::eq(Expr::field(0, 1), Expr::field(1, 0))],
            },
            StreamOp::Filter {
                predicate: Expr::eq(Expr::field(1, 1), Expr::literal(false)),
            },
            StreamOp::Penalize {
                weight: HardSoftScore::of_hard(10),
            },
        ];

        let mut constraint = build_from_stream_ops(
            ConstraintRef::new("cross_test"),
            ImpactType::Penalty,
            &ops,
            solution.descriptor().clone(),
        );

        // Initialize
        let init_score = constraint.initialize(&solution);
        let full_score = constraint.evaluate(&solution);
        assert_eq!(
            init_score, full_score,
            "Cross: Initialize delta != evaluate"
        );

        // Insert shift assigned to unavailable employee
        solution.add_entity(
            shift_class,
            DynamicEntity::new(2, vec![DynamicValue::I64(102), DynamicValue::I64(2)]),
        );
        let delta1 = constraint.on_insert(&solution, 2, shift_class);
        let accumulated1 = init_score + delta1;
        let full1 = constraint.evaluate(&solution);
        assert_eq!(
            accumulated1, full1,
            "Cross: After insert, accumulated score != evaluate"
        );
    }

    // ======================
    // Test 4: Flattened-Bi
    // ======================
    {
        let mut desc = DynamicDescriptor::new();
        let shift_class = desc.add_entity_class(EntityClassDef::new(
            "Shift",
            vec![
                FieldDef::new("shift_id", FieldType::I64),
                FieldDef::planning_variable("employee_id", FieldType::I64, "employees"),
                FieldDef::new("day", FieldType::I64),
            ],
        ));
        let employee_class = desc.add_entity_class(EntityClassDef::new(
            "Employee",
            vec![
                FieldDef::new("employee_id", FieldType::I64),
                FieldDef::new("unavailable_days", FieldType::VecI64),
            ],
        ));
        desc.add_value_range("employees", ValueRangeDef::int_range(1, 3));

        let mut solution = DynamicSolution::new(desc);

        // Employees with unavailable days
        solution.add_entity(
            employee_class,
            DynamicEntity::new(
                0,
                vec![
                    DynamicValue::I64(1),
                    DynamicValue::VecI64(vec![5, 10, 15]),
                ],
            ),
        );
        solution.add_entity(
            employee_class,
            DynamicEntity::new(1, vec![DynamicValue::I64(2), DynamicValue::VecI64(vec![7])]),
        );

        // Shifts
        solution.add_entity(
            shift_class,
            DynamicEntity::new(
                0,
                vec![
                    DynamicValue::I64(100),
                    DynamicValue::I64(1),
                    DynamicValue::I64(5),
                ],
            ),
        );
        solution.add_entity(
            shift_class,
            DynamicEntity::new(
                1,
                vec![
                    DynamicValue::I64(101),
                    DynamicValue::I64(1),
                    DynamicValue::I64(12),
                ],
            ),
        );

        let ops = vec![
            StreamOp::ForEach {
                class_idx: shift_class,
            },
            StreamOp::Join {
                class_idx: employee_class,
                conditions: vec![Expr::eq(Expr::field(0, 1), Expr::field(1, 0))],
            },
            StreamOp::FlattenLast {
                collection_expr: Expr::field(1, 1),
            },
            StreamOp::Filter {
                predicate: Expr::eq(Expr::field(0, 2), Expr::param(2)),
            },
            StreamOp::Penalize {
                weight: HardSoftScore::of_hard(10),
            },
        ];

        let mut constraint = build_from_stream_ops(
            ConstraintRef::new("flattened_test"),
            ImpactType::Penalty,
            &ops,
            solution.descriptor().clone(),
        );

        // Initialize
        let init_score = constraint.initialize(&solution);
        let full_score = constraint.evaluate(&solution);
        assert_eq!(
            init_score, full_score,
            "Flattened: Initialize delta != evaluate"
        );

        // Insert shift on unavailable day
        solution.add_entity(
            shift_class,
            DynamicEntity::new(
                2,
                vec![
                    DynamicValue::I64(102),
                    DynamicValue::I64(1),
                    DynamicValue::I64(10),
                ],
            ),
        );
        let delta1 = constraint.on_insert(&solution, 2, shift_class);
        let accumulated1 = init_score + delta1;
        let full1 = constraint.evaluate(&solution);
        assert_eq!(
            accumulated1, full1,
            "Flattened: After insert unavailable day, accumulated score != evaluate"
        );

        // Insert shift on available day
        solution.add_entity(
            shift_class,
            DynamicEntity::new(
                3,
                vec![
                    DynamicValue::I64(103),
                    DynamicValue::I64(1),
                    DynamicValue::I64(20),
                ],
            ),
        );
        let delta2 = constraint.on_insert(&solution, 3, shift_class);
        let accumulated2 = accumulated1 + delta2;
        let full2 = constraint.evaluate(&solution);
        assert_eq!(
            accumulated2, full2,
            "Flattened: After insert available day, accumulated score != evaluate"
        );
    }
}
