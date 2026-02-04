//! Closure builder functions for penta-constraint (five entity) filters and weights.

use solverforge_core::score::HardSoftScore;

use super::types::{DynPentaFilter, DynPentaWeight};
use crate::descriptor::DynamicDescriptor;
use crate::eval::{EntityRef, EvalContext};
use crate::expr::Expr;
use crate::solution::{DynamicEntity, DynamicSolution};

/// Creates a penta-entity filter closure from a filter expression.
///
/// Returns a boxed closure that evaluates the filter expression against five entities
/// and returns `true` if they satisfy the filter condition.
///
/// # Parameters
/// - `filter_expr`: Expression to evaluate (should return boolean)
/// - `class_idx`: Entity class index (all five entities must be from this class)
///
/// # Expression Context
/// - `Param(0)` refers to the first entity (parameter `a`)
/// - `Param(1)` refers to the second entity (parameter `b`)
/// - `Param(2)` refers to the third entity (parameter `c`)
/// - `Param(3)` refers to the fourth entity (parameter `d`)
/// - `Param(4)` refers to the fifth entity (parameter `e`)
/// - `Field { param_idx: 0/1/2/3/4, field_idx }` accesses fields from respective entities
/// - The full solution is available for fact lookups and other operations
///
/// # Implementation
/// Searches for entities by ID in the entity slice to find their indices (O(n) per entity),
/// then builds an `EntityRef` tuple and evaluates the filter expression using `EvalContext`.
///
/// Returns `false` if any entities are not found or if the expression evaluates to a non-bool value.
///
/// Note: The O(n) entity search is acceptable because filtering is performed only on entities
/// already matched by join key, not on the full entity set.
pub fn make_penta_filter(filter_expr: Expr, class_idx: usize) -> DynPentaFilter {
    Box::new(
        move |solution: &DynamicSolution,
              a: &DynamicEntity,
              b: &DynamicEntity,
              c: &DynamicEntity,
              d: &DynamicEntity,
              e: &DynamicEntity| {
            // Find entity indices by searching the entity slice using entity IDs.
            let entities = solution
                .entities
                .get(class_idx)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);

            let a_idx = entities.iter().position(|ent| ent.id == a.id);
            let b_idx = entities.iter().position(|ent| ent.id == b.id);
            let c_idx = entities.iter().position(|ent| ent.id == c.id);
            let d_idx = entities.iter().position(|ent| ent.id == d.id);
            let e_idx = entities.iter().position(|ent| ent.id == e.id);

            // If any entities not found, return false.
            let (Some(a_idx), Some(b_idx), Some(c_idx), Some(d_idx), Some(e_idx)) =
                (a_idx, b_idx, c_idx, d_idx, e_idx)
            else {
                return false;
            };

            // Build EntityRef tuple: all five entities from the same class.
            let tuple = vec![
                EntityRef::new(class_idx, a_idx),
                EntityRef::new(class_idx, b_idx),
                EntityRef::new(class_idx, c_idx),
                EntityRef::new(class_idx, d_idx),
                EntityRef::new(class_idx, e_idx),
            ];

            let ctx = EvalContext::new(solution, &tuple);
            crate::eval::eval_expr(&filter_expr, &ctx)
                .as_bool()
                .unwrap_or(false)
        },
    )
}

/// Creates a penta-entity weight closure from a weight expression.
///
/// Returns a boxed closure that evaluates the weight expression against five entities
/// and returns a `HardSoftScore`.
///
/// # Parameters
/// - `weight_expr`: Expression to evaluate (should return numeric value)
/// - `class_idx`: Entity class index (all five entities must be from this class)
/// - `descriptor`: Problem descriptor for creating temporary solution context
/// - `is_hard`: If true, weight is applied to hard score; otherwise soft score
///
/// # Expression Context
/// - `Param(0)` refers to the first entity (parameter `a`)
/// - `Param(1)` refers to the second entity (parameter `b`)
/// - `Param(2)` refers to the third entity (parameter `c`)
/// - `Param(3)` refers to the fourth entity (parameter `d`)
/// - `Param(4)` refers to the fifth entity (parameter `e`)
/// - `Field { param_idx: 0/1/2/3/4, field_idx }` accesses fields from respective entities
/// - Arithmetic and comparison operations work across all five entities
///
/// # Implementation
/// Creates a temporary `DynamicSolution` with all five entities for proper evaluation context.
/// This enables full penta-entity expression evaluation via `EvalContext`.
///
/// Note: This approach clones entities into a temporary solution. While this violates the
/// zero-clone principle, it's necessary because the `DynPentaWeight` signature doesn't provide
/// access to the solution or entity indices. The clone happens only for matched quintuples
/// (bounded by match count, not total entity count).
pub fn make_penta_weight(
    weight_expr: Expr,
    class_idx: usize,
    descriptor: DynamicDescriptor,
    is_hard: bool,
) -> DynPentaWeight {
    Box::new(
        move |a: &DynamicEntity,
              b: &DynamicEntity,
              c: &DynamicEntity,
              d: &DynamicEntity,
              e: &DynamicEntity| {
            // Create a temporary solution with the descriptor and the five entities.
            let mut temp_solution = DynamicSolution {
                descriptor: descriptor.clone(),
                entities: vec![Vec::new(); descriptor.entity_classes.len()],
                facts: Vec::new(),
                score: None,
                id_to_location: std::collections::HashMap::new(),
            };

            // Place all five entities at indices 0, 1, 2, 3, 4 in the class entity slice.
            temp_solution.entities[class_idx] =
                vec![a.clone(), b.clone(), c.clone(), d.clone(), e.clone()];

            // Build EntityRef tuple: all five entities from the same class.
            let tuple = vec![
                EntityRef::new(class_idx, 0),
                EntityRef::new(class_idx, 1),
                EntityRef::new(class_idx, 2),
                EntityRef::new(class_idx, 3),
                EntityRef::new(class_idx, 4),
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
