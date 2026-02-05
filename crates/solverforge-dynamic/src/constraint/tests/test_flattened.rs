//! Tests for flattened bi-constraints.

use super::*;

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
