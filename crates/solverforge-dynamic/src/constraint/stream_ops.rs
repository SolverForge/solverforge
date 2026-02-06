//! Stream operations and constraint pattern building from pipelines.

use solverforge_core::score::HardSoftScore;
use solverforge_core::{ConstraintRef, ImpactType};
use solverforge_scoring::api::constraint_set::IncrementalConstraint;

use super::factory_cross::{build_cross_bi_constraint, build_flattened_bi_constraint};
use super::factory_self::{build_bi_self_constraint, build_tri_self_constraint};
use super::factory_self_higher::{build_penta_self_constraint, build_quad_self_constraint};
use super::factory_uni::build_uni_constraint;
use super::stream_parser::{parse_stream_ops, ConstraintPattern};
use crate::descriptor::DynamicDescriptor;
use crate::expr::Expr;
use crate::solution::DynamicSolution;

/// Operations in a constraint stream pipeline.
#[derive(Debug, Clone)]
pub enum StreamOp {
    /// Iterate over all entities of a class.
    ForEach { class_idx: usize },

    /// Filter entities using a predicate expression.
    Filter { predicate: Expr },

    /// Join with another class using join conditions.
    Join {
        class_idx: usize,
        /// Join conditions that must all be true.
        conditions: Vec<Expr>,
    },

    /// Filter distinct pairs (ensuring A < B to avoid duplicates).
    DistinctPair {
        /// Expression to compare (e.g., entity IDs or indices).
        ordering_expr: Expr,
    },

    /// Penalize matching tuples.
    Penalize { weight: HardSoftScore },

    /// Penalize with a configurable amount based on expression.
    PenalizeConfigurable { match_weight: Expr },

    /// Reward matching tuples.
    Reward { weight: HardSoftScore },

    /// Reward with a configurable amount based on expression.
    RewardConfigurable { match_weight: Expr },

    /// Flatten a set/list field, creating one tuple per element.
    FlattenLast {
        /// Expression to get the set/list to flatten.
        set_expr: Expr,
    },
}

/// Builds a boxed incremental constraint from a stream operation pipeline.
///
/// This function analyzes the pipeline to determine:
/// - **Arity**: How many entities are involved (uni/bi/tri/quad/penta) based on join count
/// - **Join type**: Self-join (same class) vs cross-join (different classes)
/// - **Flattened**: Whether a `FlattenLast` operation is present
///
/// Then calls the appropriate factory function from the constraint module.
///
/// # Arguments
///
/// * `constraint_ref` - Unique identifier for this constraint
/// * `impact_type` - Whether this is a penalty or reward
/// * `ops` - Pipeline of stream operations (ForEach, Join, Filter, Penalize, etc.)
/// * `descriptor` - Solution schema for expression evaluation
///
/// # Returns
///
/// A boxed `IncrementalConstraint` wrapping a monomorphized implementation.
///
/// # Example Pipeline Patterns
///
/// ```
/// use solverforge_dynamic::{StreamOp, Expr};
/// use solverforge_core::score::HardSoftScore;
///
/// // Uni-constraint (1 entity): filter and penalize
/// let uni_ops = vec![
///     StreamOp::ForEach { class_idx: 0 },
///     StreamOp::Filter { predicate: Expr::bool(true) },
///     StreamOp::Penalize { weight: HardSoftScore::of_hard(1) },
/// ];
///
/// // Bi self-join (2 entities, same class): detect conflicts
/// let bi_self_ops = vec![
///     StreamOp::ForEach { class_idx: 0 },
///     StreamOp::Join { class_idx: 0, conditions: vec![Expr::eq(Expr::field(0, 1), Expr::field(1, 1))] },
///     StreamOp::DistinctPair { ordering_expr: Expr::lt(Expr::field(0, 0), Expr::field(1, 0)) },
///     StreamOp::Penalize { weight: HardSoftScore::of_hard(1) },
/// ];
///
/// // Cross-join (2 entities, different classes)
/// let cross_ops = vec![
///     StreamOp::ForEach { class_idx: 0 },
///     StreamOp::Join { class_idx: 1, conditions: vec![Expr::eq(Expr::field(0, 1), Expr::field(1, 0))] },
///     StreamOp::Penalize { weight: HardSoftScore::of_soft(1) },
/// ];
///
/// // Flattened (entity A + collection from entity B)
/// let flattened_ops = vec![
///     StreamOp::ForEach { class_idx: 0 },
///     StreamOp::Join { class_idx: 1, conditions: vec![] },
///     StreamOp::FlattenLast { set_expr: Expr::field(1, 2) },
///     StreamOp::Penalize { weight: HardSoftScore::of_hard(1) },
/// ];
/// ```
pub fn build_from_stream_ops(
    constraint_ref: ConstraintRef,
    impact_type: ImpactType,
    ops: &[StreamOp],
    descriptor: DynamicDescriptor,
) -> Box<dyn IncrementalConstraint<DynamicSolution, HardSoftScore> + Send + Sync> {
    // Parse the pipeline to extract constraint pattern
    let pattern = parse_stream_ops(ops);

    // Determine if hard or soft score based on the weight in the terminal operation.
    // A weight with nonzero hard component targets the hard score; otherwise soft.
    let is_hard = ops
        .iter()
        .rev()
        .find_map(|op| match op {
            StreamOp::Penalize { weight } | StreamOp::Reward { weight } => Some(weight.hard() != 0),
            StreamOp::PenalizeConfigurable { .. } | StreamOp::RewardConfigurable { .. } => {
                // Configurable weights are evaluated at runtime; default to hard
                Some(true)
            }
            _ => None,
        })
        .unwrap_or(true);

    // Call the appropriate factory based on detected pattern
    match pattern {
        ConstraintPattern::Uni {
            class_idx,
            filter_expr,
            weight_expr,
        } => build_uni_constraint(
            constraint_ref,
            impact_type,
            class_idx,
            filter_expr,
            weight_expr,
            descriptor,
            is_hard,
        ),

        ConstraintPattern::BiSelfJoin {
            class_idx,
            key_expr,
            filter_expr,
            weight_expr,
        } => build_bi_self_constraint(
            constraint_ref,
            impact_type,
            class_idx,
            key_expr,
            filter_expr,
            weight_expr,
            descriptor,
            is_hard,
        ),

        ConstraintPattern::TriSelfJoin {
            class_idx,
            key_expr,
            filter_expr,
            weight_expr,
        } => build_tri_self_constraint(
            constraint_ref,
            impact_type,
            class_idx,
            key_expr,
            filter_expr,
            weight_expr,
            descriptor,
            is_hard,
        ),

        ConstraintPattern::QuadSelfJoin {
            class_idx,
            key_expr,
            filter_expr,
            weight_expr,
        } => build_quad_self_constraint(
            constraint_ref,
            impact_type,
            class_idx,
            key_expr,
            filter_expr,
            weight_expr,
            descriptor,
            is_hard,
        ),

        ConstraintPattern::PentaSelfJoin {
            class_idx,
            key_expr,
            filter_expr,
            weight_expr,
        } => build_penta_self_constraint(
            constraint_ref,
            impact_type,
            class_idx,
            key_expr,
            filter_expr,
            weight_expr,
            descriptor,
            is_hard,
        ),

        ConstraintPattern::CrossBi {
            class_idx_a,
            class_idx_b,
            key_expr_a,
            key_expr_b,
            filter_expr,
            weight_expr,
        } => build_cross_bi_constraint(
            constraint_ref,
            impact_type,
            class_idx_a,
            class_idx_b,
            key_expr_a,
            key_expr_b,
            filter_expr,
            weight_expr,
            descriptor,
            is_hard,
        ),

        ConstraintPattern::FlattenedBi {
            class_idx_a,
            class_idx_b,
            key_expr_a,
            key_expr_b,
            flatten_expr,
            c_key_expr,
            a_lookup_expr,
            filter_expr,
            weight_expr,
        } => build_flattened_bi_constraint(
            constraint_ref,
            impact_type,
            class_idx_a,
            class_idx_b,
            key_expr_a,
            key_expr_b,
            flatten_expr,
            c_key_expr,
            a_lookup_expr,
            filter_expr,
            weight_expr,
            descriptor,
            is_hard,
        ),
    }
}
