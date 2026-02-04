//! Closure builder functions for tri-constraint (three entity) filters and weights.

use solverforge_core::score::HardSoftScore;

use super::types::{DynTriFilter, DynTriWeight};
use crate::descriptor::DynamicDescriptor;
use crate::eval::{EntityRef, EvalContext};
use crate::expr::Expr;
use crate::solution::{DynamicEntity, DynamicSolution};

/// Creates a tri-entity filter closure from a filter expression.
///
/// Returns a boxed closure that evaluates the filter expression against three entities
/// from the same class (self-join with three-way matching).
///
/// # Parameters
/// - `filter_expr`: Expression to evaluate (should return bool)
/// - `class_idx`: Entity class index (all three entities must be from this class)
///
/// # Expression Context
/// - `Param(0)` refers to the first entity (parameter `a`)
/// - `Param(1)` refers to the second entity (parameter `b`)
/// - `Param(2)` refers to the third entity (parameter `c`)
/// - `Field { param_idx: 0/1/2, field_idx }` accesses fields from respective entities
/// - The full solution is available for fact lookups
///
/// # Implementation Note
/// Entity index lookup uses the `id_to_location` HashMap for O(1) performance.
pub fn make_tri_filter(filter_expr: Expr, class_idx: usize) -> DynTriFilter {
    Box::new(
        move |solution: &DynamicSolution,
              a: &DynamicEntity,
              b: &DynamicEntity,
              c: &DynamicEntity| {
            // Look up entity indices using O(1) HashMap lookup.
            // For self-join constraints, all three entities are from class_idx.
            let a_loc = solution.get_entity_location(a.id);
            let b_loc = solution.get_entity_location(b.id);
            let c_loc = solution.get_entity_location(c.id);

            let (Some((a_class, a_idx)), Some((b_class, b_idx)), Some((c_class, c_idx))) =
                (a_loc, b_loc, c_loc)
            else {
                // Entities not found in solution - shouldn't happen, but return false defensively
                return false;
            };

            // Verify entities are from the expected class
            if a_class != class_idx || b_class != class_idx || c_class != class_idx {
                return false;
            }

            // Build EntityRef tuple: all three entities from the same class.
            let tuple = vec![
                EntityRef::new(class_idx, a_idx),
                EntityRef::new(class_idx, b_idx),
                EntityRef::new(class_idx, c_idx),
            ];

            let ctx = EvalContext::new(solution, &tuple);
            crate::eval::eval_expr(&filter_expr, &ctx)
                .as_bool()
                .unwrap_or(false)
        },
    )
}

/// Creates a tri-entity weight closure from a weight expression.
///
/// # Arguments
/// * `weight_expr` - The expression to evaluate against the entity triple (returns numeric weight)
/// * `class_idx` - The entity class index (all three entities are from this class for self-join)
/// * `_descriptor` - Unused, kept for API compatibility
/// * `is_hard` - Whether this is a hard constraint (weight applied to hard score component)
///
/// # Returns
/// A boxed closure that takes a `DynamicSolution` reference and three entity indices,
/// evaluates the weight expression in a tri-entity context, and returns a `HardSoftScore`.
///
/// # Performance
/// This function uses indices into the actual solution rather than cloned entities,
/// eliminating the need for temporary solution construction and entity cloning.
///
/// # Expression Evaluation
/// The weight expression is evaluated in a tuple context where:
/// - `Param(0)` refers to the first entity (at index `a_idx`)
/// - `Param(1)` refers to the second entity (at index `b_idx`)
/// - `Param(2)` refers to the third entity (at index `c_idx`)
/// - `Field { param_idx: 0/1/2, field_idx }` accesses fields from respective entities
///
/// The expression should return a numeric value (i64). Non-numeric results default to 0.
///
/// Weight expressions can reference facts and other solution state since the full solution
/// is available during evaluation.
///
/// # Weight Application
/// The resulting numeric value is applied to either the hard or soft score component based on
/// the `is_hard` parameter. The constraint's impact type (penalty vs reward) is NOT applied
/// here - that's handled by the monomorphized constraint wrapper's `compute_score` method.
pub fn make_tri_weight(
    weight_expr: Expr,
    class_idx: usize,
    _descriptor: DynamicDescriptor,
    is_hard: bool,
) -> DynTriWeight {
    Box::new(
        move |solution: &DynamicSolution, a_idx: usize, b_idx: usize, c_idx: usize| {
            // Use the actual solution and indices directly - no cloning needed!
            // Build entity tuple for evaluation context using the actual indices
            let tuple = vec![
                EntityRef::new(class_idx, a_idx),
                EntityRef::new(class_idx, b_idx),
                EntityRef::new(class_idx, c_idx),
            ];

            // Evaluate expression in tri-entity context using the real solution
            let ctx = EvalContext::new(solution, &tuple);
            let result = crate::eval::eval_expr(&weight_expr, &ctx);

            // Convert to numeric value (default to 0 if not numeric)
            let weight_num = result.as_i64().unwrap_or(0) as f64;

            // Apply to hard or soft component
            if is_hard {
                HardSoftScore::of_hard(weight_num as i64)
            } else {
                HardSoftScore::of_soft(weight_num as i64)
            }
        },
    )
}
