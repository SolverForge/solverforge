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
/// Returns a boxed closure that evaluates the weight expression against three entities
/// and returns a `HardSoftScore`.
///
/// # Parameters
/// - `weight_expr`: Expression to evaluate (should return numeric value)
/// - `class_idx`: Entity class index (all three entities must be from this class)
/// - `descriptor`: Problem descriptor for creating temporary solution context
/// - `is_hard`: If true, weight is applied to hard score; otherwise soft score
///
/// # Expression Context
/// - `Param(0)` refers to the first entity (parameter `a`)
/// - `Param(1)` refers to the second entity (parameter `b`)
/// - `Param(2)` refers to the third entity (parameter `c`)
/// - `Field { param_idx: 0/1/2, field_idx }` accesses fields from respective entities
/// - Arithmetic and comparison operations work across all three entities
///
/// # Implementation
/// Creates a temporary `DynamicSolution` with all three entities for proper evaluation context.
/// This enables full tri-entity expression evaluation via `EvalContext`.
///
/// Note: This approach clones entities into a temporary solution. While this violates the
/// zero-clone principle, it's necessary because the `DynTriWeight` signature doesn't provide
/// access to the solution or entity indices. The clone happens only for matched triples
/// (bounded by match count, not total entity count).
pub fn make_tri_weight(
    weight_expr: Expr,
    class_idx: usize,
    descriptor: DynamicDescriptor,
    is_hard: bool,
) -> DynTriWeight {
    Box::new(
        move |a: &DynamicEntity, b: &DynamicEntity, c: &DynamicEntity| {
            // Create a temporary solution with the descriptor and the three entities.
            let mut temp_solution = DynamicSolution {
                descriptor: descriptor.clone(),
                entities: vec![Vec::new(); descriptor.entity_classes.len()],
                facts: Vec::new(),
                score: None,
                id_to_location: std::collections::HashMap::new(),
            };

            // Place all three entities at indices 0, 1, 2 in the class entity slice.
            temp_solution.entities[class_idx] = vec![a.clone(), b.clone(), c.clone()];

            // Build EntityRef tuple: all three entities from the same class.
            let tuple = vec![
                EntityRef::new(class_idx, 0),
                EntityRef::new(class_idx, 1),
                EntityRef::new(class_idx, 2),
            ];

            let ctx = EvalContext::new(&temp_solution, &tuple);
            let result = crate::eval::eval_expr(&weight_expr, &ctx);

            // Convert result to numeric value and apply to hard or soft score.
            let weight_num = result.as_i64().unwrap_or(0) as i64;
            if is_hard {
                HardSoftScore::of_hard(weight_num)
            } else {
                HardSoftScore::of_soft(weight_num)
            }
        },
    )
}
