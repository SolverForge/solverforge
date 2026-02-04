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
/// The filter searches the entity slice to find entity indices, which is O(n) per call.
/// This is acceptable because filtering is done on already-matched entities (by join key),
/// not on the full entity set.
pub fn make_bi_filter(filter_expr: Expr, class_idx: usize) -> DynBiFilter {
    Box::new(
        move |solution: &DynamicSolution, a: &DynamicEntity, b: &DynamicEntity| {
            // Find entity indices by searching the entity slice.
            // For self-join constraints, both entities are from class_idx.
            // We search by entity ID which is unique.
            let entities = solution
                .entities
                .get(class_idx)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);

            let a_idx = entities.iter().position(|e| e.id == a.id);
            let b_idx = entities.iter().position(|e| e.id == b.id);

            let (Some(a_idx), Some(b_idx)) = (a_idx, b_idx) else {
                // Entities not found in solution - shouldn't happen, but return false defensively
                return false;
            };

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
/// * `descriptor` - The schema descriptor for building evaluation context
/// * `is_hard` - Whether this is a hard constraint (weight applied to hard score component)
///
/// # Returns
/// A boxed closure that takes two `DynamicEntity` references, evaluates the weight expression
/// in a bi-entity context, and returns a `HardSoftScore`.
///
/// # Expression Evaluation
/// The weight expression is evaluated in a tuple context where:
/// - `Param(0)` refers to the first entity (parameter `a`)
/// - `Param(1)` refers to the second entity (parameter `b`)
/// - `Field { param_idx: 0, field_idx }` accesses fields from the first entity
/// - `Field { param_idx: 1, field_idx }` accesses fields from the second entity
///
/// The expression should return a numeric value (i64). Non-numeric results default to 0.
///
/// **Design constraint**: Weight expressions should only reference entity fields and perform
/// arithmetic/comparisons. References to facts or other solution state will NOT work correctly
/// because the evaluation uses a temporary solution context with only the two entities.
///
/// # Weight Application
/// The resulting numeric value is applied to either the hard or soft score component based on
/// the `is_hard` parameter. The constraint's impact type (penalty vs reward) is NOT applied
/// here - that's handled by the monomorphized constraint wrapper's `compute_score` method.
pub fn make_bi_weight(
    weight_expr: Expr,
    class_idx: usize,
    descriptor: DynamicDescriptor,
    is_hard: bool,
) -> DynBiWeight {
    Box::new(move |a: &DynamicEntity, b: &DynamicEntity| {
        // Create a temporary solution context with just these two entities
        // This allows us to use EvalContext with proper entity indices
        let mut temp_solution = DynamicSolution {
            descriptor: descriptor.clone(),
            entities: Vec::new(),
            facts: Vec::new(),
            score: None,
        };

        // Ensure the entities vec is large enough
        while temp_solution.entities.len() <= class_idx {
            temp_solution.entities.push(Vec::new());
        }

        // Add the two entities at indices 0 and 1
        temp_solution.entities[class_idx] = vec![a.clone(), b.clone()];

        // Build entity tuple for evaluation context (indices 0 and 1)
        let tuple = vec![EntityRef::new(class_idx, 0), EntityRef::new(class_idx, 1)];

        // Evaluate expression in bi-entity context
        let ctx = EvalContext::new(&temp_solution, &tuple);
        let result = eval_expr(&weight_expr, &ctx);

        // Convert to numeric value (default to 0 if not numeric)
        let weight_num = result.as_i64().unwrap_or(0) as f64;

        // Apply to hard or soft component
        if is_hard {
            HardSoftScore::of_hard(weight_num as i64)
        } else {
            HardSoftScore::of_soft(weight_num as i64)
        }
    })
}
