//! Closure builder functions for cross-join constraints (joining two different entity classes).

use solverforge_core::score::HardSoftScore;

use super::types::{
    DynCrossExtractorA, DynCrossExtractorB, DynCrossFilter, DynCrossKeyA, DynCrossKeyB,
    DynCrossWeight,
};
use crate::descriptor::DynamicDescriptor;
use crate::eval::{eval_expr, EntityRef, EvalContext};
use crate::expr::Expr;
use crate::solution::{DynamicEntity, DynamicSolution};

/// Creates an extractor for the first entity class in a cross-join.
///
/// Returns a closure that extracts the entity slice for class A from the solution.
/// This is used by `IncrementalCrossBiConstraint` to access entities of the first class.
///
/// # Parameters
/// - `class_idx_a`: Index of the first entity class to extract
///
/// # Returns
/// A boxed closure that takes a `DynamicSolution` and returns a slice of entities for class A.
pub fn make_cross_extractor_a(class_idx_a: usize) -> DynCrossExtractorA {
    Box::new(move |solution: &DynamicSolution| {
        solution
            .entities
            .get(class_idx_a)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    })
}

/// Creates an extractor for the second entity class in a cross-join.
///
/// Returns a closure that extracts the entity slice for class B from the solution.
/// This is used by `IncrementalCrossBiConstraint` to access entities of the second class.
///
/// # Parameters
/// - `class_idx_b`: Index of the second entity class to extract
///
/// # Returns
/// A boxed closure that takes a `DynamicSolution` and returns a slice of entities for class B.
pub fn make_cross_extractor_b(class_idx_b: usize) -> DynCrossExtractorB {
    Box::new(move |solution: &DynamicSolution| {
        solution
            .entities
            .get(class_idx_b)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    })
}

/// Creates a cross-join key extractor for the first entity class (A).
///
/// This function creates a closure that evaluates a join key expression against
/// an entity from class A to extract a join key value used for cross-join indexing.
///
/// # Parameters
/// - `key_expr`: Expression to evaluate for extracting the join key
/// - `descriptor`: The solution descriptor (for expression evaluation context)
///
/// # Returns
/// A boxed closure that takes an entity from class A and returns its join key.
///
/// # Expression Context
/// - `Param(0)` refers to the entity itself (returns entity ID)
/// - `Field { param_idx: 0, field_idx }` accesses fields from the entity
///
/// # Design Constraint
/// Join key expressions should only reference entity fields, not facts or solution state.
/// The minimal solution context ensures this by having empty entities and facts vectors.
pub fn make_cross_key_a(key_expr: Expr, descriptor: DynamicDescriptor) -> DynCrossKeyA {
    // Create minimal solution with only descriptor (no entities/facts).
    // This is intentional - join keys should be stable entity attributes.
    let minimal_solution = DynamicSolution {
        descriptor,
        entities: Vec::new(),
        facts: Vec::new(),
        score: None,
        id_to_location: std::collections::HashMap::new(),
    };

    Box::new(move |entity: &DynamicEntity| {
        crate::eval::eval_entity_expr(&key_expr, &minimal_solution, entity)
    })
}

/// Creates a cross-join key extractor for the second entity class (B).
///
/// This function creates a closure that evaluates a join key expression against
/// an entity from class B to extract a join key value used for cross-join indexing.
///
/// # Parameters
/// - `key_expr`: Expression to evaluate for extracting the join key
/// - `descriptor`: The solution descriptor (for expression evaluation context)
///
/// # Returns
/// A boxed closure that takes an entity from class B and returns its join key.
///
/// # Expression Context
/// - `Param(0)` refers to the entity itself (returns entity ID)
/// - `Field { param_idx: 0, field_idx }` accesses fields from the entity
///
/// # Design Constraint
/// Join key expressions should only reference entity fields, not facts or solution state.
/// The minimal solution context ensures this by having empty entities and facts vectors.
pub fn make_cross_key_b(key_expr: Expr, descriptor: DynamicDescriptor) -> DynCrossKeyB {
    // Create minimal solution with only descriptor (no entities/facts).
    // This is intentional - join keys should be stable entity attributes.
    let minimal_solution = DynamicSolution {
        descriptor,
        entities: Vec::new(),
        facts: Vec::new(),
        score: None,
        id_to_location: std::collections::HashMap::new(),
    };

    Box::new(move |entity: &DynamicEntity| {
        crate::eval::eval_entity_expr(&key_expr, &minimal_solution, entity)
    })
}

/// Creates a cross-join filter function that evaluates an expression against entities from two different classes.
///
/// This function creates a `DynCrossFilter` closure that evaluates a filter expression against a pair
/// of entities from different entity classes (cross-join).
///
/// # Arguments
/// * `filter_expr` - The expression to evaluate against the entity pair (returns bool)
/// * `class_idx_a` - The entity class index for the first entity (class A)
/// * `class_idx_b` - The entity class index for the second entity (class B)
///
/// # Returns
/// A boxed closure that takes a `DynamicSolution` reference and two `DynamicEntity` references
/// (one from class A, one from class B), evaluates the filter expression in a cross-join context,
/// and returns whether the pair matches.
///
/// # Expression Context
/// - `Param(0)` refers to the first entity from class A (parameter `a`)
/// - `Param(1)` refers to the second entity from class B (parameter `b`)
/// - `Field { param_idx: 0, field_idx }` accesses fields from the class A entity
/// - `Field { param_idx: 1, field_idx }` accesses fields from the class B entity
/// - The full solution is available for fact lookups and other operations
///
/// The expression should return a boolean value. Non-boolean results are treated as `false`.
///
/// # Implementation Note
/// Entity index lookup uses the `id_to_location` HashMap for O(1) performance.
pub fn make_cross_filter(
    filter_expr: Expr,
    class_idx_a: usize,
    class_idx_b: usize,
) -> DynCrossFilter {
    Box::new(
        move |solution: &DynamicSolution, a: &DynamicEntity, b: &DynamicEntity| {
            // Look up entity indices using O(1) HashMap lookup.
            let a_loc = solution.get_entity_location(a.id);
            let b_loc = solution.get_entity_location(b.id);

            let (Some((a_class, a_idx)), Some((b_class, b_idx))) = (a_loc, b_loc) else {
                // Entities not found in solution - shouldn't happen, but return false defensively
                return false;
            };

            // Verify entities are from the expected classes
            if a_class != class_idx_a || b_class != class_idx_b {
                return false;
            }

            // Build entity tuple for evaluation context (different class indices)
            let tuple = vec![
                EntityRef::new(class_idx_a, a_idx),
                EntityRef::new(class_idx_b, b_idx),
            ];

            // Evaluate expression in cross-join context
            let ctx = EvalContext::new(solution, &tuple);
            let result = eval_expr(&filter_expr, &ctx);

            // Convert result to bool (default to false if not a bool)
            result.as_bool().unwrap_or(false)
        },
    )
}

/// Creates a cross-join weight function that evaluates an expression against entities from two different classes.
///
/// This function creates a `DynCrossWeight` closure that takes the full solution and two entity
/// indices (one for class A, one for class B), and evaluates a weight expression to produce a score.
///
/// # Arguments
/// * `weight_expr` - The expression to evaluate against the entity pair (returns numeric weight)
/// * `class_idx_a` - The entity class index for the first entity (class A)
/// * `class_idx_b` - The entity class index for the second entity (class B)
/// * `is_hard` - Whether this is a hard constraint (weight applied to hard score component)
///
/// # Returns
/// A boxed closure that takes a `DynamicSolution` reference and two entity indices (a_idx, b_idx),
/// evaluates the weight expression in a cross-join context, and returns a `HardSoftScore`.
///
/// # Expression Evaluation
/// - `Param(0)` refers to the first entity from class A (parameter `a`)
/// - `Param(1)` refers to the second entity from class B (parameter `b`)
/// - `Field { param_idx: 0, field_idx }` accesses fields from the class A entity
/// - `Field { param_idx: 1, field_idx }` accesses fields from the class B entity
///
/// The expression should return a numeric value (i64). Non-numeric results default to 0.
///
/// # Performance
/// This implementation uses the actual solution reference and entity indices directly,
/// avoiding temporary solution construction and entity cloning.
pub fn make_cross_weight(
    weight_expr: Expr,
    class_idx_a: usize,
    class_idx_b: usize,
    is_hard: bool,
) -> DynCrossWeight {
    Box::new(
        move |solution: &DynamicSolution, a_idx: usize, b_idx: usize| {
            // Build entity tuple for evaluation context using the actual indices
            let tuple = vec![
                EntityRef::new(class_idx_a, a_idx),
                EntityRef::new(class_idx_b, b_idx),
            ];

            // Evaluate expression in cross-join context using the actual solution
            let ctx = EvalContext::new(solution, &tuple);
            let result = eval_expr(&weight_expr, &ctx);

            // Convert to numeric value (default to 0 if not numeric)
            let weight_num = result.as_i64().unwrap_or(0) as i64;

            // Apply to hard or soft component
            if is_hard {
                HardSoftScore::of_hard(weight_num)
            } else {
                HardSoftScore::of_soft(weight_num)
            }
        },
    )
}
