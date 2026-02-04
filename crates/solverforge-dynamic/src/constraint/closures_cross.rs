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

/// Checks if an expression contains any constructs that won't work correctly
/// in a cross-join key context (where entities/facts are not available).
///
/// Returns a list of warning messages for unsupported constructs found.
pub(crate) fn check_key_expr_limitations(expr: &Expr) -> Vec<String> {
    let mut warnings = Vec::new();
    check_key_expr_limitations_recursive(expr, &mut warnings);
    warnings
}

fn check_key_expr_limitations_recursive(expr: &Expr, warnings: &mut Vec<String>) {
    match expr {
        Expr::RefField { ref_expr, .. } => {
            warnings.push(
                "Key expression contains RefField which requires entity/fact lookup. \
                 Join keys use a minimal context without entities/facts, so RefField \
                 will produce incorrect results (DynamicValue::None). Consider using \
                 a filter expression instead where the full solution is available."
                    .to_string(),
            );
            check_key_expr_limitations_recursive(ref_expr, warnings);
        }
        Expr::Param(n) if *n > 0 => {
            warnings.push(format!(
                "Key expression contains Param({}) but only Param(0) (the current entity) \
                 is available in key context. Other parameter references will fail.",
                n
            ));
        }
        // Recursively check nested expressions
        Expr::Eq(a, b)
        | Expr::Ne(a, b)
        | Expr::Lt(a, b)
        | Expr::Le(a, b)
        | Expr::Gt(a, b)
        | Expr::Ge(a, b)
        | Expr::And(a, b)
        | Expr::Or(a, b)
        | Expr::Add(a, b)
        | Expr::Sub(a, b)
        | Expr::Mul(a, b)
        | Expr::Div(a, b)
        | Expr::Mod(a, b)
        | Expr::Contains(a, b)
        | Expr::Min(a, b)
        | Expr::Max(a, b) => {
            check_key_expr_limitations_recursive(a, warnings);
            check_key_expr_limitations_recursive(b, warnings);
        }
        Expr::SetContains {
            set_expr,
            value_expr,
        } => {
            check_key_expr_limitations_recursive(set_expr, warnings);
            check_key_expr_limitations_recursive(value_expr, warnings);
        }
        Expr::Not(e) | Expr::Abs(e) | Expr::Neg(e) | Expr::IsNotNone(e) | Expr::IsNone(e) | Expr::DateOf(e) => {
            check_key_expr_limitations_recursive(e, warnings);
        }
        Expr::If {
            cond,
            then_expr,
            else_expr,
        } => {
            check_key_expr_limitations_recursive(cond, warnings);
            check_key_expr_limitations_recursive(then_expr, warnings);
            check_key_expr_limitations_recursive(else_expr, warnings);
        }
        Expr::Overlaps {
            start1,
            end1,
            start2,
            end2,
        }
        | Expr::OverlapMinutes {
            start1,
            end1,
            start2,
            end2,
        } => {
            check_key_expr_limitations_recursive(start1, warnings);
            check_key_expr_limitations_recursive(end1, warnings);
            check_key_expr_limitations_recursive(start2, warnings);
            check_key_expr_limitations_recursive(end2, warnings);
        }
        Expr::OverlapsDate { start, end, date } | Expr::OverlapDateMinutes { start, end, date } => {
            check_key_expr_limitations_recursive(start, warnings);
            check_key_expr_limitations_recursive(end, warnings);
            check_key_expr_limitations_recursive(date, warnings);
        }
        // These are safe in key context
        Expr::Literal(_) | Expr::Param(0) | Expr::Field { .. } | Expr::FlattenedValue => {}
        // Catch-all for Param(0) which is matched above
        Expr::Param(_) => {}
    }
}

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
/// # Supported Expression Types
/// - `Param(0)` - Returns the entity's ID
/// - `Field { param_idx: 0, field_idx }` - Accesses direct fields from the entity
/// - `Literal(...)` - Constant values
/// - Arithmetic operations (`Add`, `Sub`, `Mul`, `Div`, `Mod`, `Abs`, `Neg`)
/// - Comparisons (`Eq`, `Ne`, `Lt`, `Le`, `Gt`, `Ge`)
/// - Logic (`And`, `Or`, `Not`)
/// - Conditionals (`If`)
/// - `IsNone`, `IsNotNone` - Null checks
///
/// # Unsupported Expression Types (Runtime Limitations)
/// The following expression types will produce incorrect results or `DynamicValue::None`
/// because key extractors use a minimal solution context without entities or facts:
///
/// - **`RefField`** - Cannot dereference entity references; the entities vector is empty,
///   so lookups via `Ref(class_idx, entity_idx)` will fail
/// - **Fact lookups** - Cannot access problem facts; the facts vector is empty
/// - **`Param(n)` for n > 0** - Only `Param(0)` (the current entity) is available
///
/// # Design Rationale
/// Join keys are used for incremental index maintenance. They must be:
/// 1. **Stable** - Computed solely from the entity's own fields
/// 2. **Self-contained** - No external lookups to other entities or facts
/// 3. **Deterministic** - Same entity always produces the same key
///
/// If your join key needs to reference other entities (via `RefField`) or facts,
/// consider restructuring your constraint to use a filter expression instead,
/// where the full solution context is available.
pub fn make_cross_key_a(key_expr: Expr, descriptor: DynamicDescriptor) -> DynCrossKeyA {
    // Check for unsupported expressions and emit runtime warnings
    let warnings = check_key_expr_limitations(&key_expr);
    for warning in warnings {
        eprintln!(
            "[solverforge-dynamic] WARNING in make_cross_key_a: {}",
            warning
        );
    }

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
/// # Supported Expression Types
/// - `Param(0)` - Returns the entity's ID
/// - `Field { param_idx: 0, field_idx }` - Accesses direct fields from the entity
/// - `Literal(...)` - Constant values
/// - Arithmetic operations (`Add`, `Sub`, `Mul`, `Div`, `Mod`, `Abs`, `Neg`)
/// - Comparisons (`Eq`, `Ne`, `Lt`, `Le`, `Gt`, `Ge`)
/// - Logic (`And`, `Or`, `Not`)
/// - Conditionals (`If`)
/// - `IsNone`, `IsNotNone` - Null checks
///
/// # Unsupported Expression Types (Runtime Limitations)
/// The following expression types will produce incorrect results or `DynamicValue::None`
/// because key extractors use a minimal solution context without entities or facts:
///
/// - **`RefField`** - Cannot dereference entity references; the entities vector is empty,
///   so lookups via `Ref(class_idx, entity_idx)` will fail
/// - **Fact lookups** - Cannot access problem facts; the facts vector is empty
/// - **`Param(n)` for n > 0** - Only `Param(0)` (the current entity) is available
///
/// # Design Rationale
/// Join keys are used for incremental index maintenance. They must be:
/// 1. **Stable** - Computed solely from the entity's own fields
/// 2. **Self-contained** - No external lookups to other entities or facts
/// 3. **Deterministic** - Same entity always produces the same key
///
/// If your join key needs to reference other entities (via `RefField`) or facts,
/// consider restructuring your constraint to use a filter expression instead,
/// where the full solution context is available.
pub fn make_cross_key_b(key_expr: Expr, descriptor: DynamicDescriptor) -> DynCrossKeyB {
    // Check for unsupported expressions and emit runtime warnings
    let warnings = check_key_expr_limitations(&key_expr);
    for warning in warnings {
        eprintln!(
            "[solverforge-dynamic] WARNING in make_cross_key_b: {}",
            warning
        );
    }

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
