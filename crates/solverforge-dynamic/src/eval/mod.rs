//! Expression evaluation for runtime constraint checking.

#[cfg(test)]
mod tests;

use crate::expr::Expr;
use crate::solution::{DynamicFact, DynamicSolution, DynamicValue};

/// Reference to an entity in the tuple being evaluated.
#[derive(Debug, Clone, Copy)]
pub struct EntityRef {
    /// Class index.
    pub class_idx: usize,
    /// Entity index within the class.
    pub entity_idx: usize,
}

impl EntityRef {
    /// Creates a new entity reference.
    pub fn new(class_idx: usize, entity_idx: usize) -> Self {
        Self {
            class_idx,
            entity_idx,
        }
    }
}

/// Context for expression evaluation.
pub struct EvalContext<'a> {
    /// The solution being evaluated.
    pub solution: &'a DynamicSolution,
    /// The tuple of entity references being matched.
    pub tuple: &'a [EntityRef],
    /// Optional flattened value from FlattenLast operation.
    pub flattened_value: Option<&'a DynamicValue>,
}

impl<'a> EvalContext<'a> {
    /// Creates a new evaluation context.
    pub fn new(solution: &'a DynamicSolution, tuple: &'a [EntityRef]) -> Self {
        Self {
            solution,
            tuple,
            flattened_value: None,
        }
    }

    /// Gets an entity from the tuple by parameter index.
    pub fn get_entity(&self, param_idx: usize) -> Option<&'a crate::solution::DynamicEntity> {
        let entity_ref = self.tuple.get(param_idx)?;
        self.solution
            .get_entity(entity_ref.class_idx, entity_ref.entity_idx)
    }

    /// Gets a fact by class index and fact index.
    pub fn get_fact(&self, class_idx: usize, fact_idx: usize) -> Option<&'a DynamicFact> {
        self.solution.get_fact(class_idx, fact_idx)
    }
}

/// Evaluates an expression in the given context.
pub fn eval_expr(expr: &Expr, ctx: &EvalContext) -> DynamicValue {
    match expr {
        Expr::Literal(v) => v.clone(),

        Expr::Param(idx) => {
            if let Some(entity_ref) = ctx.tuple.get(*idx) {
                DynamicValue::Ref(entity_ref.class_idx, entity_ref.entity_idx)
            } else {
                DynamicValue::None
            }
        }

        Expr::Field {
            param_idx,
            field_idx,
        } => {
            if let Some(entity) = ctx.get_entity(*param_idx) {
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
            let l = eval_expr(left, ctx);
            let r = eval_expr(right, ctx);
            DynamicValue::Bool(values_equal(&l, &r))
        }

        Expr::Ne(left, right) => {
            let l = eval_expr(left, ctx);
            let r = eval_expr(right, ctx);
            DynamicValue::Bool(!values_equal(&l, &r))
        }

        Expr::Lt(left, right) => {
            let l = eval_expr(left, ctx);
            let r = eval_expr(right, ctx);
            DynamicValue::Bool(compare_values(&l, &r).map(|o| o.is_lt()).unwrap_or(false))
        }

        Expr::Le(left, right) => {
            let l = eval_expr(left, ctx);
            let r = eval_expr(right, ctx);
            DynamicValue::Bool(compare_values(&l, &r).map(|o| o.is_le()).unwrap_or(false))
        }

        Expr::Gt(left, right) => {
            let l = eval_expr(left, ctx);
            let r = eval_expr(right, ctx);
            DynamicValue::Bool(compare_values(&l, &r).map(|o| o.is_gt()).unwrap_or(false))
        }

        Expr::Ge(left, right) => {
            let l = eval_expr(left, ctx);
            let r = eval_expr(right, ctx);
            DynamicValue::Bool(compare_values(&l, &r).map(|o| o.is_ge()).unwrap_or(false))
        }

        Expr::And(left, right) => {
            let l = eval_expr(left, ctx);
            let r = eval_expr(right, ctx);
            match (l.as_bool(), r.as_bool()) {
                (Some(a), Some(b)) => DynamicValue::Bool(a && b),
                _ => DynamicValue::Bool(false),
            }
        }

        Expr::Or(left, right) => {
            let l = eval_expr(left, ctx);
            let r = eval_expr(right, ctx);
            match (l.as_bool(), r.as_bool()) {
                (Some(a), Some(b)) => DynamicValue::Bool(a || b),
                _ => DynamicValue::Bool(false),
            }
        }

        Expr::Not(inner) => {
            let v = eval_expr(inner, ctx);
            match v.as_bool() {
                Some(b) => DynamicValue::Bool(!b),
                None => DynamicValue::Bool(true), // None is considered false
            }
        }

        Expr::Abs(inner) => {
            let v = eval_expr(inner, ctx);
            match v {
                DynamicValue::I64(n) => DynamicValue::I64(n.abs()),
                DynamicValue::F64(n) => DynamicValue::F64(n.abs()),
                _ => DynamicValue::None,
            }
        }

        Expr::Add(left, right) => {
            let l = eval_expr(left, ctx);
            let r = eval_expr(right, ctx);
            match (&l, &r) {
                (DynamicValue::I64(a), DynamicValue::I64(b)) => DynamicValue::I64(a + b),
                (DynamicValue::F64(a), DynamicValue::F64(b)) => DynamicValue::F64(a + b),
                (DynamicValue::I64(a), DynamicValue::F64(b)) => DynamicValue::F64(*a as f64 + b),
                (DynamicValue::F64(a), DynamicValue::I64(b)) => DynamicValue::F64(a + *b as f64),
                _ => DynamicValue::None,
            }
        }

        Expr::Sub(left, right) => {
            let l = eval_expr(left, ctx);
            let r = eval_expr(right, ctx);
            match (&l, &r) {
                (DynamicValue::I64(a), DynamicValue::I64(b)) => DynamicValue::I64(a - b),
                (DynamicValue::F64(a), DynamicValue::F64(b)) => DynamicValue::F64(a - b),
                (DynamicValue::I64(a), DynamicValue::F64(b)) => DynamicValue::F64(*a as f64 - b),
                (DynamicValue::F64(a), DynamicValue::I64(b)) => DynamicValue::F64(a - *b as f64),
                _ => DynamicValue::None,
            }
        }

        Expr::Mul(left, right) => {
            let l = eval_expr(left, ctx);
            let r = eval_expr(right, ctx);
            match (&l, &r) {
                (DynamicValue::I64(a), DynamicValue::I64(b)) => DynamicValue::I64(a * b),
                (DynamicValue::F64(a), DynamicValue::F64(b)) => DynamicValue::F64(a * b),
                (DynamicValue::I64(a), DynamicValue::F64(b)) => DynamicValue::F64(*a as f64 * b),
                (DynamicValue::F64(a), DynamicValue::I64(b)) => DynamicValue::F64(a * *b as f64),
                _ => DynamicValue::None,
            }
        }

        Expr::Div(left, right) => {
            let l = eval_expr(left, ctx);
            let r = eval_expr(right, ctx);
            match (&l, &r) {
                (DynamicValue::I64(a), DynamicValue::I64(b)) if *b != 0 => DynamicValue::I64(a / b),
                (DynamicValue::F64(a), DynamicValue::F64(b)) if *b != 0.0 => {
                    DynamicValue::F64(a / b)
                }
                _ => DynamicValue::None,
            }
        }

        Expr::Mod(left, right) => {
            let l = eval_expr(left, ctx);
            let r = eval_expr(right, ctx);
            match (&l, &r) {
                (DynamicValue::I64(a), DynamicValue::I64(b)) if *b != 0 => DynamicValue::I64(a % b),
                _ => DynamicValue::None,
            }
        }

        Expr::Neg(inner) => {
            let v = eval_expr(inner, ctx);
            match v {
                DynamicValue::I64(n) => DynamicValue::I64(-n),
                DynamicValue::F64(n) => DynamicValue::F64(-n),
                _ => DynamicValue::None,
            }
        }

        Expr::Contains(list, elem) => {
            let list_val = eval_expr(list, ctx);
            let elem_val = eval_expr(elem, ctx);
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
            let cond_val = eval_expr(cond, ctx);
            if cond_val.as_bool().unwrap_or(false) {
                eval_expr(then_expr, ctx)
            } else {
                eval_expr(else_expr, ctx)
            }
        }

        Expr::RefField {
            ref_expr,
            field_idx,
        } => {
            let ref_val = eval_expr(ref_expr, ctx);
            match ref_val {
                DynamicValue::Ref(class_idx, entity_idx) => {
                    if let Some(entity) = ctx.solution.get_entity(class_idx, entity_idx) {
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
                    if let Some(fact) = ctx.get_fact(class_idx, fact_idx) {
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
            let v = eval_expr(inner, ctx);
            DynamicValue::Bool(!v.is_none())
        }

        Expr::IsNone(inner) => {
            let v = eval_expr(inner, ctx);
            DynamicValue::Bool(v.is_none())
        }

        Expr::SetContains {
            set_expr,
            value_expr,
        } => {
            let set_val = eval_expr(set_expr, ctx);
            let value = eval_expr(value_expr, ctx);
            DynamicValue::Bool(set_val.contains(&value))
        }

        Expr::DateOf(inner) => {
            let v = eval_expr(inner, ctx);
            match v {
                DynamicValue::DateTime(ms) => {
                    // Convert milliseconds since epoch to days since epoch
                    let days = (ms / (1000 * 60 * 60 * 24)) as i32;
                    DynamicValue::Date(days)
                }
                DynamicValue::I64(ms) => {
                    let days = (ms / (1000 * 60 * 60 * 24)) as i32;
                    DynamicValue::Date(days)
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
            let s1 = eval_expr(start1, ctx).as_datetime();
            let e1 = eval_expr(end1, ctx).as_datetime();
            let s2 = eval_expr(start2, ctx).as_datetime();
            let e2 = eval_expr(end2, ctx).as_datetime();

            match (s1, e1, s2, e2) {
                (Some(s1), Some(e1), Some(s2), Some(e2)) => {
                    // Ranges overlap if max(start1, start2) < min(end1, end2)
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
            let s1 = eval_expr(start1, ctx).as_datetime();
            let e1 = eval_expr(end1, ctx).as_datetime();
            let s2 = eval_expr(start2, ctx).as_datetime();
            let e2 = eval_expr(end2, ctx).as_datetime();

            match (s1, e1, s2, e2) {
                (Some(s1), Some(e1), Some(s2), Some(e2)) => {
                    let overlap_start = s1.max(s2);
                    let overlap_end = e1.min(e2);
                    if overlap_start < overlap_end {
                        // Convert milliseconds to minutes
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
            let s = eval_expr(start, ctx).as_datetime();
            let e = eval_expr(end, ctx).as_datetime();
            let d = eval_expr(date, ctx).as_date();

            match (s, e, d) {
                (Some(start_ms), Some(end_ms), Some(date_days)) => {
                    // Convert date to start/end of day in milliseconds
                    let date_start_ms = date_days as i64 * 24 * 60 * 60 * 1000;
                    let date_end_ms = date_start_ms + 24 * 60 * 60 * 1000;
                    // Check overlap
                    DynamicValue::Bool(start_ms.max(date_start_ms) < end_ms.min(date_end_ms))
                }
                _ => DynamicValue::Bool(false),
            }
        }

        Expr::OverlapDateMinutes { start, end, date } => {
            let s = eval_expr(start, ctx).as_datetime();
            let e = eval_expr(end, ctx).as_datetime();
            let d = eval_expr(date, ctx).as_date();

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
            let l = eval_expr(left, ctx);
            let r = eval_expr(right, ctx);
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
            let l = eval_expr(left, ctx);
            let r = eval_expr(right, ctx);
            match (&l, &r) {
                (DynamicValue::I64(a), DynamicValue::I64(b)) => DynamicValue::I64(*a.max(b)),
                (DynamicValue::F64(a), DynamicValue::F64(b)) => DynamicValue::F64(a.max(*b)),
                (DynamicValue::DateTime(a), DynamicValue::DateTime(b)) => {
                    DynamicValue::DateTime(*a.max(b))
                }
                _ => l,
            }
        }

        Expr::FlattenedValue => ctx.flattened_value.cloned().unwrap_or(DynamicValue::None),
    }
}

/// Checks if two dynamic values are equal.
pub fn values_equal(a: &DynamicValue, b: &DynamicValue) -> bool {
    match (a, b) {
        (DynamicValue::None, DynamicValue::None) => true,
        (DynamicValue::I64(x), DynamicValue::I64(y)) => x == y,
        (DynamicValue::F64(x), DynamicValue::F64(y)) => (x - y).abs() < f64::EPSILON,
        (DynamicValue::String(x), DynamicValue::String(y)) => x == y,
        (DynamicValue::Bool(x), DynamicValue::Bool(y)) => x == y,
        (DynamicValue::Ref(c1, e1), DynamicValue::Ref(c2, e2)) => c1 == c2 && e1 == e2,
        (DynamicValue::FactRef(c1, f1), DynamicValue::FactRef(c2, f2)) => c1 == c2 && f1 == f2,
        (DynamicValue::DateTime(x), DynamicValue::DateTime(y)) => x == y,
        (DynamicValue::Date(x), DynamicValue::Date(y)) => x == y,
        // Mixed numeric comparison
        (DynamicValue::I64(x), DynamicValue::F64(y)) => (*x as f64 - y).abs() < f64::EPSILON,
        (DynamicValue::F64(x), DynamicValue::I64(y)) => (x - *y as f64).abs() < f64::EPSILON,
        _ => false,
    }
}

/// Compares two dynamic values.
pub fn compare_values(a: &DynamicValue, b: &DynamicValue) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (DynamicValue::I64(x), DynamicValue::I64(y)) => Some(x.cmp(y)),
        (DynamicValue::F64(x), DynamicValue::F64(y)) => x.partial_cmp(y),
        (DynamicValue::I64(x), DynamicValue::F64(y)) => (*x as f64).partial_cmp(y),
        (DynamicValue::F64(x), DynamicValue::I64(y)) => x.partial_cmp(&(*y as f64)),
        (DynamicValue::String(x), DynamicValue::String(y)) => Some(x.cmp(y)),
        (DynamicValue::Bool(x), DynamicValue::Bool(y)) => Some(x.cmp(y)),
        (DynamicValue::DateTime(x), DynamicValue::DateTime(y)) => Some(x.cmp(y)),
        (DynamicValue::Date(x), DynamicValue::Date(y)) => Some(x.cmp(y)),
        _ => None,
    }
}
