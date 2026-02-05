//! Tests for dynamic constraints.

mod test_bi;
mod test_cross;
mod test_flattened;
mod test_incremental;
mod test_tri;

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
