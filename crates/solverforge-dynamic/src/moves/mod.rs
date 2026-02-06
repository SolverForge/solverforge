//! Move generation for dynamic solutions.

mod change_move;
mod either_move;
mod entity_placer;
mod iterator;
mod selector;
mod swap_move;

#[cfg(test)]
mod tests;

use crate::descriptor::ValueRangeDef;
use crate::solution::{DynamicSolution, DynamicValue};

pub use change_move::DynamicChangeMove;
pub use either_move::DynamicEitherMove;
pub use entity_placer::DynamicEntityPlacer;
pub use iterator::DynamicMoveIterator;
pub use selector::DynamicMoveSelector;
pub use swap_move::DynamicSwapMove;

fn get_range_values(range: &ValueRangeDef, solution: &DynamicSolution) -> Vec<DynamicValue> {
    match range {
        ValueRangeDef::Explicit(values) => values.clone(),
        ValueRangeDef::IntRange { start, end } => (*start..*end).map(DynamicValue::I64).collect(),
        ValueRangeDef::EntityClass(class_idx) => {
            let count = solution
                .entities
                .get(*class_idx)
                .map(|e| e.len())
                .unwrap_or(0);
            (0..count)
                .map(|i| DynamicValue::Ref(*class_idx, i))
                .collect()
        }
        ValueRangeDef::FactClass(class_idx) => {
            let count = solution.facts.get(*class_idx).map(|f| f.len()).unwrap_or(0);
            (0..count)
                .map(|i| DynamicValue::FactRef(*class_idx, i))
                .collect()
        }
    }
}
