//! Closure builder functions for bi-constraint (two entity) filters and weights.

use solverforge_core::score::HardSoftScore;

use super::types::{DynBiFilter, DynBiWeight};
use crate::descriptor::DynamicDescriptor;
use crate::eval::{eval_expr, EntityRef, EvalContext};
use crate::expr::Expr;
use crate::solution::{DynamicEntity, DynamicSolution};

/// Creates a bi-constraint filter that evaluates an expression against a pair of entities.
///
/// # Arguments
/// * `filter_expr` - The expression to evaluate against the entity pair (returns bool)
/// * `class_idx` - The entity class index (for self-join constraints, both entities are from this class)
///
/// # Returns
/// A boxed closure that takes a `DynamicSolution` reference and two `DynamicEntity` references,
/// evaluates the filter expression in a bi-entity context, and returns whether the pair matches.
///
/// # Expression Context
/// The filter expression is evaluated in a tuple context where:
/// - `Param(0)` refers to the first entity (parameter `a`)
/// - `Param(1)` refers to the second entity (parameter `b`)
/// - `Field { param_idx: 0, field_idx }` accesses fields from the first entity
/// - `Field { param_idx: 1, field_idx }` accesses fields from the second entity
/// - The full solution is available for fact lookups and other operations
///
/// The expression should return a boolean value. Non-boolean results are treated as `false`.
///
/// # Implementation Note
/// Entity index lookup uses the `id_to_location` HashMap for O(1) performance.
pub fn make_bi_filter(filter_expr: Expr, class_idx: usize) -> DynBiFilter {
    Box::new(
        move |solution: &DynamicSolution, a: &DynamicEntity, b: &DynamicEntity| {
            // Look up entity indices using O(1) HashMap lookup.
            // For self-join constraints, both entities are from class_idx.
            let a_loc = solution.get_entity_location(a.id);
            let b_loc = solution.get_entity_location(b.id);

            let (Some((a_class, a_idx)), Some((b_class, b_idx))) = (a_loc, b_loc) else {
                // Entities not found in solution - shouldn't happen, but return false defensively
                return false;
            };

            // Verify entities are from the expected class
            if a_class != class_idx || b_class != class_idx {
                return false;
            }

            // Build entity tuple for evaluation context
            let tuple = vec![
                EntityRef::new(class_idx, a_idx),
                EntityRef::new(class_idx, b_idx),
            ];

            // Evaluate expression in bi-entity context
            let ctx = EvalContext::new(solution, &tuple);
            let result = eval_expr(&filter_expr, &ctx);

            // Convert result to bool (default to false if not a bool)
            result.as_bool().unwrap_or(false)
        },
    )
}

/// Creates a bi-constraint weight function that evaluates an expression against a pair of entities.
///
/// # Arguments
/// * `weight_expr` - The expression to evaluate against the entity pair (returns numeric weight)
/// * `class_idx` - The entity class index (both entities are from this class for self-join)
/// * `_descriptor` - Unused, kept for API compatibility
/// * `is_hard` - Whether this is a hard constraint (weight applied to hard score component)
///
/// # Returns
/// A boxed closure that takes a `DynamicSolution` reference and two entity indices,
/// evaluates the weight expression in a bi-entity context, and returns a `HardSoftScore`.
///
/// # Performance
/// This function uses indices into the actual solution rather than cloned entities,
/// eliminating the need for temporary solution construction and entity cloning.
///
/// # Expression Evaluation
/// The weight expression is evaluated in a tuple context where:
/// - `Param(0)` refers to the first entity (at index `a_idx`)
/// - `Param(1)` refers to the second entity (at index `b_idx`)
/// - `Field { param_idx: 0, field_idx }` accesses fields from the first entity
/// - `Field { param_idx: 1, field_idx }` accesses fields from the second entity
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
pub fn make_bi_weight(
    weight_expr: Expr,
    class_idx: usize,
    _descriptor: DynamicDescriptor,
    is_hard: bool,
) -> DynBiWeight {
    Box::new(
        move |solution: &DynamicSolution, a_idx: usize, b_idx: usize| {
            // Use the actual solution and indices directly - no cloning needed!
            // Build entity tuple for evaluation context using the actual indices
            let tuple = vec![
                EntityRef::new(class_idx, a_idx),
                EntityRef::new(class_idx, b_idx),
            ];

            // Evaluate expression in bi-entity context using the real solution
            let ctx = EvalContext::new(solution, &tuple);
            let result = eval_expr(&weight_expr, &ctx);

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
