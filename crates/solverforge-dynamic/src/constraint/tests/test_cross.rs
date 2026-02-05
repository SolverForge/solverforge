//! Tests for cross-class constraints.

use super::*;

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
    use super::super::closures_cross::check_key_expr_limitations;

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
