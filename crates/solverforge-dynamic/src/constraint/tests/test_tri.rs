//! Tests for tri (three-entity) constraints.

use super::*;

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
