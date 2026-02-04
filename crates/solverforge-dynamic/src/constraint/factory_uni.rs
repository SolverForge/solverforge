//! Factory function for building unary (single entity) constraints.

use solverforge_core::score::HardSoftScore;
use solverforge_core::{ConstraintRef, ImpactType};
use solverforge_scoring::api::constraint_set::IncrementalConstraint;
use solverforge_scoring::constraint::incremental::IncrementalUniConstraint;

use super::closures_extract::make_extractor;
use super::types::{DynUniFilter, DynUniWeight};
use crate::descriptor::DynamicDescriptor;
use crate::expr::Expr;
use crate::solution::{DynamicEntity, DynamicSolution};

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
/// # Example
///
/// This internal function is called by [`build_from_stream_ops`] when a uni-constraint
/// pattern is detected. Users should typically use the public API:
///
/// ```
/// use solverforge_dynamic::{
///     DynamicDescriptor, EntityClassDef, FieldDef, FieldType,
///     Expr, StreamOp, build_from_stream_ops,
/// };
/// use solverforge_core::{ConstraintRef, ImpactType};
/// use solverforge_core::score::HardSoftScore;
///
/// // Define an employee entity with workload and capacity fields
/// let mut descriptor = DynamicDescriptor::new();
/// descriptor.add_entity_class(EntityClassDef::new(
///     "Employee",
///     vec![
///         FieldDef::new("id", FieldType::I64),
///         FieldDef::new("workload", FieldType::I64),
///         FieldDef::new("capacity", FieldType::I64),
///     ],
/// ));
///
/// // Build a uni-constraint via the public stream API:
/// // Penalize each overloaded employee (workload > capacity)
/// let ops = vec![
///     StreamOp::ForEach { class_idx: 0 },
///     StreamOp::Filter {
///         predicate: Expr::gt(Expr::field(0, 1), Expr::field(0, 2)),
///     },
///     StreamOp::Penalize { weight: HardSoftScore::of_hard(1) },
/// ];
///
/// let constraint = build_from_stream_ops(
///     ConstraintRef::new("", "overloaded_employees"),
///     ImpactType::Penalty,
///     &ops,
///     descriptor,
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
    let filter: DynUniFilter =
        Box::new(move |solution: &DynamicSolution, entity: &DynamicEntity| {
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
            id_to_location: std::collections::HashMap::new(),
        };

        // Evaluate weight expression
        let result = crate::eval::eval_entity_expr(&weight_expr, &minimal_solution, entity);
        let weight_num = result.as_i64().unwrap_or(0);

        // Apply to hard or soft score
        if is_hard {
            HardSoftScore::of_hard(weight_num)
        } else {
            HardSoftScore::of_soft(weight_num)
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
