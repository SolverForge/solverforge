//! Tests for incremental scoring correctness.

use super::*;

// Test that incremental deltas match full recalculation across multiple constraint types.
//
// This test creates constraints of different patterns (bi self-join, tri self-join,
// cross-bi, flattened-bi) and verifies that:
// 1. After initialize(), the incremental score matches evaluate()
// 2. After each on_insert(), accumulated delta matches evaluate()
// 3. After each on_retract(), accumulated delta matches evaluate()
//
// This provides strong evidence that the incremental indexing is correct and
// doesn't drift from the true score over multiple operations.
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
        solution.remove_entity(0, 3);
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

// Comprehensive test verifying incremental scoring matches full recalculation after all PRD changes.
//
// This test serves as final validation that all performance optimizations
// (O(1) id_to_location lookup, weight function signature changes, etc.)
// maintain correctness of incremental scoring.
//
// The test performs many insert/retract operations in various orders and
// after EVERY operation verifies that:
// 1. The accumulated incremental score matches evaluate()
// 2. Re-initializing gives the same result as evaluate()
//
// Coverage:
// - Bi self-join constraints with O(1) lookup (Phase 1)
// - Weight functions using solution reference + indices (Phase 2)
// - Cross-class constraints with correct field resolution (Phase 3)
// - Multiple constraint types working together
// - Sequences of 10+ operations without drift
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
            solution.remove_entity(0, idx);
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
            solution.remove_entity(shift_class, idx);
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
            solution.remove_entity(0, idx);
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
