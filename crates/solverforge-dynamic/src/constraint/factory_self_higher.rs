//! Factory functions for building higher-arity self-join constraints (quad and penta).

use solverforge_core::score::HardSoftScore;
use solverforge_core::{ConstraintRef, ImpactType};
use solverforge_scoring::api::constraint_set::IncrementalConstraint;
use solverforge_scoring::constraint::nary_incremental::{
    IncrementalPentaConstraint, IncrementalQuadConstraint,
};

use super::closures_extract::{make_extractor, make_key_extractor};
use super::closures_penta::{make_penta_filter, make_penta_weight};
use super::closures_quad::{make_quad_filter, make_quad_weight};
use crate::descriptor::DynamicDescriptor;
use crate::expr::Expr;
use crate::solution::DynamicSolution;

/// Builds a quad-constraint (self-join on single entity class) that returns a boxed IncrementalConstraint.
///
/// This factory creates an `IncrementalQuadConstraint` that evaluates a filter and weight
/// expression against quadruples of entities from the same class (self-join on four entities).
///
/// # Parameters
///
/// * `constraint_ref` - Reference identifying this constraint
/// * `impact_type` - Whether this constraint is a penalty or reward
/// * `class_idx` - Index of the entity class to iterate over
/// * `key_expr` - Expression to extract join key from entities (for efficient grouping)
/// * `filter_expr` - Expression to filter entity quadruples (returns bool)
/// * `weight_expr` - Expression to compute score weight for each matching quadruple
/// * `descriptor` - Dynamic descriptor for expression evaluation
/// * `is_hard` - Whether to apply weight to hard or soft score component
///
/// # Expression Context
///
/// All expressions are evaluated in a quad-entity context:
/// - `Param(0)` refers to the first entity in the quadruple
/// - `Param(1)` refers to the second entity in the quadruple
/// - `Param(2)` refers to the third entity in the quadruple
/// - `Param(3)` refers to the fourth entity in the quadruple
/// - `Field { param_idx: 0/1/2/3, field_idx }` accesses fields from respective entities
/// - Arithmetic, comparisons, and logical operations work across all four entities
///
/// # Performance Note
///
/// The join key enables efficient O(k^4) lookups where k is the average number of entities
/// per join key value, avoiding O(n^4) nested loops over all entities.
///
/// # Returns
///
/// A boxed `IncrementalConstraint<DynamicSolution, HardSoftScore>` that can be stored
/// in `DynamicConstraintSet`.
#[allow(clippy::too_many_arguments)]
pub fn build_quad_self_constraint(
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
    let filter = make_quad_filter(filter_expr, class_idx);

    // Create weight closure
    let weight = make_quad_weight(weight_expr, class_idx, descriptor, is_hard);

    // Create and box the IncrementalQuadConstraint
    Box::new(IncrementalQuadConstraint::new(
        constraint_ref,
        impact_type,
        extractor,
        key_extractor,
        filter,
        weight,
        is_hard,
    ))
}

/// Builds a penta-constraint (self-join on single entity class) that returns a boxed IncrementalConstraint.
///
/// This factory creates an `IncrementalPentaConstraint` that evaluates a filter and weight
/// expression against quintuples of entities from the same class (self-join on five entities).
///
/// # Parameters
///
/// * `constraint_ref` - Reference identifying this constraint
/// * `impact_type` - Whether this constraint is a penalty or reward
/// * `class_idx` - Index of the entity class to iterate over
/// * `key_expr` - Expression to extract join key from entities (for efficient grouping)
/// * `filter_expr` - Expression to filter entity quintuples (returns bool)
/// * `weight_expr` - Expression to compute score weight for each matching quintuple
/// * `descriptor` - Dynamic descriptor for expression evaluation
/// * `is_hard` - Whether to apply weight to hard or soft score component
///
/// # Expression Context
///
/// All expressions are evaluated in a penta-entity context:
/// - `Param(0)` refers to the first entity in the quintuple
/// - `Param(1)` refers to the second entity in the quintuple
/// - `Param(2)` refers to the third entity in the quintuple
/// - `Param(3)` refers to the fourth entity in the quintuple
/// - `Param(4)` refers to the fifth entity in the quintuple
/// - `Field { param_idx: 0/1/2/3/4, field_idx }` accesses fields from respective entities
/// - Arithmetic, comparisons, and logical operations work across all five entities
///
/// # Performance Note
///
/// The join key enables efficient O(k^5) lookups where k is the average number of entities
/// per join key value, avoiding O(n^5) nested loops over all entities.
///
/// # Returns
///
/// A boxed `IncrementalConstraint<DynamicSolution, HardSoftScore>` that can be stored
/// in `DynamicConstraintSet`.
#[allow(clippy::too_many_arguments)]
pub fn build_penta_self_constraint(
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
    let filter = make_penta_filter(filter_expr, class_idx);

    // Create weight closure
    let weight = make_penta_weight(weight_expr, class_idx, descriptor, is_hard);

    // Create and box the IncrementalPentaConstraint
    Box::new(IncrementalPentaConstraint::new(
        constraint_ref,
        impact_type,
        extractor,
        key_extractor,
        filter,
        weight,
        is_hard,
    ))
}
