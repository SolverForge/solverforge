//! Factory functions for building cross-join and flattened bi-constraints.

use solverforge_core::score::HardSoftScore;
use solverforge_core::{ConstraintRef, ImpactType};
use solverforge_scoring::api::constraint_set::IncrementalConstraint;
use solverforge_scoring::constraint::cross_bi_incremental::IncrementalCrossBiConstraint;
use solverforge_scoring::constraint::flattened_bi::FlattenedBiConstraint;

use super::closures_cross::{
    make_cross_extractor_a, make_cross_extractor_b, make_cross_filter, make_cross_key_a,
    make_cross_key_b, make_cross_weight,
};
use super::closures_flattened::{
    make_a_lookup, make_c_key_fn, make_flatten, make_flattened_filter, make_flattened_weight,
};
use crate::descriptor::DynamicDescriptor;
use crate::expr::Expr;
use crate::solution::DynamicSolution;

/// Factory function for cross-bi-constraints (cross-join between two different entity classes).
///
/// This creates an `IncrementalCrossBiConstraint` that evaluates pairs of entities from
/// **two different classes** (A and B), joined by matching key values.
///
/// # Arguments
///
/// * `constraint_ref` - Unique identifier for the constraint
/// * `impact_type` - Whether this constraint is a penalty or reward
/// * `class_idx_a` - Index of the first entity class (class A)
/// * `class_idx_b` - Index of the second entity class (class B)
/// * `key_expr_a` - Expression to extract join key from entities of class A
/// * `key_expr_b` - Expression to extract join key from entities of class B
/// * `filter_expr` - Expression to evaluate whether a pair (A, B) matches
/// * `weight_expr` - Expression to compute the score for a matched pair
/// * `descriptor` - The dynamic descriptor (for expression evaluation context)
/// * `is_hard` - Whether to apply the weight to hard or soft score component
///
/// # Expression Context
///
/// All expressions are evaluated in a cross-bi-entity context:
/// - `Param(0)` refers to the entity from class A
/// - `Param(1)` refers to the entity from class B
/// - `Field { param_idx: 0, field_idx }` accesses fields from the A entity
/// - `Field { param_idx: 1, field_idx }` accesses fields from the B entity
///
/// # Performance Note
///
/// The join key enables efficient O(k_a * k_b) lookups where k_a and k_b are the average
/// numbers of entities per join key value in classes A and B respectively.
///
/// # Returns
///
/// A boxed `IncrementalConstraint<DynamicSolution, HardSoftScore>` that can be stored
/// in `DynamicConstraintSet`.
#[allow(clippy::too_many_arguments)]
pub fn build_cross_bi_constraint(
    constraint_ref: ConstraintRef,
    impact_type: ImpactType,
    class_idx_a: usize,
    class_idx_b: usize,
    key_expr_a: Expr,
    key_expr_b: Expr,
    filter_expr: Expr,
    weight_expr: Expr,
    descriptor: DynamicDescriptor,
    is_hard: bool,
) -> Box<dyn IncrementalConstraint<DynamicSolution, HardSoftScore> + Send + Sync> {
    // Create extractors for both entity classes
    let extractor_a = make_cross_extractor_a(class_idx_a);
    let extractor_b = make_cross_extractor_b(class_idx_b);

    // Create key extractors for both classes
    let key_a = make_cross_key_a(key_expr_a, descriptor.clone());
    let key_b = make_cross_key_b(key_expr_b, descriptor.clone());

    // Create filter closure
    let filter = make_cross_filter(filter_expr, class_idx_a, class_idx_b);

    // Create weight closure
    let weight = make_cross_weight(weight_expr, class_idx_a, class_idx_b, descriptor, is_hard);

    // Create and box the IncrementalCrossBiConstraint
    Box::new(IncrementalCrossBiConstraint::new(
        constraint_ref,
        impact_type,
        extractor_a,
        extractor_b,
        key_a,
        key_b,
        filter,
        weight,
        is_hard,
    ))
}

/// Builds a flattened bi-constraint for O(1) lookups on entity-to-collection joins.
///
/// Flattened constraints optimize scenarios where:
/// - Entity A joins with entity B by key
/// - Entity B contains a collection field that's expanded (flattened) into individual C items
/// - We need to check if any C item matches some criterion based on entity A
///
/// Instead of O(|B| * |C_avg|) nested loops on each A entity change, this:
/// 1. Pre-indexes all C items by (join_key, c_key) during initialize
/// 2. On A entity change, looks up matching C items in O(1) using A's lookup key
///
/// # Parameters
///
/// * `constraint_ref` - Unique constraint identifier
/// * `impact_type` - Whether this is a penalty or reward
/// * `class_idx_a` - Index of entity class A (the planning entity, e.g., Shift)
/// * `class_idx_b` - Index of entity class B (the entity with collection field, e.g., Employee)
/// * `key_expr_a` - Expression to extract join key from A entity (e.g., `assigned_employee_id`)
/// * `key_expr_b` - Expression to extract join key from B entity (e.g., `employee_id`)
/// * `flatten_expr` - Expression to extract collection field from B (returns list of C items)
/// * `c_key_expr` - Expression to extract index key from flattened item C (for O(1) lookup)
/// * `a_lookup_expr` - Expression to extract lookup key from A entity (for O(1) index access)
/// * `filter_expr` - Filter predicate on (A entity, C item) pairs
/// * `weight_expr` - Weight expression on (A entity, C item) pairs
/// * `descriptor` - Dynamic descriptor for expression evaluation
/// * `is_hard` - Whether this is a hard constraint
///
/// # Returns
///
/// A boxed `IncrementalConstraint` wrapping `FlattenedBiConstraint`
///
/// # Performance
///
/// - **Initialize**: O(|B| * |C_avg|) to build the index once
/// - **on_insert(A)**: O(1) lookup in the index using (join_key, a_lookup_key)
/// - **on_retract(A)**: O(1) lookup to remove cached score
/// - **on_insert(B)** or **on_retract(B)**: O(|A_with_key| * |C_avg|) to recompute affected A entities
#[allow(clippy::too_many_arguments)]
pub fn build_flattened_bi_constraint(
    constraint_ref: ConstraintRef,
    impact_type: ImpactType,
    class_idx_a: usize,
    class_idx_b: usize,
    key_expr_a: Expr,
    key_expr_b: Expr,
    flatten_expr: Expr,
    c_key_expr: Expr,
    a_lookup_expr: Expr,
    filter_expr: Expr,
    weight_expr: Expr,
    descriptor: DynamicDescriptor,
    is_hard: bool,
) -> Box<dyn IncrementalConstraint<DynamicSolution, HardSoftScore> + Send + Sync> {
    // Create extractors for both entity classes
    let extractor_a = make_cross_extractor_a(class_idx_a);
    let extractor_b = make_cross_extractor_b(class_idx_b);

    // Create key extractors for both classes (for join)
    let key_a = make_cross_key_a(key_expr_a, descriptor.clone());
    let key_b = make_cross_key_b(key_expr_b, descriptor.clone());

    // Create flatten function (B entity -> slice of C items)
    let flatten = make_flatten(flatten_expr, descriptor.clone());

    // Create C key function (C item -> index key)
    let c_key_fn = make_c_key_fn(c_key_expr, descriptor.clone());

    // Create A lookup function (A entity -> lookup key for index access)
    let a_lookup_fn = make_a_lookup(a_lookup_expr, descriptor.clone());

    // Create filter closure (solution, A entity, C item -> bool)
    let filter = make_flattened_filter(filter_expr, class_idx_a, class_idx_b);

    // Create weight closure (A entity, C item -> score)
    let weight = make_flattened_weight(weight_expr, class_idx_a, class_idx_b, descriptor, is_hard);

    // Create and box the FlattenedBiConstraint
    Box::new(FlattenedBiConstraint::new(
        constraint_ref,
        impact_type,
        extractor_a,
        extractor_b,
        key_a,
        key_b,
        flatten,
        c_key_fn,
        a_lookup_fn,
        filter,
        weight,
        is_hard,
    ))
}
