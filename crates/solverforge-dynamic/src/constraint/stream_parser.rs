//! Parsing and analysis of stream operation pipelines to detect constraint patterns.

use super::stream_ops::StreamOp;
use crate::expr::Expr;
use crate::solution::DynamicValue;

/// Internal representation of a constraint pattern parsed from StreamOps.
#[derive(Debug)]
pub enum ConstraintPattern {
    /// Single entity constraint (ForEach + Filter + Penalize/Reward)
    Uni {
        class_idx: usize,
        filter_expr: Expr,
        weight_expr: Expr,
    },

    /// Two entities from same class (ForEach + Join(same) + DistinctPair + ...)
    BiSelfJoin {
        class_idx: usize,
        key_expr: Expr,
        filter_expr: Expr,
        weight_expr: Expr,
    },

    /// Three entities from same class
    TriSelfJoin {
        class_idx: usize,
        key_expr: Expr,
        filter_expr: Expr,
        weight_expr: Expr,
    },

    /// Four entities from same class
    QuadSelfJoin {
        class_idx: usize,
        key_expr: Expr,
        filter_expr: Expr,
        weight_expr: Expr,
    },

    /// Five entities from same class
    PentaSelfJoin {
        class_idx: usize,
        key_expr: Expr,
        filter_expr: Expr,
        weight_expr: Expr,
    },

    /// Two entities from different classes (ForEach(A) + Join(B) + ...)
    CrossBi {
        class_idx_a: usize,
        class_idx_b: usize,
        key_expr_a: Expr,
        key_expr_b: Expr,
        filter_expr: Expr,
        weight_expr: Expr,
    },

    /// Flattened collection join (ForEach(A) + Join(B) + FlattenLast + ...)
    FlattenedBi {
        class_idx_a: usize,
        class_idx_b: usize,
        key_expr_a: Expr,
        key_expr_b: Expr,
        flatten_expr: Expr,
        c_key_expr: Expr,
        a_lookup_expr: Expr,
        filter_expr: Expr,
        weight_expr: Expr,
    },
}

/// Parses a StreamOp pipeline to extract the constraint pattern.
///
/// This function:
/// 1. Counts joins to determine arity
/// 2. Compares class indices to detect self-join vs cross-join
/// 3. Detects `FlattenLast` for flattened constraints
/// 4. Extracts filter and weight expressions
pub fn parse_stream_ops(ops: &[StreamOp]) -> ConstraintPattern {
    // Extract key information from the pipeline
    let mut class_indices = Vec::new();
    let mut join_conditions = Vec::new();
    let mut filters = Vec::new();
    let mut weight_expr = None;
    let mut flatten_expr = None;
    let mut _is_distinct_pair = false;

    // Scan through operations
    for op in ops {
        match op {
            StreamOp::ForEach { class_idx } => {
                class_indices.push(*class_idx);
            }
            StreamOp::Join {
                class_idx,
                conditions,
            } => {
                class_indices.push(*class_idx);
                join_conditions.extend(conditions.clone());
            }
            StreamOp::Filter { predicate } => {
                filters.push(predicate.clone());
            }
            StreamOp::DistinctPair { .. } => {
                _is_distinct_pair = true;
            }
            StreamOp::Penalize { weight } | StreamOp::Reward { weight } => {
                // Convert HardSoftScore to weight expression
                weight_expr = Some(if weight.hard() != 0 {
                    Expr::Literal(DynamicValue::I64(weight.hard()))
                } else {
                    Expr::Literal(DynamicValue::I64(weight.soft()))
                });
            }
            StreamOp::PenalizeConfigurable { match_weight }
            | StreamOp::RewardConfigurable { match_weight } => {
                weight_expr = Some(match_weight.clone());
            }
            StreamOp::FlattenLast { set_expr } => {
                flatten_expr = Some(set_expr.clone());
            }
        }
    }

    let arity = class_indices.len();
    let weight_expr = weight_expr.unwrap_or(Expr::Literal(DynamicValue::I64(1)));

    // Separate join conditions into those usable for keys vs those that become filters
    let (key_conditions, non_key_conditions): (Vec<_>, Vec<_>) =
        join_conditions.iter().partition(|cond| {
            if let Expr::Eq(left, right) = cond {
                is_simple_field_expr(left) && is_simple_field_expr(right)
            } else {
                false
            }
        });

    // Combine explicit filters with non-key join conditions
    let mut all_filters: Vec<Expr> = filters;
    all_filters.extend(non_key_conditions.into_iter().cloned());

    let filter_expr = if all_filters.is_empty() {
        Expr::Literal(DynamicValue::Bool(true))
    } else {
        combine_filters(&all_filters)
    };

    // Use only key conditions for key extraction
    let key_join_conditions: Vec<Expr> = key_conditions.into_iter().cloned().collect();

    // Detect pattern based on arity, class indices, and flatten operation
    match (arity, class_indices.as_slice(), flatten_expr) {
        // Uni-constraint: 1 entity
        (1, [class_idx], None) => ConstraintPattern::Uni {
            class_idx: *class_idx,
            filter_expr,
            weight_expr,
        },

        // Flattened bi-constraint: 2 entities + FlattenLast
        (2, [class_idx_a, class_idx_b], Some(flatten_expr)) => {
            let (key_expr_a, key_expr_b) = extract_join_keys(&key_join_conditions);
            let (c_key_expr, a_lookup_expr) = extract_flattened_lookup_keys(&filter_expr);

            ConstraintPattern::FlattenedBi {
                class_idx_a: *class_idx_a,
                class_idx_b: *class_idx_b,
                key_expr_a,
                key_expr_b,
                flatten_expr,
                c_key_expr,
                a_lookup_expr,
                filter_expr,
                weight_expr,
            }
        }

        // Cross-join bi-constraint: 2 entities from different classes
        (2, [class_idx_a, class_idx_b], None) if class_idx_a != class_idx_b => {
            let (key_expr_a, key_expr_b) = extract_join_keys(&key_join_conditions);
            ConstraintPattern::CrossBi {
                class_idx_a: *class_idx_a,
                class_idx_b: *class_idx_b,
                key_expr_a,
                key_expr_b,
                filter_expr,
                weight_expr,
            }
        }

        // Self-join bi-constraint: 2 entities from same class
        (2, [class_idx, _], None) => {
            let (key_expr, _) = extract_join_keys(&key_join_conditions);
            ConstraintPattern::BiSelfJoin {
                class_idx: *class_idx,
                key_expr,
                filter_expr,
                weight_expr,
            }
        }

        // Tri self-join: 3 entities from same class
        (3, [class_idx, _, _], None) => {
            let (key_expr, _) = extract_join_keys(&key_join_conditions);
            ConstraintPattern::TriSelfJoin {
                class_idx: *class_idx,
                key_expr,
                filter_expr,
                weight_expr,
            }
        }

        // Quad self-join: 4 entities from same class
        (4, [class_idx, _, _, _], None) => {
            let (key_expr, _) = extract_join_keys(&key_join_conditions);
            ConstraintPattern::QuadSelfJoin {
                class_idx: *class_idx,
                key_expr,
                filter_expr,
                weight_expr,
            }
        }

        // Penta self-join: 5 entities from same class
        (5, [class_idx, _, _, _, _], None) => {
            let (key_expr, _) = extract_join_keys(&key_join_conditions);
            ConstraintPattern::PentaSelfJoin {
                class_idx: *class_idx,
                key_expr,
                filter_expr,
                weight_expr,
            }
        }

        _ => panic!(
            "Unsupported constraint pattern: arity={}, classes={:?}",
            arity, class_indices
        ),
    }
}

/// Combines multiple filter expressions into a single AND expression.
fn combine_filters(filters: &[Expr]) -> Expr {
    if filters.is_empty() {
        Expr::Literal(DynamicValue::Bool(true))
    } else if filters.len() == 1 {
        filters[0].clone()
    } else {
        // Combine with AND
        let mut result = filters[0].clone();
        for filter in &filters[1..] {
            result = Expr::And(Box::new(result), Box::new(filter.clone()));
        }
        result
    }
}

/// Extracts join key expressions from join conditions.
///
/// For join conditions like `Param(0).field_x == Param(1).field_y`,
/// this extracts the left side (key for entity A) and right side (key for entity B).
///
/// Returns `(Expr::Literal(0), Expr::Literal(0))` as a safe default if no conditions are found.
fn extract_join_keys(conditions: &[Expr]) -> (Expr, Expr) {
    // Look for equality conditions to extract join keys
    for condition in conditions {
        if let Expr::Eq(left, right) = condition {
            // Check if both sides are simple field references to different params
            if is_simple_field_expr(left) && is_simple_field_expr(right) {
                let key_a = normalize_key_expr_to_param0((**left).clone());
                let key_b = normalize_key_expr_to_param0((**right).clone());
                return (key_a, key_b);
            }
        }
    }

    // Default: use a constant key (0) for Cartesian product
    (
        Expr::Literal(DynamicValue::I64(0)),
        Expr::Literal(DynamicValue::I64(0)),
    )
}

/// Checks if an expression is a simple field reference (Field { param_idx, field_idx })
/// that can be normalized for key extraction.
fn is_simple_field_expr(expr: &Expr) -> bool {
    matches!(expr, Expr::Field { .. })
}

/// Normalizes an expression to use param_idx 0 for single-entity key extraction.
fn normalize_key_expr_to_param0(expr: Expr) -> Expr {
    match expr {
        Expr::Field {
            param_idx: _,
            field_idx,
        } => Expr::Field {
            param_idx: 0,
            field_idx,
        },
        Expr::Param(_) => Expr::Param(0),
        Expr::Add(l, r) => Expr::Add(
            Box::new(normalize_key_expr_to_param0(*l)),
            Box::new(normalize_key_expr_to_param0(*r)),
        ),
        Expr::Sub(l, r) => Expr::Sub(
            Box::new(normalize_key_expr_to_param0(*l)),
            Box::new(normalize_key_expr_to_param0(*r)),
        ),
        Expr::Mul(l, r) => Expr::Mul(
            Box::new(normalize_key_expr_to_param0(*l)),
            Box::new(normalize_key_expr_to_param0(*r)),
        ),
        Expr::Div(l, r) => Expr::Div(
            Box::new(normalize_key_expr_to_param0(*l)),
            Box::new(normalize_key_expr_to_param0(*r)),
        ),
        other => other,
    }
}

/// Extracts c_key_expr and a_lookup_expr from a flattened-bi filter expression.
///
/// In flattened-bi constraints, the filter typically compares:
/// - A field from entity A (param 0) with the flattened item C (param 2)
///
/// Example filter: `Eq(field(0, 2), param(2))` means "shift.day == flattened_date"
fn extract_flattened_lookup_keys(filter_expr: &Expr) -> (Expr, Expr) {
    // Look for Eq comparisons involving Param(2) (the flattened item C)
    if let Expr::Eq(left, right) = filter_expr {
        // Check if either side is Param(2)
        if matches!(right.as_ref(), Expr::Param(2)) {
            let a_lookup = normalize_key_expr_to_param0((**left).clone());
            let c_key = Expr::Param(0);
            return (c_key, a_lookup);
        }
        if matches!(left.as_ref(), Expr::Param(2)) {
            let a_lookup = normalize_key_expr_to_param0((**right).clone());
            let c_key = Expr::Param(0);
            return (c_key, a_lookup);
        }
    }

    // For And expressions, recursively check both sides
    if let Expr::And(left, right) = filter_expr {
        let (c_key, a_lookup) = extract_flattened_lookup_keys(left);
        if !matches!(c_key, Expr::Param(0)) || !matches!(a_lookup, Expr::Literal(_)) {
            return (c_key, a_lookup);
        }
        return extract_flattened_lookup_keys(right);
    }

    // Default: use constant key (no optimization)
    (
        Expr::Literal(DynamicValue::I64(0)),
        Expr::Literal(DynamicValue::I64(0)),
    )
}
