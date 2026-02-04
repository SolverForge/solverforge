//! Tests for dynamic constraints.

use super::*;
use crate::constraint_set::DynamicConstraintSet;
use crate::descriptor::{DynamicDescriptor, EntityClassDef, FieldDef, FieldType, ValueRangeDef};
use crate::expr::Expr;
use crate::solution::{DynamicEntity, DynamicSolution, DynamicValue};
use solverforge_core::score::HardSoftScore;
use solverforge_core::{ConstraintRef, ImpactType};
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
        ConstraintRef::new("", "row_conflict"),
        ImpactType::Penalty,
        &ops,
        solution.descriptor.clone(),
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
        ConstraintRef::new("", "row_conflict"),
        ImpactType::Penalty,
        &ops,
        solution.descriptor.clone(),
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
        ConstraintRef::new("", "row_conflict"),
        ImpactType::Penalty,
        &ops,
        solution.descriptor.clone(),
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
        ConstraintRef::new("", "row_conflict"),
        ImpactType::Penalty,
        &ops,
        solution.descriptor.clone(),
    );

    // Initialize: no conflicts (rows 0, 1, 2)
    let init_score = constraint.initialize(&solution);
    assert_eq!(init_score, HardSoftScore::ZERO);

    // Insert a new queen at column 3, row 1 (conflicts with queen at column 1)
    solution.add_entity(
        0,
        DynamicEntity::new(3, vec![DynamicValue::I64(3), DynamicValue::I64(1)]),
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
        DynamicEntity::new(4, vec![DynamicValue::I64(4), DynamicValue::I64(1)]),
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
        DynamicEntity::new(5, vec![DynamicValue::I64(5), DynamicValue::I64(0)]),
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
    descriptor.add_entity_class(EntityClassDef::new(
        "Number",
        vec![FieldDef::new("value", FieldType::I64)],
    ));
    let number_class = 0; // First entity class index

    // Initial solution: [1, 2, 3, 4]
    // Initial triplets where a + b = c: (1, 2, 3) and (1, 3, 4) → 2 matches
    let mut solution = DynamicSolution::new(descriptor.clone());
    solution.add_entity(
        number_class,
        DynamicEntity::new(0, vec![DynamicValue::I64(1)]),
    );
    solution.add_entity(
        number_class,
        DynamicEntity::new(1, vec![DynamicValue::I64(2)]),
    );
    solution.add_entity(
        number_class,
        DynamicEntity::new(2, vec![DynamicValue::I64(3)]),
    );
    solution.add_entity(
        number_class,
        DynamicEntity::new(3, vec![DynamicValue::I64(4)]),
    );

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
        ConstraintRef::new("", "row_conflict"),
        ImpactType::Penalty,
        &ops,
        solution.descriptor.clone(),
    );

    // Initialize
    let init_score = constraint.initialize(&solution);
    // Initial: (1, 2, 3) and (1, 3, 4) are triplets where a + b = c → 2 matches
    assert_eq!(init_score, HardSoftScore::of_hard(-2));
    let eval_score = constraint.evaluate(&solution);
    assert_eq!(eval_score, HardSoftScore::of_hard(-2));

    // Insert number 5 at index 4
    // New triplets involving 5: (1, 4, 5), (2, 3, 5)
    // Existing: (1, 2, 3), (1, 3, 4)
    solution.add_entity(
        number_class,
        DynamicEntity::new(4, vec![DynamicValue::I64(5)]),
    );
    let delta1 = constraint.on_insert(&solution, 4, number_class);
    // Delta should be -2 (two new triplets formed)
    assert_eq!(delta1, HardSoftScore::of_hard(-2));

    // Full evaluation: (1, 2, 3), (1, 3, 4), (1, 4, 5), (2, 3, 5) → 4 triplets
    let full_score1 = constraint.evaluate(&solution);
    assert_eq!(full_score1, HardSoftScore::of_hard(-4));

    // Insert number 6 at index 5
    // New triplets involving 6: (1, 5, 6), (2, 4, 6)
    solution.add_entity(
        number_class,
        DynamicEntity::new(5, vec![DynamicValue::I64(6)]),
    );
    let delta2 = constraint.on_insert(&solution, 5, number_class);
    // Delta should be -2 (two new triplets formed)
    assert_eq!(delta2, HardSoftScore::of_hard(-2));

    // Full evaluation: 4 previous + 2 new → 6 triplets
    let full_score2 = constraint.evaluate(&solution);
    assert_eq!(full_score2, HardSoftScore::of_hard(-6));

    // Insert number 7 at index 6
    // New triplets involving 7: (1, 6, 7), (2, 5, 7), (3, 4, 7)
    solution.add_entity(
        number_class,
        DynamicEntity::new(6, vec![DynamicValue::I64(7)]),
    );
    let delta3 = constraint.on_insert(&solution, 6, number_class);
    // Delta should be -3 (three new triplets formed)
    assert_eq!(delta3, HardSoftScore::of_hard(-3));

    // Full evaluation: 6 previous + 3 new → 9 triplets total
    let full_score3 = constraint.evaluate(&solution);
    assert_eq!(full_score3, HardSoftScore::of_hard(-9));
}

#[test]
fn test_cross_bi_constraint() {
    // Test cross-join constraint between two different entity classes
    // Scenario: penalize shifts assigned to unavailable employees
    // Shift(employee_id) joins with Employee(id) where Employee.available = false

    let mut descriptor = DynamicDescriptor::new();

    // Define Shift entity class: [shift_id, employee_id]
    descriptor.add_entity_class(EntityClassDef::new(
        "Shift",
        vec![
            FieldDef::new("shift_id", FieldType::I64),
            FieldDef::new("employee_id", FieldType::I64),
        ],
    ));
    let shift_class = 0;

    // Define Employee entity class: [employee_id, available]
    descriptor.add_entity_class(EntityClassDef::new(
        "Employee",
        vec![
            FieldDef::new("employee_id", FieldType::I64),
            FieldDef::new("available", FieldType::Bool),
        ],
    ));
    let employee_class = 1;

    // Create solution
    let mut solution = DynamicSolution::new(descriptor.clone());

    // Add employees: [employee_id, available]
    // Entity IDs must be globally unique across all entity classes for id_to_location lookup
    // Employee 1: available = true (entity id 100)
    // Employee 2: available = false (entity id 101)
    // Employee 3: available = true (entity id 102)
    solution.add_entity(
        employee_class,
        DynamicEntity::new(100, vec![DynamicValue::I64(1), DynamicValue::Bool(true)]),
    );
    solution.add_entity(
        employee_class,
        DynamicEntity::new(101, vec![DynamicValue::I64(2), DynamicValue::Bool(false)]),
    );
    solution.add_entity(
        employee_class,
        DynamicEntity::new(102, vec![DynamicValue::I64(3), DynamicValue::Bool(true)]),
    );

    // Add shifts: [shift_id, employee_id]
    // Shift 0 assigned to employee 1 (available) → no penalty (entity id 200)
    // Shift 1 assigned to employee 2 (unavailable) → penalty (entity id 201)
    solution.add_entity(
        shift_class,
        DynamicEntity::new(200, vec![DynamicValue::I64(100), DynamicValue::I64(1)]),
    );
    solution.add_entity(
        shift_class,
        DynamicEntity::new(201, vec![DynamicValue::I64(101), DynamicValue::I64(2)]),
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
        ConstraintRef::new("", "unavailable_employee"),
        ImpactType::Penalty,
        &ops,
        solution.descriptor.clone(),
    );

    // Initialize
    let init_score = constraint.initialize(&solution);
    // Shift 1 assigned to employee 2 (unavailable) → 1 match → -10
    assert_eq!(init_score, HardSoftScore::of_hard(-10));

    // Verify full evaluation matches
    let eval_score = constraint.evaluate(&solution);
    assert_eq!(eval_score, HardSoftScore::of_hard(-10));

    // Insert a new shift assigned to employee 2 (unavailable) (entity id 202, entity index 2)
    solution.add_entity(
        shift_class,
        DynamicEntity::new(202, vec![DynamicValue::I64(102), DynamicValue::I64(2)]),
    );
    let delta = constraint.on_insert(&solution, 2, shift_class);
    // New shift assigned to unavailable employee → delta = -10
    assert_eq!(delta, HardSoftScore::of_hard(-10));

    // Full evaluation: 2 shifts assigned to unavailable employee → -20
    let full_score = constraint.evaluate(&solution);
    assert_eq!(full_score, HardSoftScore::of_hard(-20));

    // Insert a new shift assigned to employee 3 (available) (entity id 203, entity index 3)
    solution.add_entity(
        shift_class,
        DynamicEntity::new(203, vec![DynamicValue::I64(103), DynamicValue::I64(3)]),
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
    descriptor.add_entity_class(EntityClassDef::new(
        "Shift",
        vec![
            FieldDef::new("shift_id", FieldType::I64),
            FieldDef::new("employee_id", FieldType::I64),
            FieldDef::new("day", FieldType::I64),
        ],
    ));
    let shift_class = 0;

    // Define Employee entity class: [employee_id, unavailable_days (List)]
    descriptor.add_entity_class(EntityClassDef::new(
        "Employee",
        vec![
            FieldDef::new("employee_id", FieldType::I64),
            FieldDef::new("unavailable_days", FieldType::List),
        ],
    ));
    let employee_class = 1;

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
            set_expr: Expr::field(1, 1), // employee.unavailable_days (field index 1)
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
        ConstraintRef::new("", "shift_on_unavailable_day"),
        ImpactType::Penalty,
        &ops,
        solution.descriptor.clone(),
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
            ConstraintRef::new("", "bi_test"),
            ImpactType::Penalty,
            &ops,
            solution.descriptor.clone(),
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

        // Retract entity - call on_retract BEFORE removing from solution
        let delta2 = constraint.on_retract(&solution, 3, 0);
        solution.entities[0].remove(3);
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
            ConstraintRef::new("", "tri_test"),
            ImpactType::Penalty,
            &ops,
            solution.descriptor.clone(),
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
        // Shift is class 0
        desc.add_entity_class(EntityClassDef::new(
            "Shift",
            vec![
                FieldDef::new("shift_id", FieldType::I64),
                FieldDef::planning_variable("employee_id", FieldType::I64, "employees"),
            ],
        ));
        let shift_class: usize = 0;
        // Employee is class 1
        desc.add_entity_class(EntityClassDef::new(
            "Employee",
            vec![
                FieldDef::new("employee_id", FieldType::I64),
                FieldDef::new("available", FieldType::Bool),
            ],
        ));
        let employee_class: usize = 1;
        desc.add_value_range("employees", ValueRangeDef::int_range(1, 4));

        let mut solution = DynamicSolution::new(desc);

        // Employees (entity IDs must be globally unique for id_to_location lookup)
        solution.add_entity(
            employee_class,
            DynamicEntity::new(100, vec![DynamicValue::I64(1), DynamicValue::Bool(true)]),
        );
        solution.add_entity(
            employee_class,
            DynamicEntity::new(101, vec![DynamicValue::I64(2), DynamicValue::Bool(false)]),
        );

        // Shifts (entity IDs must be globally unique for id_to_location lookup)
        solution.add_entity(
            shift_class,
            DynamicEntity::new(200, vec![DynamicValue::I64(100), DynamicValue::I64(1)]),
        );
        solution.add_entity(
            shift_class,
            DynamicEntity::new(201, vec![DynamicValue::I64(101), DynamicValue::I64(2)]),
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
                predicate: Expr::eq(Expr::field(1, 1), Expr::literal(DynamicValue::Bool(false))),
            },
            StreamOp::Penalize {
                weight: HardSoftScore::of_hard(10),
            },
        ];

        let mut constraint = build_from_stream_ops(
            ConstraintRef::new("", "cross_test"),
            ImpactType::Penalty,
            &ops,
            solution.descriptor.clone(),
        );

        // Initialize
        let init_score = constraint.initialize(&solution);
        let full_score = constraint.evaluate(&solution);
        assert_eq!(
            init_score, full_score,
            "Cross: Initialize delta != evaluate"
        );

        // Insert shift assigned to unavailable employee (entity id 202, entity index 2)
        solution.add_entity(
            shift_class,
            DynamicEntity::new(202, vec![DynamicValue::I64(102), DynamicValue::I64(2)]),
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
        // Shift is class 0
        desc.add_entity_class(EntityClassDef::new(
            "Shift",
            vec![
                FieldDef::new("shift_id", FieldType::I64),
                FieldDef::planning_variable("employee_id", FieldType::I64, "employees"),
                FieldDef::new("day", FieldType::I64),
            ],
        ));
        let shift_class: usize = 0;
        // Employee is class 1
        desc.add_entity_class(EntityClassDef::new(
            "Employee",
            vec![
                FieldDef::new("employee_id", FieldType::I64),
                FieldDef::new("unavailable_days", FieldType::List),
            ],
        ));
        let employee_class: usize = 1;
        desc.add_value_range("employees", ValueRangeDef::int_range(1, 3));

        let mut solution = DynamicSolution::new(desc);

        // Employees with unavailable days
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
                    DynamicValue::List(vec![DynamicValue::I64(7)]),
                ],
            ),
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
                set_expr: Expr::field(1, 1),
            },
            StreamOp::Filter {
                predicate: Expr::eq(Expr::field(0, 2), Expr::param(2)),
            },
            StreamOp::Penalize {
                weight: HardSoftScore::of_hard(10),
            },
        ];

        let mut constraint = build_from_stream_ops(
            ConstraintRef::new("", "flattened_test"),
            ImpactType::Penalty,
            &ops,
            solution.descriptor.clone(),
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

/// Test cross-class constraint with same-named fields at different indices.
///
/// This verifies the fix for the entity assignment bug where expressions like
/// `A.shift_id == B.shift_id` would incorrectly use the same field index for
/// both classes when the field had the same name in both classes.
///
/// Scenario:
/// - Employee class (class 0): fields = [id, name, assigned_shift_id]
///   → assigned_shift_id is at field index 2
/// - Shift class (class 1): fields = [assigned_shift_id, start_time]
///   → assigned_shift_id is at field index 0
///
/// A constraint joining Employee with Shift on assigned_shift_id should:
/// - Use Employee's field index 2 for parameter A
/// - Use Shift's field index 0 for parameter B
///
/// Before the fix, both would incorrectly resolve to field index 0 because
/// the code searched all classes for the first match.
#[test]
fn test_cross_class_same_named_field_constraint() {
    // Build descriptor with two classes having same-named fields at different indices
    let mut descriptor = DynamicDescriptor::new();

    // Employee class (class 0): [id, name, assigned_shift_id]
    // assigned_shift_id is at index 2
    descriptor.add_entity_class(EntityClassDef::new(
        "Employee",
        vec![
            FieldDef::new("id", FieldType::I64),
            FieldDef::new("name", FieldType::String),
            FieldDef::new("assigned_shift_id", FieldType::I64), // index 2
        ],
    ));
    let employee_class = 0;

    // Shift class (class 1): [assigned_shift_id, start_time]
    // assigned_shift_id is at index 0
    descriptor.add_entity_class(EntityClassDef::new(
        "Shift",
        vec![
            FieldDef::new("assigned_shift_id", FieldType::I64), // index 0
            FieldDef::new("start_time", FieldType::I64),
        ],
    ));
    let shift_class = 1;

    // Create solution
    let mut solution = DynamicSolution::new(descriptor.clone());

    // Add employees with assigned_shift_id values
    // Employee 0: id=1, name="Alice", assigned_shift_id=100
    // Employee 1: id=2, name="Bob", assigned_shift_id=200
    // Employee 2: id=3, name="Charlie", assigned_shift_id=100 (same as Alice)
    solution.add_entity(
        employee_class,
        DynamicEntity::new(
            100, // entity id (globally unique)
            vec![
                DynamicValue::I64(1),                 // id
                DynamicValue::String("Alice".into()), // name
                DynamicValue::I64(100),               // assigned_shift_id (field 2)
            ],
        ),
    );
    solution.add_entity(
        employee_class,
        DynamicEntity::new(
            101,
            vec![
                DynamicValue::I64(2),
                DynamicValue::String("Bob".into()),
                DynamicValue::I64(200),
            ],
        ),
    );
    solution.add_entity(
        employee_class,
        DynamicEntity::new(
            102,
            vec![
                DynamicValue::I64(3),
                DynamicValue::String("Charlie".into()),
                DynamicValue::I64(100),
            ],
        ),
    );

    // Add shifts with assigned_shift_id values
    // Shift 0: assigned_shift_id=100, start_time=9
    // Shift 1: assigned_shift_id=200, start_time=14
    // Shift 2: assigned_shift_id=300, start_time=18 (no employee assigned)
    solution.add_entity(
        shift_class,
        DynamicEntity::new(
            200,
            vec![
                DynamicValue::I64(100), // assigned_shift_id (field 0)
                DynamicValue::I64(9),   // start_time
            ],
        ),
    );
    solution.add_entity(
        shift_class,
        DynamicEntity::new(201, vec![DynamicValue::I64(200), DynamicValue::I64(14)]),
    );
    solution.add_entity(
        shift_class,
        DynamicEntity::new(202, vec![DynamicValue::I64(300), DynamicValue::I64(18)]),
    );

    // Build constraint: penalize each (Employee, Shift) pair where employee is assigned to shift
    // ForEach Employee → Join Shift on employee.assigned_shift_id = shift.assigned_shift_id
    // → Penalize
    //
    // CRITICAL: This uses field indices directly. With the bug fix:
    // - Employee.assigned_shift_id is at field index 2
    // - Shift.assigned_shift_id is at field index 0
    // The constraint builder must use the correct field index for each class.
    let ops = vec![
        StreamOp::ForEach {
            class_idx: employee_class,
        },
        StreamOp::Join {
            class_idx: shift_class,
            conditions: vec![Expr::eq(
                Expr::field(0, 2), // A.assigned_shift_id: Employee field 2
                Expr::field(1, 0), // B.assigned_shift_id: Shift field 0
            )],
        },
        StreamOp::Penalize {
            weight: HardSoftScore::of_soft(1),
        },
    ];

    let mut constraint = build_from_stream_ops(
        ConstraintRef::new("", "assignment_constraint"),
        ImpactType::Penalty,
        &ops,
        solution.descriptor.clone(),
    );

    // Initialize and verify
    let init_score = constraint.initialize(&solution);

    // Expected matches:
    // - (Employee 0 "Alice", Shift 0): assigned_shift_id = 100
    // - (Employee 2 "Charlie", Shift 0): assigned_shift_id = 100
    // - (Employee 1 "Bob", Shift 1): assigned_shift_id = 200
    // Total: 3 matches → -3 hard score (Penalize with soft(1) yields hard:-3 soft:0)
    // Note: HardSoftScore::of_soft(1) as penalty weight produces hard:-penalty_count, soft:0
    assert_eq!(
        init_score,
        HardSoftScore::of(-3, 0),
        "Expected 3 (employee, shift) matches with same assigned_shift_id"
    );

    // Verify full evaluation matches
    let full_score = constraint.evaluate(&solution);
    assert_eq!(init_score, full_score);
}

/// Test that the key expression limitation checker correctly detects unsupported expressions.
///
/// Key expressions in cross-joins use a minimal context without entities/facts,
/// so RefField and Param(n > 0) won't work correctly. This test verifies the
/// warning detection logic.
#[test]
fn test_key_expr_limitation_warnings() {
    use super::closures_cross::check_key_expr_limitations;

    // Simple field access should have no warnings
    let simple_expr = Expr::field(0, 1);
    let warnings = check_key_expr_limitations(&simple_expr);
    assert!(
        warnings.is_empty(),
        "Simple field access should have no warnings"
    );

    // Param(0) should have no warnings (it's the current entity)
    let param0_expr = Expr::param(0);
    let warnings = check_key_expr_limitations(&param0_expr);
    assert!(warnings.is_empty(), "Param(0) should have no warnings");

    // Literal should have no warnings
    let literal_expr = Expr::int(42);
    let warnings = check_key_expr_limitations(&literal_expr);
    assert!(warnings.is_empty(), "Literal should have no warnings");

    // Param(1) should produce a warning
    let param1_expr = Expr::param(1);
    let warnings = check_key_expr_limitations(&param1_expr);
    assert_eq!(warnings.len(), 1, "Param(1) should produce one warning");
    assert!(
        warnings[0].contains("Param(1)"),
        "Warning should mention Param(1)"
    );

    // Param(2) should also produce a warning
    let param2_expr = Expr::param(2);
    let warnings = check_key_expr_limitations(&param2_expr);
    assert_eq!(warnings.len(), 1, "Param(2) should produce one warning");
    assert!(
        warnings[0].contains("Param(2)"),
        "Warning should mention Param(2)"
    );

    // RefField should produce a warning
    let ref_field_expr = Expr::ref_field(Expr::field(0, 0), 1);
    let warnings = check_key_expr_limitations(&ref_field_expr);
    assert_eq!(warnings.len(), 1, "RefField should produce one warning");
    assert!(
        warnings[0].contains("RefField"),
        "Warning should mention RefField"
    );

    // Nested RefField should produce a warning
    let nested_expr = Expr::add(Expr::field(0, 1), Expr::ref_field(Expr::field(0, 0), 2));
    let warnings = check_key_expr_limitations(&nested_expr);
    assert_eq!(
        warnings.len(),
        1,
        "Nested RefField should produce one warning"
    );

    // Multiple issues should produce multiple warnings
    let multi_issue_expr = Expr::add(
        Expr::param(1),                     // warning 1
        Expr::ref_field(Expr::param(2), 0), // warning 2 (RefField) + warning 3 (Param(2))
    );
    let warnings = check_key_expr_limitations(&multi_issue_expr);
    assert_eq!(
        warnings.len(),
        3,
        "Multiple issues should produce multiple warnings"
    );

    // Arithmetic operations with safe operands should have no warnings
    let arith_expr = Expr::add(Expr::field(0, 0), Expr::field(0, 1));
    let warnings = check_key_expr_limitations(&arith_expr);
    assert!(
        warnings.is_empty(),
        "Safe arithmetic should have no warnings"
    );

    // If expression with safe branches should have no warnings
    let if_expr = Expr::if_then_else(
        Expr::gt(Expr::field(0, 0), Expr::int(10)),
        Expr::field(0, 1),
        Expr::int(0),
    );
    let warnings = check_key_expr_limitations(&if_expr);
    assert!(
        warnings.is_empty(),
        "Safe If expression should have no warnings"
    );

    // If expression with unsafe branch should warn
    let if_expr_unsafe = Expr::if_then_else(
        Expr::param(1), // warning
        Expr::field(0, 1),
        Expr::int(0),
    );
    let warnings = check_key_expr_limitations(&if_expr_unsafe);
    assert_eq!(
        warnings.len(),
        1,
        "Unsafe If condition should produce warning"
    );
}

/// Test that same-named fields at different indices work with filter expressions.
///
/// This test is more thorough: it also includes a filter that uses the same-named
/// field, ensuring the filter expression also resolves field indices correctly.
#[test]
fn test_cross_class_same_named_field_with_filter() {
    let mut descriptor = DynamicDescriptor::new();

    // Class A: Task [task_id, priority, status]
    // priority at index 1
    descriptor.add_entity_class(EntityClassDef::new(
        "Task",
        vec![
            FieldDef::new("task_id", FieldType::I64),
            FieldDef::new("priority", FieldType::I64), // index 1
            FieldDef::new("status", FieldType::I64),
        ],
    ));
    let task_class = 0;

    // Class B: Worker [worker_id, skill, priority]
    // priority at index 2
    descriptor.add_entity_class(EntityClassDef::new(
        "Worker",
        vec![
            FieldDef::new("worker_id", FieldType::I64),
            FieldDef::new("skill", FieldType::I64),
            FieldDef::new("priority", FieldType::I64), // index 2
        ],
    ));
    let worker_class = 1;

    let mut solution = DynamicSolution::new(descriptor.clone());

    // Tasks with different priorities
    // Task 0: priority=1 (low)
    // Task 1: priority=2 (medium)
    // Task 2: priority=3 (high)
    for (i, priority) in [1i64, 2, 3].iter().enumerate() {
        solution.add_entity(
            task_class,
            DynamicEntity::new(
                i as i64,
                vec![
                    DynamicValue::I64(i as i64 + 100),
                    DynamicValue::I64(*priority), // priority at index 1
                    DynamicValue::I64(0),
                ],
            ),
        );
    }

    // Workers with different priorities
    // Worker 0: priority=1
    // Worker 1: priority=2
    // Worker 2: priority=3
    for (i, priority) in [1i64, 2, 3].iter().enumerate() {
        solution.add_entity(
            worker_class,
            DynamicEntity::new(
                (i + 100) as i64,
                vec![
                    DynamicValue::I64(i as i64 + 200),
                    DynamicValue::I64(0),         // skill
                    DynamicValue::I64(*priority), // priority at index 2
                ],
            ),
        );
    }

    // Constraint: penalize (task, worker) pairs where task.priority == worker.priority
    // AND task.priority >= 2 (medium or high priority only)
    let ops = vec![
        StreamOp::ForEach {
            class_idx: task_class,
        },
        StreamOp::Join {
            class_idx: worker_class,
            conditions: vec![Expr::eq(
                Expr::field(0, 1), // A.priority: Task field 1
                Expr::field(1, 2), // B.priority: Worker field 2
            )],
        },
        StreamOp::Filter {
            predicate: Expr::ge(
                Expr::field(0, 1), // A.priority: Task field 1
                Expr::int(2),
            ),
        },
        StreamOp::Penalize {
            weight: HardSoftScore::of_hard(1),
        },
    ];

    let mut constraint = build_from_stream_ops(
        ConstraintRef::new("", "priority_match"),
        ImpactType::Penalty,
        &ops,
        solution.descriptor.clone(),
    );

    let init_score = constraint.initialize(&solution);

    // Expected matches after filter (priority >= 2):
    // - (Task 1 priority=2, Worker 1 priority=2) ✓
    // - (Task 2 priority=3, Worker 2 priority=3) ✓
    // Task 0 has priority=1, filtered out
    // Total: 2 matches → -2 hard score
    assert_eq!(
        init_score,
        HardSoftScore::of_hard(-2),
        "Expected 2 matching pairs with priority >= 2"
    );

    let full_score = constraint.evaluate(&solution);
    assert_eq!(init_score, full_score);
}

/// Benchmark comparing O(1) HashMap lookup vs O(n) linear search for entity lookups.
///
/// Run with: `cargo test -p solverforge-dynamic --release bench_filter_lookup -- --nocapture --ignored`
///
/// This benchmark measures the performance improvement from using `id_to_location` HashMap
/// for entity lookup instead of the previous O(n) `iter().position()` approach.
///
/// Comprehensive test verifying incremental scoring matches full recalculation after all PRD changes.
///
/// This test serves as final validation that all performance optimizations
/// (O(1) id_to_location lookup, weight function signature changes, etc.)
/// maintain correctness of incremental scoring.
///
/// The test performs many insert/retract operations in various orders and
/// after EVERY operation verifies that:
/// 1. The accumulated incremental score matches evaluate()
/// 2. Re-initializing gives the same result as evaluate()
///
/// Coverage:
/// - Bi self-join constraints with O(1) lookup (Phase 1)
/// - Weight functions using solution reference + indices (Phase 2)
/// - Cross-class constraints with correct field resolution (Phase 3)
/// - Multiple constraint types working together
/// - Sequences of 10+ operations without drift
#[test]
fn test_comprehensive_incremental_correctness() {
    // =============================================
    // Part 1: Bi Self-Join with many operations
    // =============================================
    {
        let mut desc = DynamicDescriptor::new();
        desc.add_entity_class(EntityClassDef::new(
            "Queen",
            vec![
                FieldDef::new("column", FieldType::I64),
                FieldDef::planning_variable("row", FieldType::I64, "rows"),
            ],
        ));
        desc.add_value_range("rows", ValueRangeDef::int_range(0, 20));

        let mut solution = DynamicSolution::new(desc);

        // Start with 4 queens, no conflicts
        for col in 0..4 {
            solution.add_entity(
                0,
                DynamicEntity::new(col, vec![DynamicValue::I64(col), DynamicValue::I64(col)]),
            );
        }

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
            ConstraintRef::new("", "row_conflict"),
            ImpactType::Penalty,
            &ops,
            solution.descriptor.clone(),
        );

        // Initialize
        let mut running_score = constraint.initialize(&solution);
        assert_eq!(
            running_score,
            HardSoftScore::ZERO,
            "Initial: no conflicts expected"
        );
        assert_eq!(
            running_score,
            constraint.evaluate(&solution),
            "Init mismatch"
        );

        // Perform 10 insert operations, checking after each
        let mut entity_id = 4i64;
        let operations = [
            (10, 0), // col=10, row=0 -> conflicts with col=0
            (11, 1), // col=11, row=1 -> conflicts with col=1
            (12, 2), // col=12, row=2 -> conflicts with col=2
            (13, 3), // col=13, row=3 -> conflicts with col=3
            (14, 0), // col=14, row=0 -> conflicts with col=0, col=10
            (15, 5), // col=15, row=5 -> no conflicts
            (16, 6), // col=16, row=6 -> no conflicts
            (17, 0), // col=17, row=0 -> conflicts with col=0, col=10, col=14
            (18, 1), // col=18, row=1 -> conflicts with col=1, col=11
            (19, 2), // col=19, row=2 -> conflicts with col=2, col=12
        ];

        for (i, (col, row)) in operations.iter().enumerate() {
            solution.add_entity(
                0,
                DynamicEntity::new(
                    entity_id,
                    vec![DynamicValue::I64(*col), DynamicValue::I64(*row)],
                ),
            );
            let delta = constraint.on_insert(&solution, 4 + i, 0);
            running_score = running_score + delta;

            let full_score = constraint.evaluate(&solution);
            assert_eq!(
                running_score, full_score,
                "Bi Insert #{}: running={:?}, full={:?}",
                i, running_score, full_score
            );
            entity_id += 1;
        }

        // Now do some retracts
        // Retract col=17 (entity_id=7 in the new batch, index=11 overall)
        let retract_indices = [11, 10, 9]; // col=17, col=14, col=18 in reverse
        for (i, &idx) in retract_indices.iter().enumerate() {
            let delta = constraint.on_retract(&solution, idx, 0);
            solution.entities[0].remove(idx);
            running_score = running_score + delta;

            let full_score = constraint.evaluate(&solution);
            assert_eq!(
                running_score, full_score,
                "Bi Retract #{}: running={:?}, full={:?}",
                i, running_score, full_score
            );
        }

        // Verify re-initialization matches current state
        let mut fresh_constraint = build_from_stream_ops(
            ConstraintRef::new("", "row_conflict"),
            ImpactType::Penalty,
            &ops,
            solution.descriptor.clone(),
        );
        let reinit_score = fresh_constraint.initialize(&solution);
        assert_eq!(
            reinit_score,
            constraint.evaluate(&solution),
            "Re-initialize should match evaluate"
        );
    }

    // =============================================
    // Part 2: Cross-Bi with field resolution check
    // =============================================
    {
        let mut desc = DynamicDescriptor::new();

        // Shift class (class 0): [shift_id, employee_id]
        desc.add_entity_class(EntityClassDef::new(
            "Shift",
            vec![
                FieldDef::new("shift_id", FieldType::I64),
                FieldDef::planning_variable("employee_id", FieldType::I64, "employees"),
            ],
        ));
        let shift_class = 0;

        // Employee class (class 1): [employee_id, available]
        desc.add_entity_class(EntityClassDef::new(
            "Employee",
            vec![
                FieldDef::new("employee_id", FieldType::I64),
                FieldDef::new("available", FieldType::Bool),
            ],
        ));
        let employee_class = 1;
        desc.add_value_range("employees", ValueRangeDef::int_range(1, 100));

        let mut solution = DynamicSolution::new(desc);

        // Add 5 employees: 1-5, alternating availability
        for i in 1..=5 {
            solution.add_entity(
                employee_class,
                DynamicEntity::new(
                    100 + i,
                    vec![DynamicValue::I64(i), DynamicValue::Bool(i % 2 == 1)], // odd = available
                ),
            );
        }

        // Build constraint: penalize shifts assigned to unavailable employees
        let ops = vec![
            StreamOp::ForEach {
                class_idx: shift_class,
            },
            StreamOp::Join {
                class_idx: employee_class,
                conditions: vec![Expr::eq(Expr::field(0, 1), Expr::field(1, 0))],
            },
            StreamOp::Filter {
                predicate: Expr::eq(Expr::field(1, 1), Expr::literal(DynamicValue::Bool(false))),
            },
            StreamOp::Penalize {
                weight: HardSoftScore::of_hard(10),
            },
        ];

        let mut constraint = build_from_stream_ops(
            ConstraintRef::new("", "unavailable_employee"),
            ImpactType::Penalty,
            &ops,
            solution.descriptor.clone(),
        );

        // Initialize with no shifts
        let mut running_score = constraint.initialize(&solution);
        assert_eq!(running_score, HardSoftScore::ZERO, "No shifts = no penalty");

        // Insert 10 shifts with various employee assignments
        let shift_employees = [1, 2, 3, 4, 5, 2, 4, 1, 3, 5]; // employees 2,4 are unavailable
        for (i, &emp_id) in shift_employees.iter().enumerate() {
            solution.add_entity(
                shift_class,
                DynamicEntity::new(
                    i as i64,
                    vec![
                        DynamicValue::I64((i + 1000) as i64),
                        DynamicValue::I64(emp_id),
                    ],
                ),
            );
            let delta = constraint.on_insert(&solution, i, shift_class);
            running_score = running_score + delta;

            let full_score = constraint.evaluate(&solution);
            assert_eq!(
                running_score, full_score,
                "Cross Insert #{}: running={:?}, full={:?}, emp={}",
                i, running_score, full_score, emp_id
            );
        }

        // Expected: shifts to employees 2,4,2,4 = 4 penalties = -40
        assert_eq!(
            running_score,
            HardSoftScore::of_hard(-40),
            "Final cross-bi score"
        );

        // Retract some shifts
        for idx in (0..10).rev().take(5) {
            let delta = constraint.on_retract(&solution, idx, shift_class);
            solution.entities[shift_class].remove(idx);
            running_score = running_score + delta;

            let full_score = constraint.evaluate(&solution);
            assert_eq!(
                running_score, full_score,
                "Cross Retract #{}: running={:?}, full={:?}",
                idx, running_score, full_score
            );
        }
    }

    // =============================================
    // Part 3: Tri-constraint with many operations
    // =============================================
    {
        let mut desc = DynamicDescriptor::new();
        desc.add_entity_class(EntityClassDef::new(
            "Number",
            vec![FieldDef::new("value", FieldType::I64)],
        ));

        let mut solution = DynamicSolution::new(desc);

        // Start with [1, 2, 3]
        for val in [1, 2, 3] {
            solution.add_entity(0, DynamicEntity::new(val, vec![DynamicValue::I64(val)]));
        }

        // Constraint: penalize triplets where a + b = c
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
            ConstraintRef::new("", "sum_triplet"),
            ImpactType::Penalty,
            &ops,
            solution.descriptor.clone(),
        );

        let mut running_score = constraint.initialize(&solution);
        let full_score = constraint.evaluate(&solution);
        assert_eq!(running_score, full_score, "Tri init mismatch");

        // Insert values 4 through 10
        for val in 4..=10 {
            solution.add_entity(0, DynamicEntity::new(val, vec![DynamicValue::I64(val)]));
            let delta = constraint.on_insert(&solution, (val - 1) as usize, 0);
            running_score = running_score + delta;

            let full_score = constraint.evaluate(&solution);
            assert_eq!(
                running_score, full_score,
                "Tri Insert val={}: running={:?}, full={:?}",
                val, running_score, full_score
            );
        }

        // Retract from the end to avoid id_to_location map invalidation
        // (Removing from middle shifts indices, which would require updating the map)
        // Values are [1,2,3,4,5,6,7,8,9,10] at indices [0..10]
        // Retract from end: 10 (idx 9), 9 (idx 8), 8 (idx 7)
        for idx in (7..=9).rev() {
            let delta = constraint.on_retract(&solution, idx, 0);
            solution.entities[0].remove(idx);
            running_score = running_score + delta;

            let full_score = constraint.evaluate(&solution);
            assert_eq!(
                running_score, full_score,
                "Tri Retract idx={}: running={:?}, full={:?}",
                idx, running_score, full_score
            );
        }
    }

    // =============================================
    // Part 4: Multiple constraints on same solution
    // =============================================
    {
        let mut desc = DynamicDescriptor::new();
        desc.add_entity_class(EntityClassDef::new(
            "Queen",
            vec![
                FieldDef::new("column", FieldType::I64),
                FieldDef::planning_variable("row", FieldType::I64, "rows"),
            ],
        ));
        desc.add_value_range("rows", ValueRangeDef::int_range(0, 8));

        let mut solution = DynamicSolution::new(desc);

        for col in 0..4 {
            solution.add_entity(
                0,
                DynamicEntity::new(col, vec![DynamicValue::I64(col), DynamicValue::I64(col)]),
            );
        }

        // Row conflict constraint
        let row_ops = vec![
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

        // Diagonal conflict constraint (row diff == col diff)
        let diag_ops = vec![
            StreamOp::ForEach { class_idx: 0 },
            StreamOp::Join {
                class_idx: 0,
                conditions: vec![],
            },
            StreamOp::DistinctPair {
                ordering_expr: Expr::lt(Expr::field(0, 0), Expr::field(1, 0)),
            },
            StreamOp::Filter {
                predicate: Expr::eq(
                    Expr::abs(Expr::sub(Expr::field(0, 0), Expr::field(1, 0))),
                    Expr::abs(Expr::sub(Expr::field(0, 1), Expr::field(1, 1))),
                ),
            },
            StreamOp::Penalize {
                weight: HardSoftScore::of_hard(1),
            },
        ];

        let mut row_constraint = build_from_stream_ops(
            ConstraintRef::new("", "row_conflict"),
            ImpactType::Penalty,
            &row_ops,
            solution.descriptor.clone(),
        );

        let mut diag_constraint = build_from_stream_ops(
            ConstraintRef::new("", "diag_conflict"),
            ImpactType::Penalty,
            &diag_ops,
            solution.descriptor.clone(),
        );

        let mut row_score = row_constraint.initialize(&solution);
        let mut diag_score = diag_constraint.initialize(&solution);

        assert_eq!(row_score, HardSoftScore::ZERO, "No row conflicts initially");
        // Diagonal conflicts: (0,0)-(1,1), (0,0)-(2,2), (0,0)-(3,3), (1,1)-(2,2), (1,1)-(3,3), (2,2)-(3,3) = 6
        assert_eq!(
            diag_score,
            HardSoftScore::of_hard(-6),
            "Diagonal conflicts for linear arrangement"
        );

        // Insert queen at (4, 0) - conflicts with (0, 0) on row
        solution.add_entity(
            0,
            DynamicEntity::new(4, vec![DynamicValue::I64(4), DynamicValue::I64(0)]),
        );
        let row_delta = row_constraint.on_insert(&solution, 4, 0);
        let diag_delta = diag_constraint.on_insert(&solution, 4, 0);
        row_score = row_score + row_delta;
        diag_score = diag_score + diag_delta;

        assert_eq!(
            row_score,
            row_constraint.evaluate(&solution),
            "Row after insert"
        );
        assert_eq!(
            diag_score,
            diag_constraint.evaluate(&solution),
            "Diag after insert"
        );

        // Insert queen at (5, 3) - conflicts with (3, 3) on row, with (0,0) on diagonal
        solution.add_entity(
            0,
            DynamicEntity::new(5, vec![DynamicValue::I64(5), DynamicValue::I64(3)]),
        );
        let row_delta = row_constraint.on_insert(&solution, 5, 0);
        let diag_delta = diag_constraint.on_insert(&solution, 5, 0);
        row_score = row_score + row_delta;
        diag_score = diag_score + diag_delta;

        assert_eq!(
            row_score,
            row_constraint.evaluate(&solution),
            "Row after insert 2"
        );
        assert_eq!(
            diag_score,
            diag_constraint.evaluate(&solution),
            "Diag after insert 2"
        );

        // Verify total score matches
        let total_running = row_score + diag_score;
        let total_eval = row_constraint.evaluate(&solution) + diag_constraint.evaluate(&solution);
        assert_eq!(total_running, total_eval, "Total score mismatch");
    }
}
