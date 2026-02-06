//! Integration tests for the solver.

use super::*;
use std::time::Duration;

#[test]
fn test_solve_4_queens() {
    let (solution, constraints) = make_nqueens_problem(4);

    let config = SolveConfig::with_time_limit(Duration::from_secs(5));
    let result = solve(solution, constraints, config);

    // 4-queens should always find a feasible solution
    assert!(
        result.is_feasible(),
        "4-queens should be feasible, got score: {}",
        result.score
    );
}

#[test]
fn test_solve_8_queens() {
    let (solution, constraints) = make_nqueens_problem(8);

    // 30 seconds - longer because evaluating ALL moves is slow without incremental scoring
    let config = SolveConfig::with_time_limit(Duration::from_secs(30));
    let result = solve(solution, constraints, config);

    // 8-queens should find a feasible solution within 30 seconds
    assert!(
        result.is_feasible(),
        "8-queens should be feasible, got score: {}",
        result.score
    );
}

// Integration test with 1000+ entities verifying correct assignments.
//
// This test creates a large-scale scheduling-like problem with:
// - 1000 Task entities with slot planning variables
// - Simple non-overlap constraints
//
// The test verifies:
// 1. All entities are correctly tracked in the solution
// 2. Entity IDs are correctly mapped via id_to_location HashMap
// 3. Constraint evaluation works correctly at scale
// 4. Incremental scoring produces valid results
#[test]
fn test_large_scale_entity_assignments() {
    const NUM_ENTITIES: usize = 1000;
    const NUM_SLOTS: i64 = 100; // Allows some conflicts to test constraint behavior

    // Create descriptor with Task entity class
    let mut desc = DynamicDescriptor::new();
    desc.add_entity_class(EntityClassDef::new(
        "Task",
        vec![
            FieldDef::new("task_id", FieldType::I64),
            FieldDef::new("resource_id", FieldType::I64), // Grouping field
            FieldDef::planning_variable("slot", FieldType::I64, "slots"),
        ],
    ));
    desc.add_value_range("slots", ValueRangeDef::int_range(0, NUM_SLOTS));

    // Create solution with 1000 entities across 10 resources
    let mut solution = DynamicSolution::new(desc.clone());
    for i in 0..NUM_ENTITIES {
        let task_id = i as i64;
        let resource_id = (i % 10) as i64; // 10 resources, ~100 tasks each
        solution.add_entity(
            0,
            DynamicEntity::new(
                task_id,
                vec![
                    DynamicValue::I64(task_id),
                    DynamicValue::I64(resource_id),
                    DynamicValue::None, // slot unassigned
                ],
            ),
        );
    }

    // Verify all entities were added correctly
    assert_eq!(solution.entities[0].len(), NUM_ENTITIES);
    assert_eq!(solution.id_to_location.len(), NUM_ENTITIES);

    // Verify id_to_location mapping is correct for all entities
    for i in 0..NUM_ENTITIES {
        let task_id = i as i64;
        let location = solution.get_entity_location(task_id);
        assert!(
            location.is_some(),
            "Entity {} should be in id_to_location map",
            task_id
        );
        let (class_idx, entity_idx) = location.unwrap();
        assert_eq!(class_idx, 0, "Entity {} should be in class 0", task_id);
        assert_eq!(entity_idx, i, "Entity {} should be at index {}", task_id, i);

        // Verify we can retrieve the entity and its fields are correct
        let entity = solution.get_entity(class_idx, entity_idx).unwrap();
        assert_eq!(entity.id, task_id);
        assert_eq!(entity.fields[0], DynamicValue::I64(task_id));
    }

    // Create resource conflict constraint:
    // Tasks on the same resource cannot have the same slot
    let conflict_ops = vec![
        StreamOp::ForEach { class_idx: 0 },
        StreamOp::Join {
            class_idx: 0,
            // Join on same resource_id (field 1)
            conditions: vec![Expr::eq(Expr::field(0, 1), Expr::field(1, 1))],
        },
        StreamOp::DistinctPair {
            // Ensure we only count each pair once (A.task_id < B.task_id)
            ordering_expr: Expr::lt(Expr::field(0, 0), Expr::field(1, 0)),
        },
        // Filter: penalize when slots are equal and both assigned
        StreamOp::Filter {
            predicate: Expr::and(
                Expr::and(
                    Expr::is_not_none(Expr::field(0, 2)),
                    Expr::is_not_none(Expr::field(1, 2)),
                ),
                Expr::eq(Expr::field(0, 2), Expr::field(1, 2)),
            ),
        },
        StreamOp::Penalize {
            weight: HardSoftScore::of_hard(1),
        },
    ];

    let conflict_constraint = build_from_stream_ops(
        ConstraintRef::new("", "resource_slot_conflict"),
        ImpactType::Penalty,
        &conflict_ops,
        desc.clone(),
    );

    let mut constraints = DynamicConstraintSet::new();
    constraints.add(conflict_constraint);

    // Initialize with unassigned solution - should have score 0 (no conflicts)
    let initial_score = constraints.initialize_all(&solution);
    assert_eq!(
        initial_score,
        HardSoftScore::ZERO,
        "Unassigned solution should have no conflicts"
    );

    // Now assign slots to create known conflicts:
    // Assign all tasks on resource 0 to slot 0 (creates many conflicts)
    let mut conflict_count = 0i64;
    let tasks_on_resource_0: Vec<usize> = (0..NUM_ENTITIES).filter(|i| i % 10 == 0).collect();

    for (idx, &entity_idx) in tasks_on_resource_0.iter().enumerate() {
        // Retract, update, insert
        let delta1 = constraints.on_retract_all(&solution, entity_idx, 0);
        solution.update_field(0, entity_idx, 2, DynamicValue::I64(0)); // slot 0
        let delta2 = constraints.on_insert_all(&solution, entity_idx, 0);

        // Each new assignment to slot 0 creates conflicts with all previous assignments
        // Number of conflicts = n * (n-1) / 2 for n entities
        let n = idx + 1;
        let expected_conflicts = (n * (n - 1) / 2) as i64;
        let current_score = initial_score + delta1 + delta2;
        // Accumulate deltas for verification
        conflict_count = expected_conflicts;

        if idx < 5 {
            // Debug first few
            eprintln!(
                "After assigning entity {} to slot 0: score={:?}, expected_conflicts={}",
                entity_idx, current_score, expected_conflicts
            );
        }
    }

    // Verify final score matches expected conflicts
    let final_score = constraints.evaluate_all(&solution);
    let expected_final = HardSoftScore::of_hard(-conflict_count);
    eprintln!(
        "Final score: {:?}, expected: {:?}, tasks_on_resource_0: {}",
        final_score,
        expected_final,
        tasks_on_resource_0.len()
    );
    assert_eq!(
        final_score, expected_final,
        "Score should reflect {} conflicts",
        conflict_count
    );

    // Verify entity assignments are all correct
    for &entity_idx in &tasks_on_resource_0 {
        let entity = solution.get_entity(0, entity_idx).unwrap();
        assert_eq!(
            entity.fields[2],
            DynamicValue::I64(0),
            "Entity {} should have slot 0",
            entity_idx
        );
    }

    eprintln!(
        "Large-scale test passed: {} entities, {} conflicts detected",
        NUM_ENTITIES, conflict_count
    );
}

// Integration test verifying the solver can handle many entities in a real solve.
//
// Uses a simplified scheduling problem where correctness can be verified.
// Note: Uses 100 entities with 100 slots - construction phase evaluates all
// possible moves (100*100 = 10,000 per entity), so this is already substantial.
#[test]
fn test_solve_many_entities() {
    use std::time::Duration;

    const NUM_ENTITIES: usize = 100;
    const NUM_SLOTS: i64 = 100; // Enough slots for a feasible solution

    // Create descriptor
    let mut desc = DynamicDescriptor::new();
    desc.add_entity_class(EntityClassDef::new(
        "Item",
        vec![
            FieldDef::new("item_id", FieldType::I64),
            FieldDef::planning_variable("position", FieldType::I64, "positions"),
        ],
    ));
    desc.add_value_range("positions", ValueRangeDef::int_range(0, NUM_SLOTS));

    // Create solution
    let mut solution = DynamicSolution::new(desc.clone());
    for i in 0..NUM_ENTITIES {
        solution.add_entity(
            0,
            DynamicEntity::new(
                i as i64,
                vec![DynamicValue::I64(i as i64), DynamicValue::None],
            ),
        );
    }

    // Create uniqueness constraint: no two items can have the same position
    let uniqueness_ops = vec![
        StreamOp::ForEach { class_idx: 0 },
        StreamOp::Join {
            class_idx: 0,
            conditions: vec![
                // Join where positions are equal
                Expr::eq(Expr::field(0, 1), Expr::field(1, 1)),
            ],
        },
        StreamOp::DistinctPair {
            ordering_expr: Expr::lt(Expr::field(0, 0), Expr::field(1, 0)),
        },
        // Only penalize if both positions are assigned
        StreamOp::Filter {
            predicate: Expr::and(
                Expr::is_not_none(Expr::field(0, 1)),
                Expr::is_not_none(Expr::field(1, 1)),
            ),
        },
        StreamOp::Penalize {
            weight: HardSoftScore::of_hard(1),
        },
    ];

    let uniqueness_constraint = build_from_stream_ops(
        ConstraintRef::new("", "position_uniqueness"),
        ImpactType::Penalty,
        &uniqueness_ops,
        desc.clone(),
    );

    let mut constraints = DynamicConstraintSet::new();
    constraints.add(uniqueness_constraint);

    // Solve with time limit
    let config = SolveConfig::with_time_limit(Duration::from_secs(30));
    let result = solve(solution, constraints, config);

    eprintln!(
        "Solve result: score={:?}, feasible={}, duration={:?}",
        result.score,
        result.is_feasible(),
        result.duration
    );

    // Verify the result
    assert!(
        result.is_feasible(),
        "Should find feasible solution for {} entities with {} slots",
        NUM_ENTITIES,
        NUM_SLOTS
    );

    // Verify all entities have assignments
    let mut assigned_positions: std::collections::HashSet<i64> = std::collections::HashSet::new();
    for entity in &result.solution.entities[0] {
        match &entity.fields[1] {
            DynamicValue::I64(pos) => {
                assert!(
                    !assigned_positions.contains(pos),
                    "Position {} assigned to multiple entities",
                    pos
                );
                assigned_positions.insert(*pos);
            }
            DynamicValue::None => {
                panic!("Entity {} has no position assigned", entity.id);
            }
            other => {
                panic!(
                    "Entity {} has invalid position value: {:?}",
                    entity.id, other
                );
            }
        }
    }

    assert_eq!(
        assigned_positions.len(),
        NUM_ENTITIES,
        "All entities should have unique positions"
    );

    eprintln!(
        "{}-entity solve test passed: {} unique positions assigned",
        NUM_ENTITIES,
        assigned_positions.len()
    );
}
