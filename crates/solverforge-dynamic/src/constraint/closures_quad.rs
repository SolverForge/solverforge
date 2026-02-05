//! Closure builder functions for quad-constraint (four entity) filters and weights.

use solverforge_core::score::HardSoftScore;

use super::types::{DynQuadFilter, DynQuadWeight};
use crate::descriptor::DynamicDescriptor;
use crate::eval::{EntityRef, EvalContext};
use crate::expr::Expr;
use crate::solution::{DynamicEntity, DynamicSolution};

// Note: DynamicDescriptor is kept in make_quad_weight signature for API consistency,
// even though it's no longer needed after switching to solution+indices pattern.

/// Creates a quad-entity filter closure from a filter expression.
///
/// Returns a boxed closure that evaluates the filter expression against four entities
/// and returns `true` if they match.
///
/// # Parameters
/// - `filter_expr`: Expression to evaluate (should return bool)
/// - `class_idx`: Entity class index (all four entities must be from this class)
///
/// # Expression Context
/// - `Param(0)` refers to the first entity (parameter `a`)
/// - `Param(1)` refers to the second entity (parameter `b`)
/// - `Param(2)` refers to the third entity (parameter `c`)
/// - `Param(3)` refers to the fourth entity (parameter `d`)
/// - `Field { param_idx: 0/1/2/3, field_idx }` accesses fields from respective entities
/// - The full solution is available for fact lookups and other operations
///
/// # Implementation
/// The closure searches the entity slice by entity ID to find indices, then builds an
/// `EntityRef` tuple and evaluates the expression using `EvalContext`. This is O(n) per
/// entity but acceptable because filtering is performed only on entities already matched
/// by join key, not the full entity set.
pub fn make_quad_filter(filter_expr: Expr, class_idx: usize) -> DynQuadFilter {
    Box::new(
        move |solution: &DynamicSolution,
              a: &DynamicEntity,
              b: &DynamicEntity,
              c: &DynamicEntity,
              d: &DynamicEntity| {
            // Find entity indices by searching the entity slice using entity IDs.
            let entities = solution
                .entities
                .get(class_idx)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            let a_idx = entities.iter().position(|e| e.id == a.id);
            let b_idx = entities.iter().position(|e| e.id == b.id);
            let c_idx = entities.iter().position(|e| e.id == c.id);
            let d_idx = entities.iter().position(|e| e.id == d.id);

            // If any entities not found, filter fails.
            let (Some(a_idx), Some(b_idx), Some(c_idx), Some(d_idx)) = (a_idx, b_idx, c_idx, d_idx)
            else {
                return false;
            };

            // Build EntityRef tuple: all four entities from the same class.
            let tuple = vec![
                EntityRef::new(class_idx, a_idx),
                EntityRef::new(class_idx, b_idx),
                EntityRef::new(class_idx, c_idx),
                EntityRef::new(class_idx, d_idx),
            ];

            let ctx = EvalContext::new(solution, &tuple);
            let result = crate::eval::eval_expr(&filter_expr, &ctx);

            // Return true only if result is a boolean true.
            result.as_bool().unwrap_or(false)
        },
    )
}

/// Creates a quad-entity weight closure from a weight expression.
///
/// Returns a boxed closure that evaluates the weight expression against four entities
/// and returns a `HardSoftScore`.
///
/// # Parameters
/// - `weight_expr`: Expression to evaluate (should return numeric value)
/// - `class_idx`: Entity class index (all four entities must be from this class)
/// - `_descriptor`: Problem descriptor (unused, kept for API consistency)
/// - `is_hard`: If true, weight is applied to hard score; otherwise soft score
///
/// # Expression Context
/// - `Param(0)` refers to the first entity (parameter `a`)
/// - `Param(1)` refers to the second entity (parameter `b`)
/// - `Param(2)` refers to the third entity (parameter `c`)
/// - `Param(3)` refers to the fourth entity (parameter `d`)
/// - `Field { param_idx: 0/1/2/3, field_idx }` accesses fields from respective entities
/// - Arithmetic and comparison operations work across all four entities
///
/// # Implementation
/// Takes the solution reference and entity indices directly, avoiding entity cloning.
/// The indices refer to positions within `solution.entities[class_idx]`.
pub fn make_quad_weight(
    weight_expr: Expr,
    class_idx: usize,
    _descriptor: DynamicDescriptor,
    is_hard: bool,
) -> DynQuadWeight {
    Box::new(
        move |solution: &DynamicSolution,
              a_idx: usize,
              b_idx: usize,
              c_idx: usize,
              d_idx: usize| {
            // Build EntityRef tuple using the provided indices.
            let tuple = vec![
                EntityRef::new(class_idx, a_idx),
                EntityRef::new(class_idx, b_idx),
                EntityRef::new(class_idx, c_idx),
                EntityRef::new(class_idx, d_idx),
            ];

            let ctx = EvalContext::new(solution, &tuple);
            let result = crate::eval::eval_expr(&weight_expr, &ctx);

            // Convert result to numeric value and apply to hard or soft score.
            let weight_num = result.as_i64().unwrap_or(0);
            if is_hard {
                HardSoftScore::of_hard(weight_num)
            } else {
                HardSoftScore::of_soft(weight_num)
            }
        },
    )
}
