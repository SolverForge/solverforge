//! Factory functions for building self-join bi and tri constraints.

use solverforge_core::score::HardSoftScore;
use solverforge_core::{ConstraintRef, ImpactType};
use solverforge_scoring::api::constraint_set::IncrementalConstraint;
use solverforge_scoring::constraint::nary_incremental::{
    IncrementalBiConstraint, IncrementalTriConstraint,
};

use super::closures_bi::{make_bi_filter, make_bi_weight};
use super::closures_extract::{make_extractor, make_key_extractor};
use super::closures_tri::{make_tri_filter, make_tri_weight};
use crate::descriptor::DynamicDescriptor;
use crate::expr::Expr;
use crate::solution::DynamicSolution;

/// Builds a bi-constraint (self-join on single entity class) that returns a boxed IncrementalConstraint.
///
/// This factory creates an `IncrementalBiConstraint` that evaluates a filter and weight
/// expression against pairs of entities from the same class (self-join).
///
/// # Parameters
///
/// * `constraint_ref` - Reference identifying this constraint
/// * `impact_type` - Whether this constraint is a penalty or reward
/// * `class_idx` - Index of the entity class to iterate over
/// * `key_expr` - Expression to extract join key from entities (for efficient pairing)
/// * `filter_expr` - Expression to filter entity pairs (returns bool)
/// * `weight_expr` - Expression to compute score weight for each matching pair
/// * `descriptor` - Dynamic descriptor for expression evaluation
/// * `is_hard` - Whether to apply weight to hard or soft score component
///
/// # Expression Context
///
/// All expressions are evaluated in a bi-entity context:
/// - `Param(0)` refers to the first entity in the pair
/// - `Param(1)` refers to the second entity in the pair
/// - `Field { param_idx: 0, field_idx }` accesses fields from the first entity
/// - `Field { param_idx: 1, field_idx }` accesses fields from the second entity
/// - Arithmetic, comparisons, and logical operations work across both entities
///
/// # Returns
///
/// A boxed `IncrementalConstraint<DynamicSolution, HardSoftScore>` that can be stored
/// in `DynamicConstraintSet`.
#[allow(clippy::too_many_arguments)]
pub fn build_bi_self_constraint(
    constraint_ref: ConstraintRef,
    impact_type: ImpactType,
    class_idx: usize,
    key_expr: Expr,
    filter_expr: Expr,
    weight_expr: Expr,
    descriptor: DynamicDescriptor,
    is_hard: bool,
) -> Box<dyn IncrementalConstraint<DynamicSolution, HardSoftScore> + Send + Sync> {
    // Create extractor for the entity class
    let extractor = make_extractor(class_idx);

    // Create key extractor
    let key_extractor = make_key_extractor(key_expr, descriptor.clone());

    // Create filter closure
    let filter = make_bi_filter(filter_expr, class_idx);

    // Create weight closure
    let weight = make_bi_weight(weight_expr, class_idx, descriptor, is_hard);

    // Create and box the IncrementalBiConstraint
    Box::new(IncrementalBiConstraint::new(
        constraint_ref,
        impact_type,
        extractor,
        key_extractor,
        filter,
        weight,
        is_hard,
    ))
}

/// Builds a tri-constraint (self-join on single entity class) that returns a boxed IncrementalConstraint.
///
/// This factory creates an `IncrementalTriConstraint` that evaluates a filter and weight
/// expression against triples of entities from the same class (self-join on three entities).
///
/// # Parameters
///
/// * `constraint_ref` - Reference identifying this constraint
/// * `impact_type` - Whether this constraint is a penalty or reward
/// * `class_idx` - Index of the entity class to iterate over
/// * `key_expr` - Expression to extract join key from entities (for efficient grouping)
/// * `filter_expr` - Expression to filter entity triples (returns bool)
/// * `weight_expr` - Expression to compute score weight for each matching triple
/// * `descriptor` - Dynamic descriptor for expression evaluation
/// * `is_hard` - Whether to apply weight to hard or soft score component
///
/// # Expression Context
///
/// All expressions are evaluated in a tri-entity context:
/// - `Param(0)` refers to the first entity in the triple
/// - `Param(1)` refers to the second entity in the triple
/// - `Param(2)` refers to the third entity in the triple
/// - `Field { param_idx: 0/1/2, field_idx }` accesses fields from respective entities
/// - Arithmetic, comparisons, and logical operations work across all three entities
///
/// # Returns
///
/// A boxed `IncrementalConstraint<DynamicSolution, HardSoftScore>` that can be stored
/// in `DynamicConstraintSet`.
#[allow(clippy::too_many_arguments)]
pub fn build_tri_self_constraint(
    constraint_ref: ConstraintRef,
    impact_type: ImpactType,
    class_idx: usize,
    key_expr: Expr,
    filter_expr: Expr,
    weight_expr: Expr,
    descriptor: DynamicDescriptor,
    is_hard: bool,
) -> Box<dyn IncrementalConstraint<DynamicSolution, HardSoftScore> + Send + Sync> {
    // Create extractor for the entity class
    let extractor = make_extractor(class_idx);

    // Create key extractor
    let key_extractor = make_key_extractor(key_expr, descriptor.clone());

    // Create filter closure
    let filter = make_tri_filter(filter_expr, class_idx);

    // Create weight closure
    let weight = make_tri_weight(weight_expr, class_idx, descriptor, is_hard);

    // Create and box the IncrementalTriConstraint
    Box::new(IncrementalTriConstraint::new(
        constraint_ref,
        impact_type,
        extractor,
        key_extractor,
        filter,
        weight,
        is_hard,
    ))
}
