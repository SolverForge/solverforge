//! Test utilities for solverforge-dynamic
//!
//! Provides common test fixtures for creating dynamic solutions.

use crate::descriptor::{DynamicDescriptor, EntityClassDef, FieldDef, FieldType, ValueRangeDef};
use crate::solution::{DynamicEntity, DynamicSolution, DynamicValue};

/// Creates an N-Queens solution with queens placed at the given row positions.
///
/// Each queen has two fields:
/// - `column` (I64): The fixed column position (0-indexed)
/// - `row` (I64): The assigned row position (planning variable)
///
/// # Arguments
/// * `rows` - A slice of row positions, one for each queen. The index is the column.
///
/// # Example
/// ```ignore
/// let solution = make_nqueens_solution(&[0, 2, 1, 3]); // 4 queens
/// ```
pub fn make_nqueens_solution(rows: &[i64]) -> DynamicSolution {
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

/// Creates a simple scheduling solution with shifts and employees.
///
/// # Arguments
/// * `num_shifts` - Number of shifts to create
/// * `num_employees` - Number of employees available
/// * `assignments` - Initial shift-to-employee assignments (None for unassigned)
pub fn make_schedule_solution(
    num_shifts: usize,
    num_employees: usize,
    assignments: &[Option<i64>],
) -> DynamicSolution {
    let mut desc = DynamicDescriptor::new();

    // Shift entity class
    desc.add_entity_class(EntityClassDef::new(
        "Shift",
        vec![
            FieldDef::new("id", FieldType::I64),
            FieldDef::planning_variable("employee_id", FieldType::I64, "employees"),
        ],
    ));

    // Employee fact class
    desc.add_fact_class(crate::descriptor::FactClassDef::new(
        "Employee",
        vec![FieldDef::new("id", FieldType::I64)],
    ));

    desc.add_value_range(
        "employees",
        ValueRangeDef::int_range(0, num_employees as i64),
    );

    let mut solution = DynamicSolution::new(desc);

    // Add shifts
    for (i, assignment) in assignments.iter().enumerate().take(num_shifts) {
        let employee_id = assignment.unwrap_or(-1);
        solution.add_entity(
            0,
            DynamicEntity::new(
                i as i64,
                vec![DynamicValue::I64(i as i64), DynamicValue::I64(employee_id)],
            ),
        );
    }

    // Add employees as facts
    for i in 0..num_employees {
        solution.add_fact(
            0,
            crate::solution::DynamicFact::new(i as i64, vec![DynamicValue::I64(i as i64)]),
        );
    }

    solution
}

/// Creates a minimal test solution with a single Queen entity class.
pub fn make_test_solution() -> DynamicSolution {
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
        DynamicEntity::new(1, vec![DynamicValue::I64(0), DynamicValue::I64(2)]),
    );
    solution.add_entity(
        0,
        DynamicEntity::new(2, vec![DynamicValue::I64(1), DynamicValue::I64(2)]),
    );
    solution
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_nqueens_solution() {
        let solution = make_nqueens_solution(&[0, 1, 2, 3]);
        assert_eq!(solution.entities[0].len(), 4);
    }

    #[test]
    fn test_make_schedule_solution() {
        let solution = make_schedule_solution(3, 2, &[Some(0), Some(1), None]);
        assert_eq!(solution.entities[0].len(), 3);
        assert_eq!(solution.facts[0].len(), 2);
    }

    #[test]
    fn test_make_test_solution() {
        let solution = make_test_solution();
        assert_eq!(solution.entities[0].len(), 2);
    }
}
