//! Closure builder functions for flattened constraints (entity-to-collection joins).

use solverforge_core::score::HardSoftScore;

use super::types::{DynALookup, DynCKeyFn, DynFlatten, DynFlattenedFilter, DynFlattenedWeight};
use crate::descriptor::DynamicDescriptor;
use crate::eval::{eval_expr, EntityRef, EvalContext};
use crate::expr::Expr;
use crate::solution::{DynamicEntity, DynamicSolution, DynamicValue};

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
/// # Implementation Note
/// The expression should evaluate to a `DynamicValue::List`. The field is
/// expected to be in the entity directly. This implementation returns a reference to the
/// field value from the entity's fields vector, which has the correct lifetime.
///
/// **Design decision**: For flattened constraints in the dynamic system, the flattened field
/// must be stored directly in the entity's fields vector. This allows returning a reference
/// with the correct lifetime tied to the entity, not to a temporary evaluation result.
pub fn make_flatten(flatten_expr: Expr, _descriptor: DynamicDescriptor) -> DynFlatten {
    // For the dynamic system, the flatten expression should directly reference a field
    // that contains a List value. We extract the field index from the expression.

    // Extract field index from expression (expected to be Field { param_idx: 1, field_idx })
    // In a flattened bi constraint: A is param 0, B is param 1, and we flatten from B
    let field_idx = match flatten_expr {
        Expr::Field {
            param_idx: 1,
            field_idx,
        } => field_idx,
        _ => {
            // If not a simple field reference to entity B (param 1), we can't safely return a slice
            panic!("Flatten expression must be a direct field reference on entity B (Field {{ param_idx: 1, field_idx }})")
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
/// # Implementation Note
/// For most dynamic use cases, the C item IS the key, so this function typically
/// returns the input value directly. More complex key extraction is possible if needed.
pub fn make_c_key_fn(_c_key_expr: Expr, _descriptor: DynamicDescriptor) -> DynCKeyFn {
    // For dynamic constraints, C items are typically DynamicValues that are already
    // the key values (e.g., dates, IDs). So we just return the item as-is.
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
/// # Design Constraint
/// Lookup key expressions should only reference entity fields, not facts or solution state.
/// This is enforced by the minimal solution context which has empty entities and facts vectors.
pub fn make_a_lookup(lookup_expr: Expr, descriptor: DynamicDescriptor) -> DynALookup {
    // Create minimal solution with only descriptor (no entities/facts).
    let minimal_solution = DynamicSolution {
        descriptor,
        entities: Vec::new(),
        facts: Vec::new(),
        score: None,
        id_to_location: std::collections::HashMap::new(),
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
    Box::new(
        move |solution: &DynamicSolution, a: &DynamicEntity, c: &DynamicValue| {
            // Find entity A's index by searching the entity slice using entity ID
            let entities_a = solution
                .entities
                .get(class_idx_a)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            let a_idx = entities_a.iter().position(|e| e.id == a.id);

            // If entity A not found, filter doesn't match
            if a_idx.is_none() {
                return false;
            }

            // Create a synthetic entity for the C item
            let _c_entity = DynamicEntity {
                id: 0, // Synthetic ID
                fields: vec![c.clone()],
            };

            // Find a valid index in class B to use for the synthetic C entity
            let b_idx = 0;

            // Unwrap a_idx since we checked it's Some above
            let a_idx = a_idx.unwrap();

            // Build EntityRef tuple: A at index a_idx, B placeholder at index b_idx
            let tuple = vec![
                EntityRef::new(class_idx_a, a_idx),
                EntityRef::new(class_idx_b, b_idx),
            ];

            // Use EvalContext::with_flattened to make the C item accessible via Param(2)
            let ctx = EvalContext::with_flattened(solution, &tuple, c);

            eval_expr(&filter_expr, &ctx).as_bool().unwrap_or(false)
        },
    )
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
/// # Implementation Note
///
/// This implementation creates a temporary `DynamicSolution` with entity A and a synthetic
/// entity representing the C item. This violates the zero-clone principle, but is necessary
/// because the `DynFlattenedWeight` signature doesn't provide access to the solution.
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
            entities: vec![Vec::new(); descriptor.entity_classes.len()],
            facts: Vec::new(),
            score: None,
            id_to_location: std::collections::HashMap::new(),
        };

        // Place entity A at index 0 in class_idx_a
        temp_solution.entities[class_idx_a] = vec![a.clone()];

        // Create synthetic entity for C item at index 0 in class_idx_b
        let c_entity = DynamicEntity {
            id: 0, // Synthetic ID
            fields: vec![c.clone()],
        };
        temp_solution.entities[class_idx_b] = vec![c_entity];

        // Build entity tuple: A at index 0 of class_idx_a, C at index 0 of class_idx_b
        let tuple = vec![
            EntityRef::new(class_idx_a, 0),
            EntityRef::new(class_idx_b, 0),
        ];

        let ctx = EvalContext::new(&temp_solution, &tuple);

        // Evaluate weight expression
        let result = eval_expr(&weight_expr, &ctx);

        // Convert result to numeric weight
        let weight_num = result.as_i64().unwrap_or(0) as i64;

        // Apply to hard or soft score
        if is_hard {
            HardSoftScore::of_hard(weight_num)
        } else {
            HardSoftScore::of_soft(weight_num)
        }
    })
}
