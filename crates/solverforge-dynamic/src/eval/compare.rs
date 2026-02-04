//! Value comparison functions for dynamic values.

use crate::solution::DynamicValue;

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
