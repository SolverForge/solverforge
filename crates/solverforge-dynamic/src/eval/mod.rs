//! Expression evaluation for runtime constraint checking.

#[cfg(test)]
mod tests;

mod compare;
mod eval_entity;
mod evaluator;

use crate::solution::{DynamicFact, DynamicSolution, DynamicValue};

// Re-export evaluation functions
pub use eval_entity::eval_entity_expr;
pub use evaluator::eval_expr;

/// Reference to an entity in the tuple being evaluated.
#[derive(Debug, Clone, Copy)]
pub struct EntityRef {
    /// Class index.
    pub class_idx: usize,
    /// Entity index within the class.
    pub entity_idx: usize,
}

impl EntityRef {
    /// Creates a new entity reference.
    pub fn new(class_idx: usize, entity_idx: usize) -> Self {
        Self {
            class_idx,
            entity_idx,
        }
    }
}

/// Context for expression evaluation.
pub struct EvalContext<'a> {
    /// The solution being evaluated.
    pub solution: &'a DynamicSolution,
    /// The tuple of entity references being matched.
    pub tuple: &'a [EntityRef],
    /// Optional flattened value from FlattenLast operation.
    pub flattened_value: Option<&'a DynamicValue>,
}

impl<'a> EvalContext<'a> {
    /// Creates a new evaluation context.
    pub fn new(solution: &'a DynamicSolution, tuple: &'a [EntityRef]) -> Self {
        Self {
            solution,
            tuple,
            flattened_value: None,
        }
    }

    /// Creates a new evaluation context with a flattened value.
    /// The flattened value is accessible via `Param(2)` in expressions.
    pub fn with_flattened(
        solution: &'a DynamicSolution,
        tuple: &'a [EntityRef],
        flattened_value: &'a DynamicValue,
    ) -> Self {
        Self {
            solution,
            tuple,
            flattened_value: Some(flattened_value),
        }
    }

    /// Gets an entity from the tuple by parameter index.
    pub fn get_entity(&self, param_idx: usize) -> Option<&'a crate::solution::DynamicEntity> {
        let entity_ref = self.tuple.get(param_idx)?;
        self.solution
            .get_entity(entity_ref.class_idx, entity_ref.entity_idx)
    }

    /// Gets a fact by class index and fact index.
    pub fn get_fact(&self, class_idx: usize, fact_idx: usize) -> Option<&'a DynamicFact> {
        self.solution.get_fact(class_idx, fact_idx)
    }
}
