//! Single-entity expression evaluation without tuple context.

use super::compare::{compare_values, values_equal};
use crate::expr::Expr;
use crate::solution::{DynamicEntity, DynamicSolution, DynamicValue};

/// Evaluates an expression against a single entity without tuple context.
///
/// This function is used when building closures for constraint evaluation where
/// only a single entity is in scope. `Expr::Param(0)` refers to the entity itself,
/// and `Expr::Field { param_idx: 0, field_idx }` accesses fields directly from the entity.
///
/// # Arguments
/// * `expr` - The expression to evaluate
/// * `solution` - The solution context (for fact lookups and references)
/// * `entity` - The single entity being evaluated
///
/// # Returns
/// The evaluated `DynamicValue` result.
pub fn eval_entity_expr(
    expr: &Expr,
    solution: &DynamicSolution,
    entity: &DynamicEntity,
) -> DynamicValue {
    match expr {
        Expr::Literal(v) => v.clone(),

        Expr::Param(idx) => {
            // Param(0) refers to the entity itself as a reference
            if *idx == 0 {
                // Return entity id as a reference (we don't have class_idx here, use 0 as placeholder)
                DynamicValue::I64(entity.id)
            } else {
                DynamicValue::None
            }
        }

        Expr::Field {
            param_idx,
            field_idx,
        } => {
            if *param_idx == 0 {
                entity
                    .fields
                    .get(*field_idx)
                    .cloned()
                    .unwrap_or(DynamicValue::None)
            } else {
                DynamicValue::None
            }
        }

        Expr::Eq(left, right) => {
            let l = eval_entity_expr(left, solution, entity);
            let r = eval_entity_expr(right, solution, entity);
            DynamicValue::Bool(values_equal(&l, &r))
        }

        Expr::Ne(left, right) => {
            let l = eval_entity_expr(left, solution, entity);
            let r = eval_entity_expr(right, solution, entity);
            DynamicValue::Bool(!values_equal(&l, &r))
        }

        Expr::Lt(left, right) => {
            let l = eval_entity_expr(left, solution, entity);
            let r = eval_entity_expr(right, solution, entity);
            DynamicValue::Bool(compare_values(&l, &r).map(|o| o.is_lt()).unwrap_or(false))
        }

        Expr::Le(left, right) => {
            let l = eval_entity_expr(left, solution, entity);
            let r = eval_entity_expr(right, solution, entity);
            DynamicValue::Bool(compare_values(&l, &r).map(|o| o.is_le()).unwrap_or(false))
        }

        Expr::Gt(left, right) => {
            let l = eval_entity_expr(left, solution, entity);
            let r = eval_entity_expr(right, solution, entity);
            DynamicValue::Bool(compare_values(&l, &r).map(|o| o.is_gt()).unwrap_or(false))
        }

        Expr::Ge(left, right) => {
            let l = eval_entity_expr(left, solution, entity);
            let r = eval_entity_expr(right, solution, entity);
            DynamicValue::Bool(compare_values(&l, &r).map(|o| o.is_ge()).unwrap_or(false))
        }

        Expr::And(left, right) => {
            let l = eval_entity_expr(left, solution, entity);
            let r = eval_entity_expr(right, solution, entity);
            match (l.as_bool(), r.as_bool()) {
                (Some(a), Some(b)) => DynamicValue::Bool(a && b),
                _ => DynamicValue::Bool(false),
            }
        }

        Expr::Or(left, right) => {
            let l = eval_entity_expr(left, solution, entity);
            let r = eval_entity_expr(right, solution, entity);
            match (l.as_bool(), r.as_bool()) {
                (Some(a), Some(b)) => DynamicValue::Bool(a || b),
                _ => DynamicValue::Bool(false),
            }
        }

        Expr::Not(inner) => {
            let v = eval_entity_expr(inner, solution, entity);
            match v.as_bool() {
                Some(b) => DynamicValue::Bool(!b),
                None => DynamicValue::Bool(true),
            }
        }

        Expr::Abs(inner) => {
            let v = eval_entity_expr(inner, solution, entity);
            match v {
                DynamicValue::I64(n) => DynamicValue::I64(n.abs()),
                DynamicValue::F64(n) => DynamicValue::F64(n.abs()),
                _ => DynamicValue::None,
            }
        }

        Expr::Add(left, right) => {
            let l = eval_entity_expr(left, solution, entity);
            let r = eval_entity_expr(right, solution, entity);
            match (&l, &r) {
                (DynamicValue::I64(a), DynamicValue::I64(b)) => DynamicValue::I64(a + b),
                (DynamicValue::F64(a), DynamicValue::F64(b)) => DynamicValue::F64(a + b),
                (DynamicValue::I64(a), DynamicValue::F64(b)) => DynamicValue::F64(*a as f64 + b),
                (DynamicValue::F64(a), DynamicValue::I64(b)) => DynamicValue::F64(a + *b as f64),
                _ => DynamicValue::None,
            }
        }

        Expr::Sub(left, right) => {
            let l = eval_entity_expr(left, solution, entity);
            let r = eval_entity_expr(right, solution, entity);
            match (&l, &r) {
                (DynamicValue::I64(a), DynamicValue::I64(b)) => DynamicValue::I64(a - b),
                (DynamicValue::F64(a), DynamicValue::F64(b)) => DynamicValue::F64(a - b),
                (DynamicValue::I64(a), DynamicValue::F64(b)) => DynamicValue::F64(*a as f64 - b),
                (DynamicValue::F64(a), DynamicValue::I64(b)) => DynamicValue::F64(a - *b as f64),
                _ => DynamicValue::None,
            }
        }

        Expr::Mul(left, right) => {
            let l = eval_entity_expr(left, solution, entity);
            let r = eval_entity_expr(right, solution, entity);
            match (&l, &r) {
                (DynamicValue::I64(a), DynamicValue::I64(b)) => DynamicValue::I64(a * b),
                (DynamicValue::F64(a), DynamicValue::F64(b)) => DynamicValue::F64(a * b),
                (DynamicValue::I64(a), DynamicValue::F64(b)) => DynamicValue::F64(*a as f64 * b),
                (DynamicValue::F64(a), DynamicValue::I64(b)) => DynamicValue::F64(a * *b as f64),
                _ => DynamicValue::None,
            }
        }

        Expr::Div(left, right) => {
            let l = eval_entity_expr(left, solution, entity);
            let r = eval_entity_expr(right, solution, entity);
            match (&l, &r) {
                (DynamicValue::I64(a), DynamicValue::I64(b)) if *b != 0 => DynamicValue::I64(a / b),
                (DynamicValue::F64(a), DynamicValue::F64(b)) if *b != 0.0 => {
                    DynamicValue::F64(a / b)
                }
                _ => DynamicValue::None,
            }
        }

        Expr::Mod(left, right) => {
            let l = eval_entity_expr(left, solution, entity);
            let r = eval_entity_expr(right, solution, entity);
            match (&l, &r) {
                (DynamicValue::I64(a), DynamicValue::I64(b)) if *b != 0 => DynamicValue::I64(a % b),
                _ => DynamicValue::None,
            }
        }

        Expr::Neg(inner) => {
            let v = eval_entity_expr(inner, solution, entity);
            match v {
                DynamicValue::I64(n) => DynamicValue::I64(-n),
                DynamicValue::F64(n) => DynamicValue::F64(-n),
                _ => DynamicValue::None,
            }
        }

        Expr::Contains(list, elem) => {
            let list_val = eval_entity_expr(list, solution, entity);
            let elem_val = eval_entity_expr(elem, solution, entity);
            match list_val {
                DynamicValue::List(items) => {
                    DynamicValue::Bool(items.iter().any(|item| values_equal(item, &elem_val)))
                }
                _ => DynamicValue::Bool(false),
            }
        }

        Expr::If {
            cond,
            then_expr,
            else_expr,
        } => {
            let cond_val = eval_entity_expr(cond, solution, entity);
            if cond_val.as_bool().unwrap_or(false) {
                eval_entity_expr(then_expr, solution, entity)
            } else {
                eval_entity_expr(else_expr, solution, entity)
            }
        }

        Expr::RefField {
            ref_expr,
            field_idx,
        } => {
            let ref_val = eval_entity_expr(ref_expr, solution, entity);
            match ref_val {
                DynamicValue::Ref(class_idx, entity_idx) => {
                    if let Some(entity) = solution.get_entity(class_idx, entity_idx) {
                        entity
                            .fields
                            .get(*field_idx)
                            .cloned()
                            .unwrap_or(DynamicValue::None)
                    } else {
                        DynamicValue::None
                    }
                }
                DynamicValue::FactRef(class_idx, fact_idx) => {
                    if let Some(fact) = solution.get_fact(class_idx, fact_idx) {
                        fact.fields
                            .get(*field_idx)
                            .cloned()
                            .unwrap_or(DynamicValue::None)
                    } else {
                        DynamicValue::None
                    }
                }
                _ => DynamicValue::None,
            }
        }

        Expr::IsNotNone(inner) => {
            let v = eval_entity_expr(inner, solution, entity);
            DynamicValue::Bool(!v.is_none())
        }

        Expr::IsNone(inner) => {
            let v = eval_entity_expr(inner, solution, entity);
            DynamicValue::Bool(v.is_none())
        }

        Expr::SetContains {
            set_expr,
            value_expr,
        } => {
            let set_val = eval_entity_expr(set_expr, solution, entity);
            let value = eval_entity_expr(value_expr, solution, entity);
            DynamicValue::Bool(set_val.contains(&value))
        }

        Expr::DateOf(inner) => {
            let v = eval_entity_expr(inner, solution, entity);
            match v {
                DynamicValue::DateTime(ms) => {
                    let days = ms / (1000 * 60 * 60 * 24);
                    let days_i32 =
                        i32::try_from(days).expect("DateTime days overflow i32 in DateOf");
                    DynamicValue::Date(days_i32)
                }
                DynamicValue::I64(ms) => {
                    let days = ms / (1000 * 60 * 60 * 24);
                    let days_i32 = i32::try_from(days).expect("I64 days overflow i32 in DateOf");
                    DynamicValue::Date(days_i32)
                }
                DynamicValue::Date(d) => DynamicValue::Date(d),
                _ => DynamicValue::None,
            }
        }

        Expr::Overlaps {
            start1,
            end1,
            start2,
            end2,
        } => {
            let s1 = eval_entity_expr(start1, solution, entity).as_datetime();
            let e1 = eval_entity_expr(end1, solution, entity).as_datetime();
            let s2 = eval_entity_expr(start2, solution, entity).as_datetime();
            let e2 = eval_entity_expr(end2, solution, entity).as_datetime();

            match (s1, e1, s2, e2) {
                (Some(s1), Some(e1), Some(s2), Some(e2)) => {
                    DynamicValue::Bool(s1.max(s2) < e1.min(e2))
                }
                _ => DynamicValue::Bool(false),
            }
        }

        Expr::OverlapMinutes {
            start1,
            end1,
            start2,
            end2,
        } => {
            let s1 = eval_entity_expr(start1, solution, entity).as_datetime();
            let e1 = eval_entity_expr(end1, solution, entity).as_datetime();
            let s2 = eval_entity_expr(start2, solution, entity).as_datetime();
            let e2 = eval_entity_expr(end2, solution, entity).as_datetime();

            match (s1, e1, s2, e2) {
                (Some(s1), Some(e1), Some(s2), Some(e2)) => {
                    let overlap_start = s1.max(s2);
                    let overlap_end = e1.min(e2);
                    if overlap_start < overlap_end {
                        let minutes = (overlap_end - overlap_start) / (1000 * 60);
                        DynamicValue::I64(minutes)
                    } else {
                        DynamicValue::I64(0)
                    }
                }
                _ => DynamicValue::I64(0),
            }
        }

        Expr::OverlapsDate { start, end, date } => {
            let s = eval_entity_expr(start, solution, entity).as_datetime();
            let e = eval_entity_expr(end, solution, entity).as_datetime();
            let d = eval_entity_expr(date, solution, entity).as_date();

            match (s, e, d) {
                (Some(start_ms), Some(end_ms), Some(date_days)) => {
                    let date_start_ms = date_days as i64 * 24 * 60 * 60 * 1000;
                    let date_end_ms = date_start_ms + 24 * 60 * 60 * 1000;
                    DynamicValue::Bool(start_ms.max(date_start_ms) < end_ms.min(date_end_ms))
                }
                _ => DynamicValue::Bool(false),
            }
        }

        Expr::OverlapDateMinutes { start, end, date } => {
            let s = eval_entity_expr(start, solution, entity).as_datetime();
            let e = eval_entity_expr(end, solution, entity).as_datetime();
            let d = eval_entity_expr(date, solution, entity).as_date();

            match (s, e, d) {
                (Some(start_ms), Some(end_ms), Some(date_days)) => {
                    let date_start_ms = date_days as i64 * 24 * 60 * 60 * 1000;
                    let date_end_ms = date_start_ms + 24 * 60 * 60 * 1000;
                    let overlap_start = start_ms.max(date_start_ms);
                    let overlap_end = end_ms.min(date_end_ms);
                    if overlap_start < overlap_end {
                        let minutes = (overlap_end - overlap_start) / (1000 * 60);
                        DynamicValue::I64(minutes)
                    } else {
                        DynamicValue::I64(0)
                    }
                }
                _ => DynamicValue::I64(0),
            }
        }

        Expr::Min(left, right) => {
            let l = eval_entity_expr(left, solution, entity);
            let r = eval_entity_expr(right, solution, entity);
            match (&l, &r) {
                (DynamicValue::I64(a), DynamicValue::I64(b)) => DynamicValue::I64(*a.min(b)),
                (DynamicValue::F64(a), DynamicValue::F64(b)) => DynamicValue::F64(a.min(*b)),
                (DynamicValue::DateTime(a), DynamicValue::DateTime(b)) => {
                    DynamicValue::DateTime(*a.min(b))
                }
                _ => l,
            }
        }

        Expr::Max(left, right) => {
            let l = eval_entity_expr(left, solution, entity);
            let r = eval_entity_expr(right, solution, entity);
            match (&l, &r) {
                (DynamicValue::I64(a), DynamicValue::I64(b)) => DynamicValue::I64(*a.max(b)),
                (DynamicValue::F64(a), DynamicValue::F64(b)) => DynamicValue::F64(a.max(*b)),
                (DynamicValue::DateTime(a), DynamicValue::DateTime(b)) => {
                    DynamicValue::DateTime(*a.max(b))
                }
                _ => l,
            }
        }

        Expr::FlattenedValue => DynamicValue::None, // Not supported in single-entity context
    }
}
