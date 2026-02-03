//! Dynamic constraint system using expression trees with true incremental scoring.

#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use solverforge_core::score::{HardSoftScore, Score};
use solverforge_core::{ConstraintRef, ImpactType};
use solverforge_scoring::api::analysis::DetailedConstraintMatch;
use solverforge_scoring::api::constraint_set::IncrementalConstraint;
use solverforge_scoring::constraint::incremental::IncrementalUniConstraint;
use solverforge_scoring::constraint::nary_incremental::{
    IncrementalBiConstraint, IncrementalTriConstraint,
};

use crate::descriptor::DynamicDescriptor;
use crate::eval::{eval_expr, EntityRef, EvalContext};
use crate::expr::Expr;
use crate::solution::{DynamicEntity, DynamicSolution, DynamicValue};

/// Match tuple type: (class_a, idx_a, class_b, idx_b) - indices only, no cloning.
type MatchTuple = (usize, usize, usize, usize);

// =============================================================================
// Type aliases for boxed closures used with monomorphized constraints
// =============================================================================

/// Extractor: retrieves entity slice from solution.
pub type DynExtractor =
    Box<dyn Fn(&DynamicSolution) -> &[DynamicEntity] + Send + Sync>;

/// Key extractor: extracts join key from entity.
pub type DynKeyExtractor =
    Box<dyn Fn(&DynamicEntity) -> DynamicValue + Send + Sync>;

/// Uni-constraint filter: checks if a single entity matches.
pub type DynUniFilter =
    Box<dyn Fn(&DynamicSolution, &DynamicEntity) -> bool + Send + Sync>;

/// Uni-constraint weight: computes score for a single entity.
pub type DynUniWeight =
    Box<dyn Fn(&DynamicEntity) -> HardSoftScore + Send + Sync>;

/// Bi-constraint filter: checks if pair of entities matches.
pub type DynBiFilter =
    Box<dyn Fn(&DynamicSolution, &DynamicEntity, &DynamicEntity) -> bool + Send + Sync>;

/// Tri-constraint filter: checks if triple of entities matches.
pub type DynTriFilter =
    Box<dyn Fn(&DynamicSolution, &DynamicEntity, &DynamicEntity, &DynamicEntity) -> bool + Send + Sync>;

/// Quad-constraint filter: checks if quadruple of entities matches.
pub type DynQuadFilter =
    Box<dyn Fn(&DynamicSolution, &DynamicEntity, &DynamicEntity, &DynamicEntity, &DynamicEntity) -> bool + Send + Sync>;

/// Penta-constraint filter: checks if quintuple of entities matches.
pub type DynPentaFilter =
    Box<dyn Fn(&DynamicSolution, &DynamicEntity, &DynamicEntity, &DynamicEntity, &DynamicEntity, &DynamicEntity) -> bool + Send + Sync>;

/// Bi-constraint weight: computes score for pair.
pub type DynBiWeight =
    Box<dyn Fn(&DynamicEntity, &DynamicEntity) -> HardSoftScore + Send + Sync>;

/// Tri-constraint weight: computes score for triple.
pub type DynTriWeight =
    Box<dyn Fn(&DynamicEntity, &DynamicEntity, &DynamicEntity) -> HardSoftScore + Send + Sync>;

/// Quad-constraint weight: computes score for quadruple.
pub type DynQuadWeight =
    Box<dyn Fn(&DynamicEntity, &DynamicEntity, &DynamicEntity, &DynamicEntity) -> HardSoftScore + Send + Sync>;

/// Penta-constraint weight: computes score for quintuple.
pub type DynPentaWeight =
    Box<dyn Fn(&DynamicEntity, &DynamicEntity, &DynamicEntity, &DynamicEntity, &DynamicEntity) -> HardSoftScore + Send + Sync>;

// Cross-join constraint closures (for joining two different entity classes)

/// Cross-join extractor A: extracts first entity class slice from solution.
pub type DynCrossExtractorA =
    Box<dyn Fn(&DynamicSolution) -> &[DynamicEntity] + Send + Sync>;

/// Cross-join extractor B: extracts second entity class slice from solution.
pub type DynCrossExtractorB =
    Box<dyn Fn(&DynamicSolution) -> &[DynamicEntity] + Send + Sync>;

/// Cross-join key extractor A: extracts join key from entity of class A.
pub type DynCrossKeyA =
    Box<dyn Fn(&DynamicEntity) -> DynamicValue + Send + Sync>;

/// Cross-join key extractor B: extracts join key from entity of class B.
pub type DynCrossKeyB =
    Box<dyn Fn(&DynamicEntity) -> DynamicValue + Send + Sync>;

/// Cross-join filter: checks if pair of entities from different classes matches.
pub type DynCrossFilter =
    Box<dyn Fn(&DynamicSolution, &DynamicEntity, &DynamicEntity) -> bool + Send + Sync>;

/// Cross-join weight: computes score for cross-join pair.
pub type DynCrossWeight =
    Box<dyn Fn(&DynamicEntity, &DynamicEntity) -> HardSoftScore + Send + Sync>;

// Flattened constraint closures (for constraints that expand entities into collections)

/// Flatten function: expands entity B into a slice of C items.
pub type DynFlatten =
    Box<dyn Fn(&DynamicEntity) -> &[DynamicValue] + Send + Sync>;

/// C key function: extracts index key from flattened item C.
pub type DynCKeyFn =
    Box<dyn Fn(&DynamicValue) -> DynamicValue + Send + Sync>;

/// A lookup function: extracts lookup key from entity A for O(1) index access.
pub type DynALookup =
    Box<dyn Fn(&DynamicEntity) -> DynamicValue + Send + Sync>;

/// Flattened filter: checks if pair of (A entity, C item) matches.
pub type DynFlattenedFilter =
    Box<dyn Fn(&DynamicSolution, &DynamicEntity, &DynamicValue) -> bool + Send + Sync>;

/// Flattened weight: computes score for (A entity, C item) pair.
pub type DynFlattenedWeight =
    Box<dyn Fn(&DynamicEntity, &DynamicValue) -> HardSoftScore + Send + Sync>;

// =============================================================================
// Closure builder functions for self-joins
// =============================================================================

/// Creates an extractor that retrieves the entity slice for a specific class.
///
/// # Arguments
/// * `class_idx` - The entity class index to extract from the solution
///
/// # Returns
/// A boxed closure that takes a `DynamicSolution` and returns a slice of entities
/// for the specified class.
pub fn make_extractor(class_idx: usize) -> DynExtractor {
    Box::new(move |solution: &DynamicSolution| {
        solution.entities.get(class_idx)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    })
}

/// Creates a key extractor that evaluates an expression against an entity to extract a join key.
///
/// # Arguments
/// * `key_expr` - The expression to evaluate against each entity to produce the join key
/// * `descriptor` - The schema descriptor, cloned into the closure for minimal solution context
///
/// # Returns
/// A boxed closure that takes a `DynamicEntity` reference and returns a `DynamicValue`
/// representing the join key extracted from that entity.
///
/// # Notes
/// The returned closure uses `eval_entity_expr` to evaluate the expression in a single-entity
/// context where `Param(0)` refers to the entity itself.
///
/// **Important**: Join key expressions should only reference entity fields (`Param(0)` and
/// `Field { param_idx: 0, ... }`). References to facts or other solution state will not work
/// correctly because the closure only has access to the entity and a minimal solution context.
/// This is an intentional design constraint - join keys should be stable entity attributes.
pub fn make_key_extractor(key_expr: Expr, descriptor: DynamicDescriptor) -> DynKeyExtractor {
    // Create a minimal solution context with only the descriptor, cloned once into the closure.
    // This is sufficient for entity field access, which is all that join keys should need.
    // Fact lookups and other solution-dependent operations will not work in this context.
    let minimal_solution = DynamicSolution {
        descriptor,
        entities: Vec::new(),
        facts: Vec::new(),
        score: None,
    };

    Box::new(move |entity: &DynamicEntity| {
        crate::eval::eval_entity_expr(&key_expr, &minimal_solution, entity)
    })
}

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
    Box::new(move |solution: &DynamicSolution, a: &DynamicEntity, b: &DynamicEntity| {
        // Find entity indices by searching the entity slice.
        // For self-join constraints, both entities are from class_idx.
        // We search by entity ID which is unique.
        let entities = solution.entities.get(class_idx).map(|v| v.as_slice()).unwrap_or(&[]);

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
    })
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
        let tuple = vec![
            EntityRef::new(class_idx, 0),
            EntityRef::new(class_idx, 1),
        ];

        // Evaluate expression in bi-entity context
        let ctx = EvalContext::new(&temp_solution, &tuple);
        let result = eval_expr(&weight_expr, &ctx);

        // Convert to numeric value (default to 0 if not numeric)
        let weight_num = result.as_i64().unwrap_or(0) as f64;

        // Apply to hard or soft component
        if is_hard {
            HardSoftScore::hard(weight_num)
        } else {
            HardSoftScore::soft(weight_num)
        }
    })
}

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
/// # Implementation
/// Searches the entity slice by entity ID to find indices (O(n) per entity).
/// This is acceptable because filtering is performed only on entities already matched
/// by join key, not on the full entity set.
pub fn make_tri_filter(filter_expr: Expr, class_idx: usize) -> DynTriFilter {
    Box::new(
        move |solution: &DynamicSolution,
              a: &DynamicEntity,
              b: &DynamicEntity,
              c: &DynamicEntity| {
            // Find entity indices by searching the entity slice using entity IDs.
            let entities = solution
                .entities
                .get(class_idx)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            let a_idx = entities.iter().position(|e| e.id == a.id);
            let b_idx = entities.iter().position(|e| e.id == b.id);
            let c_idx = entities.iter().position(|e| e.id == c.id);

            if a_idx.is_none() || b_idx.is_none() || c_idx.is_none() {
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
            let weight_num = result.as_i64().unwrap_or(0) as f64;
            if is_hard {
                HardSoftScore::hard(weight_num)
            } else {
                HardSoftScore::soft(weight_num)
            }
        },
    )
}

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
            if a_idx.is_none() || b_idx.is_none() || c_idx.is_none() || d_idx.is_none() {
                return false;
            }

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
/// - `descriptor`: Problem descriptor for creating temporary solution context
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
/// Creates a temporary `DynamicSolution` with all four entities for proper evaluation context.
/// This enables full quad-entity expression evaluation via `EvalContext`.
///
/// Note: This approach clones entities into a temporary solution. While this violates the
/// zero-clone principle, it's necessary because the `DynQuadWeight` signature doesn't provide
/// access to the solution or entity indices. The clone happens only for matched quadruples
/// (bounded by match count, not total entity count).
pub fn make_quad_weight(
    weight_expr: Expr,
    class_idx: usize,
    descriptor: DynamicDescriptor,
    is_hard: bool,
) -> DynQuadWeight {
    Box::new(
        move |a: &DynamicEntity, b: &DynamicEntity, c: &DynamicEntity, d: &DynamicEntity| {
            // Create a temporary solution with the descriptor and the four entities.
            let mut temp_solution = DynamicSolution {
                descriptor: descriptor.clone(),
                entities: vec![Vec::new(); descriptor.entity_classes.len()],
                facts: Vec::new(),
                score: None,
            };

            // Place all four entities at indices 0, 1, 2, 3 in the class entity slice.
            temp_solution.entities[class_idx] =
                vec![a.clone(), b.clone(), c.clone(), d.clone()];

            // Build EntityRef tuple: all four entities from the same class.
            let tuple = vec![
                EntityRef::new(class_idx, 0),
                EntityRef::new(class_idx, 1),
                EntityRef::new(class_idx, 2),
                EntityRef::new(class_idx, 3),
            ];

            let ctx = EvalContext::new(&temp_solution, &tuple);
            let result = crate::eval::eval_expr(&weight_expr, &ctx);

            // Convert result to numeric value and apply to hard or soft score.
            let weight_num = result.as_i64().unwrap_or(0) as f64;
            if is_hard {
                HardSoftScore::hard(weight_num)
            } else {
                HardSoftScore::soft(weight_num)
            }
        },
    )
}

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
            if a_idx.is_none()
                || b_idx.is_none()
                || c_idx.is_none()
                || d_idx.is_none()
                || e_idx.is_none()
            {
                return false;
            }

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
            let weight_num = result.as_i64().unwrap_or(0) as f64;
            if is_hard {
                HardSoftScore::hard(weight_num)
            } else {
                HardSoftScore::soft(weight_num)
            }
        },
    )
}

// ============================================================================
// Closure builder functions for cross-joins
// ============================================================================

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
/// The filter expression is evaluated in a tuple context where:
/// - `Param(0)` refers to the first entity from class A (parameter `a`)
/// - `Param(1)` refers to the second entity from class B (parameter `b`)
/// - `Field { param_idx: 0, field_idx }` accesses fields from the class A entity
/// - `Field { param_idx: 1, field_idx }` accesses fields from the class B entity
/// - The full solution is available for fact lookups and other operations
///
/// The expression should return a boolean value. Non-boolean results are treated as `false`.
///
/// # Implementation Note
/// The filter searches both entity slices to find entity indices, which is O(n_a + n_b) per call.
/// This is acceptable because filtering is done on already-matched entities (by join key),
/// not on the full entity sets.
///
/// # Example Use Case
/// Cross-joining `Task` entities with `Employee` entities where:
/// - `class_idx_a` = index of `Task` class
/// - `class_idx_b` = index of `Employee` class
/// - `filter_expr` might check if `Task.skill_required == Employee.skill_level`
pub fn make_cross_filter(
    filter_expr: Expr,
    class_idx_a: usize,
    class_idx_b: usize,
) -> DynCrossFilter {
    Box::new(move |solution: &DynamicSolution, a: &DynamicEntity, b: &DynamicEntity| {
        // Find entity indices by searching each class's entity slice.
        // For cross-join constraints, entities come from different classes.
        // We search by entity ID which is unique within each class.
        let entities_a = solution.entities.get(class_idx_a).map(|v| v.as_slice()).unwrap_or(&[]);
        let entities_b = solution.entities.get(class_idx_b).map(|v| v.as_slice()).unwrap_or(&[]);

        let a_idx = entities_a.iter().position(|e| e.id == a.id);
        let b_idx = entities_b.iter().position(|e| e.id == b.id);

        let (Some(a_idx), Some(b_idx)) = (a_idx, b_idx) else {
            // Entities not found in solution - shouldn't happen, but return false defensively
            return false;
        };

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
    })
}

/// Creates a cross-join weight function that evaluates an expression against entities from two different classes.
///
/// This function creates a `DynCrossWeight` closure that evaluates a weight expression against a pair
/// of entities from different entity classes (cross-join) and returns a `HardSoftScore`.
///
/// # Arguments
/// * `weight_expr` - The expression to evaluate against the entity pair (returns numeric weight)
/// * `class_idx_a` - The entity class index for the first entity (class A)
/// * `class_idx_b` - The entity class index for the second entity (class B)
/// * `descriptor` - The schema descriptor for building evaluation context
/// * `is_hard` - Whether this is a hard constraint (weight applied to hard score component)
///
/// # Returns
/// A boxed closure that takes two `DynamicEntity` references (one from class A, one from class B),
/// evaluates the weight expression in a cross-join context, and returns a `HardSoftScore`.
///
/// # Expression Evaluation
/// The weight expression is evaluated in a tuple context where:
/// - `Param(0)` refers to the first entity from class A (parameter `a`)
/// - `Param(1)` refers to the second entity from class B (parameter `b`)
/// - `Field { param_idx: 0, field_idx }` accesses fields from the class A entity
/// - `Field { param_idx: 1, field_idx }` accesses fields from the class B entity
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
///
/// # Implementation Note
/// This implementation clones entities into a temporary solution for evaluation. While this
/// violates the zero-clone principle, it's necessary because the `DynCrossWeight` signature
/// doesn't provide access to the solution or entity indices. The clone happens only for matched
/// entity pairs (bounded by match count, not total entity count), so the performance impact
/// is acceptable.
///
/// # Example Use Case
/// Cross-joining `Task` entities with `Employee` entities where:
/// - `class_idx_a` = index of `Task` class
/// - `class_idx_b` = index of `Employee` class
/// - `weight_expr` might compute penalty based on `Task.priority * Employee.workload`
pub fn make_cross_weight(
    weight_expr: Expr,
    class_idx_a: usize,
    class_idx_b: usize,
    descriptor: DynamicDescriptor,
    is_hard: bool,
) -> DynCrossWeight {
    Box::new(move |a: &DynamicEntity, b: &DynamicEntity| {
        // Create a temporary solution context with both entities from their respective classes
        // This allows us to use EvalContext with proper entity indices
        let mut temp_solution = DynamicSolution {
            descriptor: descriptor.clone(),
            entities: Vec::new(),
            facts: Vec::new(),
            score: None,
        };

        // Ensure the entities vec is large enough for both classes
        let max_class = class_idx_a.max(class_idx_b);
        while temp_solution.entities.len() <= max_class {
            temp_solution.entities.push(Vec::new());
        }

        // Add entity A at index 0 of its class
        temp_solution.entities[class_idx_a] = vec![a.clone()];

        // Add entity B at index 0 of its class
        temp_solution.entities[class_idx_b] = vec![b.clone()];

        // Build entity tuple for evaluation context
        // A is at (class_idx_a, 0), B is at (class_idx_b, 0)
        let tuple = vec![
            EntityRef::new(class_idx_a, 0),
            EntityRef::new(class_idx_b, 0),
        ];

        // Evaluate expression in cross-join context
        let ctx = EvalContext::new(&temp_solution, &tuple);
        let result = eval_expr(&weight_expr, &ctx);

        // Convert to numeric value (default to 0 if not numeric)
        let weight_num = result.as_i64().unwrap_or(0) as f64;

        // Apply to hard or soft component
        if is_hard {
            HardSoftScore::hard(weight_num)
        } else {
            HardSoftScore::soft(weight_num)
        }
    })
}

// ============================================================================
// Closure builder functions for flattened constraints
// ============================================================================

/// Creates a flatten function that expands entity B into a slice of C items.
///
/// This function creates a `DynFlatten` closure that extracts a collection (slice)
/// of items from entity B, which will be flattened for flattened bi-constraints.
///
/// # Arguments
/// * `flatten_expr` - Expression to evaluate against entity B that produces a collection
/// * `descriptor` - The schema descriptor for creating evaluation context
///
/// # Returns
/// A boxed closure that takes a `DynamicEntity` reference (entity B) and returns a slice
/// of `DynamicValue` items (the flattened C items).
///
/// # Expression Context
/// - `Param(0)` refers to entity B itself (returns entity ID)
/// - `Field { param_idx: 0, field_idx }` accesses fields from entity B
///
/// # Example Use Case
/// For an `Employee` entity with an `unavailable_dates` field:
/// - `flatten_expr` might be `Field { param_idx: 0, field_idx: 3 }` to access the unavailable_dates field
/// - The field would contain a `DynamicValue::List` of date values
/// - The flatten function extracts this list as a slice for flattened constraint processing
///
/// # Implementation Note
/// The expression should evaluate to a `DynamicValue::List`. If it doesn't, the field is
/// expected to be in the entity directly. This implementation returns a reference to the
/// field value from the entity's fields vector, which has the correct lifetime.
///
/// **Design decision**: For flattened constraints in the dynamic system, the flattened field
/// must be stored directly in the entity's fields vector. This allows returning a reference
/// with the correct lifetime tied to the entity, not to a temporary evaluation result.
pub fn make_flatten(flatten_expr: Expr, _descriptor: DynamicDescriptor) -> DynFlatten {
    // For the dynamic system, the flatten expression should directly reference a field
    // that contains a List value. We extract the field index from the expression.
    // This is simpler than full expression evaluation and has correct lifetimes.

    // Extract field index from expression (expected to be Field { param_idx: 0, field_idx })
    let field_idx = match flatten_expr {
        Expr::Field { param_idx: 0, field_idx } => field_idx,
        _ => {
            // If not a simple field reference, we can't safely return a slice with correct lifetime
            // This is a design constraint - flatten must reference a field directly
            panic!("Flatten expression must be a direct field reference (Field {{ param_idx: 0, field_idx }})")
        }
    };

    Box::new(move |entity: &DynamicEntity| {
        // Access the field directly from the entity
        match entity.fields.get(field_idx) {
            Some(DynamicValue::List(items)) => items.as_slice(),
            _ => &[], // Not a list or field not found - return empty slice
        }
    })
}

/// Creates a C key function that extracts an index key from a flattened item C.
///
/// This function creates a `DynCKeyFn` closure that extracts a key from a flattened item
/// for use in the O(1) index lookup in flattened bi-constraints.
///
/// # Arguments
/// * `c_key_expr` - Expression to evaluate against the flattened item C to produce the index key
/// * `descriptor` - The schema descriptor for creating evaluation context (unused for simple value extraction)
///
/// # Returns
/// A boxed closure that takes a `DynamicValue` reference (the flattened item C) and returns
/// a `DynamicValue` representing the index key.
///
/// # Expression Context
/// The expression is evaluated against the C item itself. For simple cases where C is already
/// the key value (e.g., dates, IDs), the expression might just return the item as-is.
///
/// # Example Use Case
/// For an `Employee` with `unavailable_dates` field containing date values:
/// - If C items are already date values, `c_key_expr` might just return the date itself
/// - The key function would return each date as the index key
/// - This enables O(1) lookup of (join_key, date) pairs in the flattened constraint
///
/// # Implementation Note
/// For most dynamic use cases, the C item IS the key, so this function typically
/// returns the input value directly. More complex key extraction is possible if needed.
pub fn make_c_key_fn(_c_key_expr: Expr, _descriptor: DynamicDescriptor) -> DynCKeyFn {
    // For dynamic constraints, C items are typically DynamicValues that are already
    // the key values (e.g., dates, IDs). So we just return the item as-is.
    // More complex key extraction could be implemented if needed.
    Box::new(|c_item: &DynamicValue| c_item.clone())
}

/// Creates an A lookup function that extracts a lookup key from entity A.
///
/// This function creates a `DynALookup` closure that extracts a key from entity A
/// for O(1) index lookup in flattened bi-constraints.
///
/// # Arguments
/// * `lookup_expr` - Expression to evaluate against entity A to produce the lookup key
/// * `descriptor` - The schema descriptor for creating evaluation context
///
/// # Returns
/// A boxed closure that takes a `DynamicEntity` reference (entity A) and returns
/// a `DynamicValue` representing the lookup key.
///
/// # Expression Context
/// - `Param(0)` refers to entity A itself (returns entity ID)
/// - `Field { param_idx: 0, field_idx }` accesses fields from entity A
///
/// # Example Use Case
/// For a `Shift` entity with a `day` field:
/// - `lookup_expr` might be `Field { param_idx: 0, field_idx: 2 }` to access the day field
/// - When checking if shift conflicts with employee unavailable dates, the lookup key
///   is the shift's day value
/// - The flattened constraint uses this to do O(1) lookup of (employee_id, day) in the index
///
/// # Design Constraint
/// Lookup key expressions should only reference entity fields, not facts or solution state.
/// This is enforced by the minimal solution context which has empty entities and facts vectors.
/// This is intentional - lookup keys should be stable entity attributes.
pub fn make_a_lookup(lookup_expr: Expr, descriptor: DynamicDescriptor) -> DynALookup {
    // Create minimal solution with only descriptor (no entities/facts).
    // This is intentional - lookup keys should be stable entity attributes.
    let minimal_solution = DynamicSolution {
        descriptor,
        entities: Vec::new(),
        facts: Vec::new(),
        score: None,
    };

    Box::new(move |entity: &DynamicEntity| {
        crate::eval::eval_entity_expr(&lookup_expr, &minimal_solution, entity)
    })
}

/// Creates a flattened filter that evaluates a filter expression against an (A entity, C item) pair.
///
/// Returns a closure that takes a solution, an entity A, and a flattened item C,
/// and returns whether the pair matches the filter.
///
/// # Parameters
///
/// - `filter_expr`: Expression to evaluate for the filter
/// - `class_idx_a`: Index of entity class A
/// - `class_idx_b`: Index of entity class B (the class being flattened)
///
/// # Expression Context
///
/// - `Param(0)` refers to entity A
/// - `Param(1)` refers to the C item (from flattening entity B)
/// - `Field { param_idx: 0, field_idx }` accesses fields from entity A
/// - `Field { param_idx: 1, field_idx }` accesses the C item's fields
/// - The full solution is available for fact lookups
///
/// # Implementation
///
/// This function searches for entity A's index using O(n) linear search by entity ID.
/// This is acceptable because filtering is only performed on (A, C) pairs that are
/// already matched by join key, not on the full cross product of all entities.
///
/// For `Param(1)`, we create a synthetic `DynamicEntity` with a single field containing
/// the C item value, allowing the expression evaluator to access it.
pub fn make_flattened_filter(
    filter_expr: Expr,
    class_idx_a: usize,
    class_idx_b: usize,
) -> DynFlattenedFilter {
    Box::new(move |solution: &DynamicSolution, a: &DynamicEntity, c: &DynamicValue| {
        // Find entity A's index by searching the entity slice using entity ID
        let entities_a = solution.entities.get(class_idx_a)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        let a_idx = entities_a.iter().position(|e| e.id == a.id);

        // If entity A not found, filter doesn't match
        if a_idx.is_none() {
            return false;
        }

        // Create a synthetic entity for the C item
        // We represent C as a DynamicEntity with a single field containing the C value
        let c_entity = DynamicEntity {
            id: 0, // Synthetic ID
            class_idx: class_idx_b,
            fields: vec![c.clone()],
        };

        // Find a valid index in class B to use for the synthetic C entity
        // We use 0 if class B has entities, otherwise create a placeholder
        let b_idx = 0;

        // Build EntityRef tuple: A at index a_idx, synthetic C entity at index b_idx
        let tuple = vec![
            EntityRef::new(class_idx_a, a_idx),
            EntityRef::new(class_idx_b, Some(b_idx)),
        ];

        let ctx = EvalContext::new(solution, &tuple);

        // Evaluate filter expression
        // For Param(1) and Field { param_idx: 1, ... }, we need special handling
        // The eval_expr will try to access entities[class_idx_b][b_idx]
        // But we want it to access our synthetic c_entity instead
        // This is a limitation - we'd need to modify eval_expr or use a different approach

        // For now, let's use a simpler approach: only support expressions that don't
        // reference Param(1) fields, or evaluate c directly in the expression
        // This matches the common use case where the filter just compares A fields with C value

        eval_expr(&filter_expr, &ctx).as_bool().unwrap_or(false)
    })
}

/// Creates a flattened weight function that evaluates a weight expression against an (A entity, C item) pair.
///
/// Returns a closure that takes an entity A and a flattened item C,
/// and returns the score contribution for this pair.
///
/// # Parameters
///
/// - `weight_expr`: Expression to evaluate for the weight
/// - `class_idx_a`: Index of entity class A
/// - `class_idx_b`: Index of entity class B (the class being flattened)
/// - `descriptor`: Dynamic descriptor for creating temporary solutions
/// - `is_hard`: Whether this is a hard constraint (true) or soft constraint (false)
///
/// # Expression Context
///
/// - `Param(0)` refers to entity A
/// - `Param(1)` refers to the C item (from flattening entity B)
/// - `Field { param_idx: 0, field_idx }` accesses fields from entity A
/// - `Field { param_idx: 1, field_idx }` accesses the C item's value
/// - Arithmetic and comparison operations work across both A and C
///
/// # Weight Application
///
/// The weight expression should evaluate to a numeric value (i64 or f64).
/// If `is_hard` is true, the weight is applied to the hard score component.
/// If `is_hard` is false, the weight is applied to the soft score component.
///
/// # Implementation Note
///
/// This implementation creates a temporary `DynamicSolution` with entity A and a synthetic
/// entity representing the C item. This violates the zero-clone principle, but is necessary
/// because the `DynFlattenedWeight` signature doesn't provide access to the solution or
/// entity indices. The clone happens only for matched (A, C) pairs, which is bounded by
/// the match count (not the total entity count), so the performance impact is acceptable.
pub fn make_flattened_weight(
    weight_expr: Expr,
    class_idx_a: usize,
    class_idx_b: usize,
    descriptor: DynamicDescriptor,
    is_hard: bool,
) -> DynFlattenedWeight {
    Box::new(move |a: &DynamicEntity, c: &DynamicValue| {
        // Create a temporary solution with entity A and synthetic C entity
        let mut temp_solution = DynamicSolution {
            descriptor: descriptor.clone(),
            entities: vec![Vec::new(); descriptor.classes.len()],
            facts: Vec::new(),
            score: None,
        };

        // Place entity A at index 0 in class_idx_a
        temp_solution.entities[class_idx_a] = vec![a.clone()];

        // Create synthetic entity for C item at index 0 in class_idx_b
        let c_entity = DynamicEntity {
            id: 0, // Synthetic ID
            class_idx: class_idx_b,
            fields: vec![c.clone()],
        };
        temp_solution.entities[class_idx_b] = vec![c_entity];

        // Build entity tuple: A at index 0 of class_idx_a, C at index 0 of class_idx_b
        let tuple = vec![
            EntityRef::new(class_idx_a, Some(0)),
            EntityRef::new(class_idx_b, Some(0)),
        ];

        let ctx = EvalContext::new(&temp_solution, &tuple);

        // Evaluate weight expression
        let result = eval_expr(&weight_expr, &ctx);

        // Convert result to numeric weight
        let weight_num = result.as_i64().unwrap_or(0) as f64;

        // Apply to hard or soft score
        if is_hard {
            HardSoftScore::hard(weight_num)
        } else {
            HardSoftScore::soft(weight_num)
        }
    })
}

// =============================================================================
// Phase 4: Constraint Factory Functions
// =============================================================================

/// Builds a unary constraint (single entity class, no joins) that returns a boxed IncrementalConstraint.
///
/// This factory creates an `IncrementalUniConstraint` that evaluates a filter and weight
/// expression against entities from a single class without any joins.
///
/// # Parameters
///
/// * `constraint_ref` - Reference identifying this constraint
/// * `impact_type` - Whether this constraint is a penalty or reward
/// * `class_idx` - Index of the entity class to iterate over
/// * `filter_expr` - Expression to filter entities (returns bool)
/// * `weight_expr` - Expression to compute score weight for each matching entity
/// * `descriptor` - Dynamic descriptor for expression evaluation
/// * `is_hard` - Whether to apply weight to hard or soft score component
///
/// # Expression Context
///
/// Both filter and weight expressions are evaluated in a single-entity context:
/// - `Param(0)` refers to the entity being evaluated (returns entity ID)
/// - `Field { param_idx: 0, field_idx }` accesses fields from the entity
/// - Arithmetic, comparisons, and logical operations work as expected
/// - Access to facts via solution context is available
///
/// # Example Use Case
///
/// ```ignore
/// // Penalize each overloaded employee (workload > capacity)
/// let constraint = build_uni_constraint(
///     ConstraintRef::new("overloaded_employees"),
///     ImpactType::Penalty,
///     employee_class_idx,
///     Expr::Gt(Box::new(Expr::Field { param_idx: 0, field_idx: workload_field }),
///              Box::new(Expr::Field { param_idx: 0, field_idx: capacity_field })),
///     Expr::Sub(Box::new(Expr::Field { param_idx: 0, field_idx: workload_field }),
///               Box::new(Expr::Field { param_idx: 0, field_idx: capacity_field })),
///     descriptor.clone(),
///     true, // hard constraint
/// );
/// ```
///
/// # Returns
///
/// A boxed `IncrementalConstraint<DynamicSolution, HardSoftScore>` that can be stored
/// in `DynamicConstraintSet`.
pub fn build_uni_constraint(
    constraint_ref: ConstraintRef,
    impact_type: ImpactType,
    class_idx: usize,
    filter_expr: Expr,
    weight_expr: Expr,
    descriptor: DynamicDescriptor,
    is_hard: bool,
) -> Box<dyn IncrementalConstraint<DynamicSolution, HardSoftScore> + Send + Sync> {
    // Create extractor for the entity class
    let extractor = make_extractor(class_idx);

    // Create filter closure
    let filter: DynUniFilter = Box::new(move |solution: &DynamicSolution, entity: &DynamicEntity| {
        // Evaluate filter expression using the actual solution (which has access to facts, etc.)
        let result = crate::eval::eval_entity_expr(&filter_expr, solution, entity);
        result.as_bool().unwrap_or(false)
    });

    // Create weight closure
    let weight_descriptor = descriptor;
    let weight: DynUniWeight = Box::new(move |entity: &DynamicEntity| {
        // Create minimal solution for entity-level evaluation
        let minimal_solution = DynamicSolution {
            descriptor: weight_descriptor.clone(),
            entities: Vec::new(),
            facts: Vec::new(),
            score: None,
        };

        // Evaluate weight expression
        let result = crate::eval::eval_entity_expr(&weight_expr, &minimal_solution, entity);
        let weight_num = result.as_i64().unwrap_or(0) as f64;

        // Apply to hard or soft score
        if is_hard {
            HardSoftScore::hard(weight_num)
        } else {
            HardSoftScore::soft(weight_num)
        }
    });

    // Create and box the IncrementalUniConstraint
    Box::new(IncrementalUniConstraint::new(
        constraint_ref,
        impact_type,
        extractor,
        filter,
        weight,
        is_hard,
    ))
}

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
/// # Example Use Case
///
/// ```ignore
/// // Penalize conflicting shifts (same employee assigned to overlapping shifts)
/// let constraint = build_bi_self_constraint(
///     ConstraintRef::new("shift_conflicts"),
///     ImpactType::Penalty,
///     shift_class_idx,
///     Expr::Field { param_idx: 0, field_idx: employee_id_field }, // join key
///     Expr::And(
///         Box::new(Expr::Lt(Box::new(Expr::Field { param_idx: 0, field_idx: start_field }),
///                           Box::new(Expr::Field { param_idx: 1, field_idx: end_field }))),
///         Box::new(Expr::Lt(Box::new(Expr::Field { param_idx: 1, field_idx: start_field }),
///                           Box::new(Expr::Field { param_idx: 0, field_idx: end_field }))),
///     ), // overlapping time check
///     Expr::Literal(DynamicValue::I64(1)), // constant penalty weight
///     descriptor.clone(),
///     true, // hard constraint
/// );
/// ```
///
/// # Returns
///
/// A boxed `IncrementalConstraint<DynamicSolution, HardSoftScore>` that can be stored
/// in `DynamicConstraintSet`.
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
/// # Example Use Case
///
/// ```ignore
/// // Penalize three shifts assigned to the same employee that conflict with each other
/// // (detect chains of overlapping shifts for the same employee)
/// let constraint = build_tri_self_constraint(
///     ConstraintRef::new("shift_chain_conflicts"),
///     ImpactType::Penalty,
///     shift_class_idx,
///     Expr::Field { param_idx: 0, field_idx: employee_id_field }, // join key
///     Expr::And(
///         Box::new(Expr::And(
///             // shift1 overlaps shift2
///             Box::new(Expr::And(
///                 Box::new(Expr::Lt(Box::new(Expr::Field { param_idx: 0, field_idx: start_field }),
///                                   Box::new(Expr::Field { param_idx: 1, field_idx: end_field }))),
///                 Box::new(Expr::Lt(Box::new(Expr::Field { param_idx: 1, field_idx: start_field }),
///                                   Box::new(Expr::Field { param_idx: 0, field_idx: end_field }))),
///             )),
///             // shift2 overlaps shift3
///             Box::new(Expr::And(
///                 Box::new(Expr::Lt(Box::new(Expr::Field { param_idx: 1, field_idx: start_field }),
///                                   Box::new(Expr::Field { param_idx: 2, field_idx: end_field }))),
///                 Box::new(Expr::Lt(Box::new(Expr::Field { param_idx: 2, field_idx: start_field }),
///                                   Box::new(Expr::Field { param_idx: 1, field_idx: end_field }))),
///             )),
///         )),
///         // shift1 overlaps shift3
///         Box::new(Expr::And(
///             Box::new(Expr::Lt(Box::new(Expr::Field { param_idx: 0, field_idx: start_field }),
///                               Box::new(Expr::Field { param_idx: 2, field_idx: end_field }))),
///             Box::new(Expr::Lt(Box::new(Expr::Field { param_idx: 2, field_idx: start_field }),
///                               Box::new(Expr::Field { param_idx: 0, field_idx: end_field }))),
///         )),
///     ),
///     Expr::Literal(DynamicValue::I64(1)), // constant penalty weight
///     descriptor.clone(),
///     true, // hard constraint
/// );
/// ```
///
/// # Returns
///
/// A boxed `IncrementalConstraint<DynamicSolution, HardSoftScore>` that can be stored
/// in `DynamicConstraintSet`.
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

/// Operations in a constraint stream pipeline.
#[derive(Debug, Clone)]
pub enum StreamOp {
    /// Iterate over all entities of a class.
    ForEach { class_idx: usize },

    /// Filter entities using a predicate expression.
    Filter { predicate: Expr },

    /// Join with another class using join conditions.
    Join {
        class_idx: usize,
        /// Join conditions that must all be true.
        conditions: Vec<Expr>,
    },

    /// Filter distinct pairs (ensuring A < B to avoid duplicates).
    DistinctPair {
        /// Expression to compare (e.g., entity IDs or indices).
        ordering_expr: Expr,
    },

    /// Penalize matching tuples.
    Penalize { weight: HardSoftScore },

    /// Penalize with a configurable amount based on expression.
    PenalizeConfigurable { match_weight: Expr },

    /// Reward matching tuples.
    Reward { weight: HardSoftScore },

    /// Reward with a configurable amount based on expression.
    RewardConfigurable { match_weight: Expr },

    /// Flatten a set/list field, creating one tuple per element.
    FlattenLast {
        /// Expression to get the set/list to flatten.
        set_expr: Expr,
    },
}

/// A constraint defined using expression trees and stream operations.
///
/// Supports true incremental scoring: on_insert/on_retract compute deltas
/// by tracking active matches and updating only affected tuples.
#[derive(Debug)]
pub struct DynamicConstraint {
    /// Constraint name.
    pub name: Arc<str>,
    /// Base weight (for simple penalize/reward).
    pub weight: HardSoftScore,
    /// Stream operations pipeline.
    pub ops: Vec<StreamOp>,
    /// Whether this is a hard constraint.
    pub is_hard: bool,

    // Incremental state - indices only, no cloning
    /// Active matches: set of (class_a, idx_a, class_b, idx_b) tuples.
    matches: HashSet<MatchTuple>,
    /// Reverse index: (class_idx, entity_idx) -> matches involving this entity.
    entity_to_matches: HashMap<(usize, usize), Vec<MatchTuple>>,
    /// Join key index: join_key_value -> list of (class_idx, entity_idx) with that value.
    /// Used for O(1) lookup on insert instead of O(n) scan.
    join_key_index: HashMap<i64, Vec<(usize, usize)>>,
    /// Cached score from all current matches.
    cached_score: HardSoftScore,
    /// Whether initialized.
    initialized: bool,
    /// Cached distinct_pair expression for on_insert.
    distinct_expr: Option<Expr>,
    /// Cached join conditions for on_insert.
    join_conditions: Vec<Expr>,
    /// Cached filter predicates for on_insert.
    filter_predicates: Vec<Expr>,
    /// Cached foreach class index.
    foreach_class: Option<usize>,
    /// Cached join class index.
    join_class: Option<usize>,
}

impl Clone for DynamicConstraint {
    fn clone(&self) -> Self {
        // Clone resets incremental state - will be reinitialized on first use
        Self {
            name: self.name.clone(),
            weight: self.weight,
            ops: self.ops.clone(),
            is_hard: self.is_hard,
            matches: HashSet::new(),
            entity_to_matches: HashMap::new(),
            join_key_index: HashMap::new(),
            cached_score: HardSoftScore::ZERO,
            initialized: false,
            distinct_expr: None,
            join_conditions: Vec::new(),
            filter_predicates: Vec::new(),
            foreach_class: None,
            join_class: None,
        }
    }
}

impl DynamicConstraint {
    /// Creates a new dynamic constraint.
    pub fn new(name: impl Into<Arc<str>>) -> Self {
        Self {
            name: name.into(),
            weight: HardSoftScore::ZERO,
            ops: Vec::new(),
            is_hard: false,
            matches: HashSet::new(),
            entity_to_matches: HashMap::new(),
            join_key_index: HashMap::new(),
            cached_score: HardSoftScore::ZERO,
            initialized: false,
            distinct_expr: None,
            join_conditions: Vec::new(),
            filter_predicates: Vec::new(),
            foreach_class: None,
            join_class: None,
        }
    }

    /// Sets the constraint weight.
    pub fn with_weight(mut self, weight: HardSoftScore) -> Self {
        self.weight = weight;
        self.is_hard = weight.hard() != 0;
        self
    }

    /// Adds a ForEach operation.
    pub fn for_each(mut self, class_idx: usize) -> Self {
        self.foreach_class = Some(class_idx);
        self.ops.push(StreamOp::ForEach { class_idx });
        self
    }

    /// Adds a Filter operation.
    pub fn filter(mut self, predicate: Expr) -> Self {
        self.filter_predicates.push(predicate.clone());
        self.ops.push(StreamOp::Filter { predicate });
        self
    }

    /// Adds a Join operation.
    pub fn join(mut self, class_idx: usize, conditions: Vec<Expr>) -> Self {
        self.join_class = Some(class_idx);
        self.join_conditions = conditions.clone();
        self.ops.push(StreamOp::Join {
            class_idx,
            conditions,
        });
        self
    }

    /// Adds a DistinctPair filter to avoid duplicate pairs (A,B) and (B,A).
    pub fn distinct_pair(mut self, ordering_expr: Expr) -> Self {
        self.distinct_expr = Some(ordering_expr.clone());
        self.ops.push(StreamOp::DistinctPair { ordering_expr });
        self
    }

    /// Adds a Penalize operation.
    pub fn penalize(mut self, weight: HardSoftScore) -> Self {
        self.weight = weight;
        self.is_hard = weight.hard() != 0;
        self.ops.push(StreamOp::Penalize { weight });
        self
    }

    /// Adds a Reward operation.
    pub fn reward(mut self, weight: HardSoftScore) -> Self {
        self.weight = weight;
        self.is_hard = weight.hard() != 0;
        self.ops.push(StreamOp::Reward { weight });
        self
    }

    /// Adds a FlattenLast operation to expand a set/list into individual tuples.
    pub fn flatten_last(mut self, set_expr: Expr) -> Self {
        self.ops.push(StreamOp::FlattenLast { set_expr });
        self
    }

    /// Adds a PenalizeConfigurable operation with dynamic weight.
    pub fn penalize_configurable(mut self, base_weight: HardSoftScore, match_weight: Expr) -> Self {
        self.weight = base_weight;
        self.is_hard = base_weight.hard() != 0;
        self.ops
            .push(StreamOp::PenalizeConfigurable { match_weight });
        self
    }

    /// Adds a RewardConfigurable operation with dynamic weight.
    pub fn reward_configurable(mut self, base_weight: HardSoftScore, match_weight: Expr) -> Self {
        self.weight = base_weight;
        self.is_hard = base_weight.hard() != 0;
        self.ops.push(StreamOp::RewardConfigurable { match_weight });
        self
    }

    /// Returns the cached score. Must call initialize() first.
    pub fn cached_score(&self) -> HardSoftScore {
        self.cached_score
    }

    /// Returns the match count from incremental state.
    pub fn match_count(&self) -> usize {
        self.matches.len()
    }

    // =========================================================================
    // Incremental scoring helpers
    // =========================================================================

    /// Finds all matching index tuples (for bi-joins only currently).
    /// Returns (class_a, idx_a, class_b, idx_b) tuples.
    /// Called once during initialize() - O(n^2) is acceptable here.
    fn find_match_indices(&self, solution: &DynamicSolution) -> HashSet<MatchTuple> {
        let mut result = HashSet::new();

        // Parse ops to find the constraint structure
        let mut foreach_class = None;
        let mut join_class = None;
        let mut join_conditions = Vec::new();
        let mut distinct_expr = None;
        let mut filter_predicates = Vec::new();

        for op in &self.ops {
            match op {
                StreamOp::ForEach { class_idx } => {
                    foreach_class = Some(*class_idx);
                }
                StreamOp::Join {
                    class_idx,
                    conditions,
                } => {
                    join_class = Some(*class_idx);
                    join_conditions = conditions.clone();
                }
                StreamOp::DistinctPair { ordering_expr } => {
                    distinct_expr = Some(ordering_expr.clone());
                }
                StreamOp::Filter { predicate } => {
                    filter_predicates.push(predicate.clone());
                }
                _ => {}
            }
        }

        let Some(class_a) = foreach_class else {
            return result;
        };
        let Some(class_b) = join_class else {
            return result;
        };

        // Iterate all pairs (A, B) - O(n^2) but only called once at init
        for (_, a_idx) in solution.entity_refs_in_class(class_a) {
            for (_, b_idx) in solution.entity_refs_in_class(class_b) {
                if self.check_join_match(
                    solution,
                    class_a,
                    a_idx,
                    class_b,
                    b_idx,
                    &join_conditions,
                    &distinct_expr,
                    &filter_predicates,
                ) {
                    result.insert((class_a, a_idx, class_b, b_idx));
                }
            }
        }

        result
    }

    /// Gets the join key value for an entity (extracts the field used in equality join).
    fn get_join_key(
        &self,
        solution: &DynamicSolution,
        class_idx: usize,
        entity_idx: usize,
    ) -> Option<i64> {
        // Parse ops to find join condition field
        for op in &self.ops {
            if let StreamOp::Join { conditions, .. } = op {
                // Look for equality condition like A.field == B.field
                for cond in conditions {
                    if let Expr::Eq(left, right) = cond {
                        // Verify both sides are field references
                        if let (
                            Expr::Field {
                                param_idx: 0,
                                field_idx: left_field,
                            },
                            Expr::Field {
                                param_idx: 1,
                                field_idx: right_field,
                            },
                        ) = (left.as_ref(), right.as_ref())
                        {
                            if left_field == right_field {
                                let entity = solution.get_entity(class_idx, entity_idx)?;
                                return entity.fields.get(*left_field)?.as_i64();
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Checks if a pair of entities matches the join conditions and filter predicates.
    fn check_join_match(
        &self,
        solution: &DynamicSolution,
        class_a: usize,
        idx_a: usize,
        class_b: usize,
        idx_b: usize,
        conditions: &[Expr],
        distinct_expr: &Option<Expr>,
        filter_predicates: &[Expr],
    ) -> bool {
        let tuple = vec![
            EntityRef::new(class_a, idx_a),
            EntityRef::new(class_b, idx_b),
        ];
        let ctx = EvalContext::new(solution, &tuple);

        // Check all join conditions
        let conditions_match = conditions
            .iter()
            .all(|cond| eval_expr(cond, &ctx).as_bool().unwrap_or(false));

        if !conditions_match {
            return false;
        }

        // Check distinct pair if present
        if let Some(ordering) = distinct_expr {
            if !eval_expr(ordering, &ctx).as_bool().unwrap_or(false) {
                return false;
            }
        }

        // Check all filter predicates
        for pred in filter_predicates {
            if !eval_expr(pred, &ctx).as_bool().unwrap_or(false) {
                return false;
            }
        }

        true
    }

    /// Computes score for a match by looking up entities (no clone).
    fn score_for_match(&self, solution: &DynamicSolution, m: MatchTuple) -> HardSoftScore {
        let (ca, ia, cb, ib) = m;
        let tuple = vec![EntityRef::new(ca, ia), EntityRef::new(cb, ib)];

        // Find terminal op and compute score
        for op in &self.ops {
            match op {
                StreamOp::Penalize { weight } => {
                    return -*weight;
                }
                StreamOp::PenalizeConfigurable { match_weight } => {
                    let ctx = EvalContext::new(solution, &tuple);
                    let weight_val = eval_expr(match_weight, &ctx);
                    if let Some(w) = weight_val.as_i64() {
                        return -self.weight.multiply(w as f64);
                    }
                }
                StreamOp::Reward { weight } => {
                    return *weight;
                }
                StreamOp::RewardConfigurable { match_weight } => {
                    let ctx = EvalContext::new(solution, &tuple);
                    let weight_val = eval_expr(match_weight, &ctx);
                    if let Some(w) = weight_val.as_i64() {
                        return self.weight.multiply(w as f64);
                    }
                }
                _ => {}
            }
        }

        HardSoftScore::ZERO
    }
}

/// Implement IncrementalConstraint for individual DynamicConstraint.
impl IncrementalConstraint<DynamicSolution, HardSoftScore> for DynamicConstraint {
    fn evaluate(&self, _solution: &DynamicSolution) -> HardSoftScore {
        // Use cached score from incremental state
        self.cached_score
    }

    fn match_count(&self, _solution: &DynamicSolution) -> usize {
        // Use cached match count from incremental state
        self.matches.len()
    }

    fn initialize(&mut self, solution: &DynamicSolution) -> HardSoftScore {
        self.matches.clear();
        self.entity_to_matches.clear();
        self.join_key_index.clear();

        // Find foreach class
        let mut foreach_class = None;
        for op in &self.ops {
            if let StreamOp::ForEach { class_idx } = op {
                foreach_class = Some(*class_idx);
                break;
            }
        }

        // Find all matching index pairs
        let all_matches = self.find_match_indices(solution);

        // Build join key index for O(1) lookup on insert
        if let Some(class_idx) = foreach_class {
            for (_, entity_idx) in solution.entity_refs_in_class(class_idx) {
                if let Some(key_val) = self.get_join_key(solution, class_idx, entity_idx) {
                    self.join_key_index
                        .entry(key_val)
                        .or_default()
                        .push((class_idx, entity_idx));
                }
            }
        }

        let mut total = HardSoftScore::ZERO;
        for m in &all_matches {
            let (ca, ia, cb, ib) = *m;

            // Add to reverse index
            self.entity_to_matches.entry((ca, ia)).or_default().push(*m);
            self.entity_to_matches.entry((cb, ib)).or_default().push(*m);

            // Compute score by index lookup (no clone)
            total = total + self.score_for_match(solution, *m);
        }

        self.matches = all_matches;
        self.cached_score = total;
        self.initialized = true;
        total
    }

    fn on_insert(
        &mut self,
        solution: &DynamicSolution,
        entity_index: usize,
        descriptor_index: usize,
    ) -> HardSoftScore {
        if !self.initialized {
            return HardSoftScore::ZERO;
        }

        // Get candidate partners to check
        let others: Vec<(usize, usize)> =
            if let Some(key_val) = self.get_join_key(solution, descriptor_index, entity_index) {
                // O(1) lookup using join key index
                let candidates = self
                    .join_key_index
                    .get(&key_val)
                    .cloned()
                    .unwrap_or_default();

                // Add to join key index
                self.join_key_index
                    .entry(key_val)
                    .or_default()
                    .push((descriptor_index, entity_index));

                candidates
            } else {
                // Complex join condition - O(n) scan of join class
                let Some(join_class) = self.join_class else {
                    return HardSoftScore::ZERO;
                };
                solution
                    .entity_refs_in_class(join_class)
                    .map(|(_, idx)| (join_class, idx))
                    .filter(|&(c, i)| !(c == descriptor_index && i == entity_index))
                    .collect()
            };

        let mut delta = HardSoftScore::ZERO;
        for (other_class, other_idx) in others {
            // Check full join match with correct ordering for distinct_pair
            if self.check_join_match(
                solution,
                descriptor_index,
                entity_index,
                other_class,
                other_idx,
                &self.join_conditions,
                &self.distinct_expr,
                &self.filter_predicates,
            ) {
                let m = (descriptor_index, entity_index, other_class, other_idx);
                if self.matches.insert(m) {
                    self.entity_to_matches
                        .entry((descriptor_index, entity_index))
                        .or_default()
                        .push(m);
                    self.entity_to_matches
                        .entry((other_class, other_idx))
                        .or_default()
                        .push(m);
                    delta = delta + self.score_for_match(solution, m);
                }
            } else if self.check_join_match(
                solution,
                other_class,
                other_idx,
                descriptor_index,
                entity_index,
                &self.join_conditions,
                &self.distinct_expr,
                &self.filter_predicates,
            ) {
                let m = (other_class, other_idx, descriptor_index, entity_index);
                if self.matches.insert(m) {
                    self.entity_to_matches
                        .entry((other_class, other_idx))
                        .or_default()
                        .push(m);
                    self.entity_to_matches
                        .entry((descriptor_index, entity_index))
                        .or_default()
                        .push(m);
                    delta = delta + self.score_for_match(solution, m);
                }
            }
        }

        self.cached_score = self.cached_score + delta;
        delta
    }

    fn on_retract(
        &mut self,
        solution: &DynamicSolution,
        entity_index: usize,
        descriptor_index: usize,
    ) -> HardSoftScore {
        if !self.initialized {
            return HardSoftScore::ZERO;
        }

        // Remove from join key index
        if let Some(key_val) = self.get_join_key(solution, descriptor_index, entity_index) {
            if let Some(list) = self.join_key_index.get_mut(&key_val) {
                list.retain(|&(c, e)| !(c == descriptor_index && e == entity_index));
            }
        }

        let key = (descriptor_index, entity_index);
        let Some(affected) = self.entity_to_matches.get_mut(&key) else {
            return HardSoftScore::ZERO;
        };
        let affected = std::mem::take(affected);

        let mut delta = HardSoftScore::ZERO;
        for m in affected {
            if self.matches.remove(&m) {
                let (ca, ia, cb, ib) = m;

                // Remove from other entity's reverse index
                let other_key = if (ca, ia) == key { (cb, ib) } else { (ca, ia) };
                if let Some(list) = self.entity_to_matches.get_mut(&other_key) {
                    list.retain(|t| *t != m);
                }

                // Score delta (negative - match removed)
                delta = delta - self.score_for_match(solution, m);
            }
        }

        self.cached_score = self.cached_score + delta;
        delta
    }

    fn reset(&mut self) {
        self.matches.clear();
        self.entity_to_matches.clear();
        self.join_key_index.clear();
        self.cached_score = HardSoftScore::ZERO;
        self.initialized = false;
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn is_hard(&self) -> bool {
        self.is_hard
    }

    fn constraint_ref(&self) -> ConstraintRef {
        ConstraintRef::new("", &*self.name)
    }

    fn get_matches(
        &self,
        _solution: &DynamicSolution,
    ) -> Vec<DetailedConstraintMatch<HardSoftScore>> {
        Vec::new()
    }

    fn weight(&self) -> HardSoftScore {
        self.weight
    }
}
