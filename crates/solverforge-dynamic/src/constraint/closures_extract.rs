//! Closure builder functions for entity extraction and key extraction.

use super::types::{DynExtractor, DynKeyExtractor};
use crate::descriptor::DynamicDescriptor;
use crate::expr::Expr;
use crate::solution::{DynamicEntity, DynamicSolution};

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
        solution
            .entities
            .get(class_idx)
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
    let minimal_solution = DynamicSolution::empty(descriptor);

    Box::new(move |entity: &DynamicEntity| {
        crate::eval::eval_entity_expr(&key_expr, &minimal_solution, entity)
    })
}
