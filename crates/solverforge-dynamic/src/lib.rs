//! Runtime-defined solution types for SolverForge.
//!
//! This crate provides dynamic solution types that implement real SolverForge traits
//! but with schemas defined at runtime rather than compile time. This enables
//! language bindings (e.g., Python) to define problems without Rust compilation.

mod constraint;
mod constraint_set;
mod descriptor;
mod eval;
mod expr;
pub mod jit;
mod manager;
mod moves;
mod solution;
mod solve;

#[cfg(test)]
pub mod test_utils;

pub use constraint::build_from_stream_ops;
pub use constraint::StreamOp;
pub use constraint_set::DynamicConstraintSet;
pub use descriptor::{
    DynamicDescriptor, EntityClassDef, FactClassDef, FieldDef, FieldType, ValueRangeDef,
};
pub use eval::{eval_expr, EntityRef, EvalContext};
pub use expr::Expr;
pub use manager::{DynamicSolverManager, SolveStatus};
pub use moves::{DynamicChangeMove, DynamicEntityPlacer, DynamicMoveSelector};
pub use solution::{DynamicEntity, DynamicFact, DynamicSolution, DynamicValue};
pub use solve::{solve, solve_with_controls, SolveConfig, SolveResult};

use solverforge_core::score::HardSoftScore;
use solverforge_core::{ConstraintRef, ImpactType};

/// A builder for constructing dynamic constraints.
///
/// This type provides a fluent API for building constraint pipelines
/// that can be converted into incremental constraints.
#[derive(Debug, Clone)]
pub struct DynamicConstraint {
    name: String,
    ops: Vec<StreamOp>,
}

impl DynamicConstraint {
    /// Create a new constraint builder with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ops: Vec::new(),
        }
    }

    /// Iterate over all entities of a class.
    pub fn for_each(mut self, class_idx: usize) -> Self {
        self.ops.push(StreamOp::ForEach { class_idx });
        self
    }

    /// Join with another class using conditions.
    pub fn join(mut self, class_idx: usize, conditions: Vec<Expr>) -> Self {
        self.ops.push(StreamOp::Join {
            class_idx,
            conditions,
        });
        self
    }

    /// Filter tuples using a predicate.
    pub fn filter(mut self, predicate: Expr) -> Self {
        self.ops.push(StreamOp::Filter { predicate });
        self
    }

    /// Filter to distinct pairs (A < B).
    pub fn distinct_pair(mut self, ordering_expr: Expr) -> Self {
        self.ops.push(StreamOp::DistinctPair { ordering_expr });
        self
    }

    /// Penalize matching tuples with a weight.
    pub fn penalize(mut self, weight: HardSoftScore) -> Self {
        self.ops.push(StreamOp::Penalize { weight });
        self
    }

    /// Reward matching tuples with a weight.
    pub fn reward(mut self, weight: HardSoftScore) -> Self {
        self.ops.push(StreamOp::Reward { weight });
        self
    }

    /// Build into an incremental constraint.
    pub fn build(
        self,
        descriptor: DynamicDescriptor,
    ) -> Box<
        dyn solverforge_scoring::api::constraint_set::IncrementalConstraint<
                DynamicSolution,
                HardSoftScore,
            > + Send
            + Sync,
    > {
        // Determine impact type from the last operation
        let impact_type = self
            .ops
            .iter()
            .rev()
            .find_map(|op| match op {
                StreamOp::Penalize { .. } => Some(ImpactType::Penalty),
                StreamOp::Reward { .. } => Some(ImpactType::Reward),
                _ => None,
            })
            .unwrap_or(ImpactType::Penalty);

        build_from_stream_ops(
            ConstraintRef::new("", &self.name),
            impact_type,
            &self.ops,
            descriptor,
        )
    }
}
