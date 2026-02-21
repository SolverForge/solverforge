//! Tests for move generation.

use super::*;
use crate::descriptor::{DynamicDescriptor, EntityClassDef, FieldDef, FieldType, ValueRangeDef};
use crate::solution::DynamicEntity;

#[test]
fn test_generate_change_moves() {
    let mut desc = DynamicDescriptor::new();
    desc.add_entity_class(EntityClassDef::new(
        "Queen",
        vec![
            FieldDef::new("column", FieldType::I64),
            FieldDef::planning_variable("row", FieldType::I64, "rows"),
        ],
    ));
    desc.add_value_range("rows", ValueRangeDef::int_range(0, 4));

    let mut solution = DynamicSolution::new(desc);
    solution.add_entity(
        0,
        DynamicEntity::new(0, vec![DynamicValue::I64(0), DynamicValue::I64(0)]),
    );
    solution.add_entity(
        0,
        DynamicEntity::new(1, vec![DynamicValue::I64(1), DynamicValue::I64(1)]),
    );

    // Use DynamicMoveIterator directly for change moves
    let moves: Vec<_> = DynamicMoveIterator::new(&solution).collect();

    // 2 entities * 4 possible row values = 8 change moves
    assert_eq!(moves.len(), 8);
}

#[test]
fn test_selector_generates_change_and_swap() {
    let mut desc = DynamicDescriptor::new();
    desc.add_entity_class(EntityClassDef::new(
        "Queen",
        vec![
            FieldDef::new("column", FieldType::I64),
            FieldDef::planning_variable("row", FieldType::I64, "rows"),
        ],
    ));
    desc.add_value_range("rows", ValueRangeDef::int_range(0, 4));

    let mut solution = DynamicSolution::new(desc);
    solution.add_entity(
        0,
        DynamicEntity::new(0, vec![DynamicValue::I64(0), DynamicValue::I64(0)]),
    );
    solution.add_entity(
        0,
        DynamicEntity::new(1, vec![DynamicValue::I64(1), DynamicValue::I64(1)]),
    );

    let selector = DynamicMoveSelector::new();

    // Build a minimal score director to call iter_moves via the trait
    let constraint_set = crate::constraint_set::DynamicConstraintSet::new();
    let descriptor = solverforge_core::domain::SolutionDescriptor::new(
        "DynSol",
        std::any::TypeId::of::<DynamicSolution>(),
    );
    fn counter(s: &DynamicSolution, idx: usize) -> usize {
        s.entities.get(idx).map(|v| v.len()).unwrap_or(0)
    }
    let director = solverforge_scoring::TypedScoreDirector::with_descriptor(
        solution,
        constraint_set,
        descriptor,
        counter,
    );

    let moves: Vec<_> =
        solverforge_solver::heuristic::selector::MoveSelector::iter_moves(&selector, &director)
            .collect();

    // 8 change moves + 1 swap pair (2 entities, 1 variable) = 9 total
    assert_eq!(moves.len(), 9);

    let change_count = moves
        .iter()
        .filter(|m| matches!(m, DynamicEitherMove::Change(_)))
        .count();
    let swap_count = moves
        .iter()
        .filter(|m| matches!(m, DynamicEitherMove::Swap(_)))
        .count();
    assert_eq!(change_count, 8);
    assert_eq!(swap_count, 1);
}

#[test]
fn test_dynamic_move_iterator() {
    let mut desc = DynamicDescriptor::new();
    desc.add_entity_class(EntityClassDef::new(
        "Queen",
        vec![
            FieldDef::new("column", FieldType::I64),
            FieldDef::planning_variable("row", FieldType::I64, "rows"),
        ],
    ));
    desc.add_value_range("rows", ValueRangeDef::int_range(0, 4));

    let mut solution = DynamicSolution::new(desc);
    solution.add_entity(
        0,
        DynamicEntity::new(0, vec![DynamicValue::I64(0), DynamicValue::I64(0)]),
    );
    solution.add_entity(
        0,
        DynamicEntity::new(1, vec![DynamicValue::I64(1), DynamicValue::I64(1)]),
    );

    // Create iterator and collect moves
    let iterator = DynamicMoveIterator::new(&solution);
    let moves: Vec<_> = iterator.collect();

    // 2 entities * 4 possible row values = 8 moves
    assert_eq!(moves.len(), 8);

    // Verify moves have correct structure
    // First entity (entity_idx=0) should have moves for values 0,1,2,3
    assert_eq!(moves[0].entity_idx, 0);
    assert_eq!(moves[0].class_idx, 0);
    assert_eq!(moves[0].field_idx, 1); // row is field index 1
    assert_eq!(moves[0].new_value, DynamicValue::I64(0));

    assert_eq!(moves[1].entity_idx, 0);
    assert_eq!(moves[1].new_value, DynamicValue::I64(1));

    assert_eq!(moves[2].entity_idx, 0);
    assert_eq!(moves[2].new_value, DynamicValue::I64(2));

    assert_eq!(moves[3].entity_idx, 0);
    assert_eq!(moves[3].new_value, DynamicValue::I64(3));

    // Second entity (entity_idx=1) should have moves for values 0,1,2,3
    assert_eq!(moves[4].entity_idx, 1);
    assert_eq!(moves[4].new_value, DynamicValue::I64(0));

    assert_eq!(moves[7].entity_idx, 1);
    assert_eq!(moves[7].new_value, DynamicValue::I64(3));
}

#[test]
fn test_dynamic_move_iterator_multiple_variables() {
    let mut desc = DynamicDescriptor::new();
    desc.add_entity_class(EntityClassDef::new(
        "Task",
        vec![
            FieldDef::new("id", FieldType::I64),
            FieldDef::planning_variable("worker", FieldType::I64, "workers"),
            FieldDef::planning_variable("machine", FieldType::I64, "machines"),
        ],
    ));
    desc.add_value_range("workers", ValueRangeDef::int_range(0, 2));
    desc.add_value_range("machines", ValueRangeDef::int_range(0, 3));

    let mut solution = DynamicSolution::new(desc);
    solution.add_entity(
        0,
        DynamicEntity::new(
            0,
            vec![
                DynamicValue::I64(0),
                DynamicValue::I64(0),
                DynamicValue::I64(0),
            ],
        ),
    );

    let iterator = DynamicMoveIterator::new(&solution);
    let moves: Vec<_> = iterator.collect();

    // 1 entity * (2 worker values + 3 machine values) = 5 moves
    assert_eq!(moves.len(), 5);

    // First two moves should be for worker (field_idx=1)
    assert_eq!(moves[0].field_idx, 1);
    assert_eq!(moves[0].new_value, DynamicValue::I64(0));
    assert_eq!(moves[1].field_idx, 1);
    assert_eq!(moves[1].new_value, DynamicValue::I64(1));

    // Next three moves should be for machine (field_idx=2)
    assert_eq!(moves[2].field_idx, 2);
    assert_eq!(moves[2].new_value, DynamicValue::I64(0));
    assert_eq!(moves[3].field_idx, 2);
    assert_eq!(moves[3].new_value, DynamicValue::I64(1));
    assert_eq!(moves[4].field_idx, 2);
    assert_eq!(moves[4].new_value, DynamicValue::I64(2));
}

#[test]
fn test_dynamic_move_iterator_empty_solution() {
    let mut desc = DynamicDescriptor::new();
    desc.add_entity_class(EntityClassDef::new(
        "Queen",
        vec![
            FieldDef::new("column", FieldType::I64),
            FieldDef::planning_variable("row", FieldType::I64, "rows"),
        ],
    ));
    desc.add_value_range("rows", ValueRangeDef::int_range(0, 4));

    let solution = DynamicSolution::new(desc);
    // No entities added

    let iterator = DynamicMoveIterator::new(&solution);
    let moves: Vec<_> = iterator.collect();

    // No entities = no moves
    assert_eq!(moves.len(), 0);
}

#[test]
fn test_dynamic_move_iterator_multiple_classes() {
    let mut desc = DynamicDescriptor::new();
    desc.add_entity_class(EntityClassDef::new(
        "Employee",
        vec![
            FieldDef::new("id", FieldType::I64),
            FieldDef::planning_variable("shift", FieldType::I64, "shifts"),
        ],
    ));
    desc.add_entity_class(EntityClassDef::new(
        "Vehicle",
        vec![
            FieldDef::new("id", FieldType::I64),
            FieldDef::planning_variable("route", FieldType::I64, "routes"),
        ],
    ));
    desc.add_value_range("shifts", ValueRangeDef::int_range(0, 2));
    desc.add_value_range("routes", ValueRangeDef::int_range(0, 3));

    let mut solution = DynamicSolution::new(desc);
    // Add 1 employee
    solution.add_entity(
        0,
        DynamicEntity::new(0, vec![DynamicValue::I64(0), DynamicValue::I64(0)]),
    );
    // Add 1 vehicle
    solution.add_entity(
        1,
        DynamicEntity::new(1, vec![DynamicValue::I64(0), DynamicValue::I64(0)]),
    );

    let iterator = DynamicMoveIterator::new(&solution);
    let moves: Vec<_> = iterator.collect();

    // 1 employee * 2 shifts + 1 vehicle * 3 routes = 5 moves
    assert_eq!(moves.len(), 5);

    // First two should be for employee (class_idx=0)
    assert_eq!(moves[0].class_idx, 0);
    assert_eq!(moves[1].class_idx, 0);

    // Last three should be for vehicle (class_idx=1)
    assert_eq!(moves[2].class_idx, 1);
    assert_eq!(moves[3].class_idx, 1);
    assert_eq!(moves[4].class_idx, 1);
}
