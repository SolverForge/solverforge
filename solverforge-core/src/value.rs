//! Core value types for SolverForge
//!
//! This module defines the language-agnostic [`Value`] enum that represents
//! all possible values that can be passed between the core library and
//! language bindings.

use crate::handles::ObjectHandle;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Language-agnostic value representation
///
/// This enum can represent any value that needs to be passed between
/// the core library and language bindings. It maps to JSON types for
/// easy serialization when communicating with the solver service.
///
/// # Example
///
/// ```
/// use solverforge_core::Value;
///
/// let int_val = Value::from(42i64);
/// assert_eq!(int_val.as_int(), Some(42));
///
/// let str_val = Value::from("hello");
/// assert_eq!(str_val.as_str(), Some("hello"));
/// ```
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    /// Null/None value
    #[default]
    Null,
    /// Boolean value
    Bool(bool),
    /// Integer value (64-bit)
    Int(i64),
    /// Floating point value
    Float(f64),
    /// Decimal value for precise arithmetic (scores, etc.)
    #[serde(with = "decimal_serde")]
    Decimal(Decimal),
    /// String value
    String(String),
    /// Array/List of values
    Array(Vec<Value>),
    /// Object/Map of string keys to values
    Object(HashMap<String, Value>),
    /// Reference to a host language object
    #[serde(skip)]
    ObjectRef(ObjectHandle),
}

impl Value {
    /// Check if this value is null
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Try to get this value as a boolean
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Try to get this value as an integer
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// Try to get this value as a float
    ///
    /// This will also convert integers to floats.
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Int(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// Try to get this value as a decimal
    pub fn as_decimal(&self) -> Option<Decimal> {
        match self {
            Value::Decimal(d) => Some(*d),
            _ => None,
        }
    }

    /// Try to get this value as a string
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// Try to get this value as an array
    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match self {
            Value::Array(arr) => Some(arr),
            _ => None,
        }
    }

    /// Try to get this value as a mutable array
    pub fn as_array_mut(&mut self) -> Option<&mut Vec<Value>> {
        match self {
            Value::Array(arr) => Some(arr),
            _ => None,
        }
    }

    /// Try to get this value as an object/map
    pub fn as_object(&self) -> Option<&HashMap<String, Value>> {
        match self {
            Value::Object(obj) => Some(obj),
            _ => None,
        }
    }

    /// Try to get this value as a mutable object/map
    pub fn as_object_mut(&mut self) -> Option<&mut HashMap<String, Value>> {
        match self {
            Value::Object(obj) => Some(obj),
            _ => None,
        }
    }

    /// Try to get this value as an object handle
    pub fn as_object_ref(&self) -> Option<ObjectHandle> {
        match self {
            Value::ObjectRef(handle) => Some(*handle),
            _ => None,
        }
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Value::Int(i)
    }
}

impl From<i32> for Value {
    fn from(i: i32) -> Self {
        Value::Int(i as i64)
    }
}

impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Value::Float(f)
    }
}

impl From<f32> for Value {
    fn from(f: f32) -> Self {
        Value::Float(f as f64)
    }
}

impl From<Decimal> for Value {
    fn from(d: Decimal) -> Self {
        Value::Decimal(d)
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_string())
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(v: Vec<T>) -> Self {
        Value::Array(v.into_iter().map(Into::into).collect())
    }
}

impl From<ObjectHandle> for Value {
    fn from(handle: ObjectHandle) -> Self {
        Value::ObjectRef(handle)
    }
}

impl<K: Into<String>, V: Into<Value>> FromIterator<(K, V)> for Value {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        Value::Object(
            iter.into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        )
    }
}

impl From<serde_json::Value> for Value {
    fn from(json: serde_json::Value) -> Self {
        match json {
            serde_json::Value::Null => Value::Null,
            serde_json::Value::Bool(b) => Value::Bool(b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Value::Int(i)
                } else if let Some(f) = n.as_f64() {
                    Value::Float(f)
                } else {
                    Value::Null
                }
            }
            serde_json::Value::String(s) => Value::String(s),
            serde_json::Value::Array(arr) => {
                Value::Array(arr.into_iter().map(Value::from).collect())
            }
            serde_json::Value::Object(obj) => {
                Value::Object(obj.into_iter().map(|(k, v)| (k, Value::from(v))).collect())
            }
        }
    }
}

impl Value {
    /// Convert from a serde_json::Value
    pub fn from_json_value(json: serde_json::Value) -> Self {
        Self::from(json)
    }
}

pub mod decimal_serde {
    use rust_decimal::Decimal;
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(decimal: &Decimal, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&decimal.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Decimal, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_conversions() {
        assert_eq!(Value::from(true), Value::Bool(true));
        assert_eq!(Value::from(false), Value::Bool(false));
        assert_eq!(Value::from(42i64), Value::Int(42));
        assert_eq!(Value::from(42i32), Value::Int(42));
        assert_eq!(Value::from(3.14f64), Value::Float(3.14));
        assert_eq!(Value::from(3.14f32), Value::Float(3.14f32 as f64));
        assert_eq!(Value::from("hello"), Value::String("hello".to_string()));
        assert_eq!(
            Value::from("world".to_string()),
            Value::String("world".to_string())
        );
    }

    #[test]
    fn test_value_accessors() {
        assert_eq!(Value::Bool(true).as_bool(), Some(true));
        assert_eq!(Value::Bool(false).as_bool(), Some(false));
        assert_eq!(Value::Int(42).as_int(), Some(42));
        assert_eq!(Value::Float(3.14).as_float(), Some(3.14));
        assert_eq!(Value::Int(42).as_float(), Some(42.0)); // int to float
        assert_eq!(Value::String("hello".to_string()).as_str(), Some("hello"));
    }

    #[test]
    fn test_null_check() {
        assert!(Value::Null.is_null());
        assert!(!Value::Bool(false).is_null());
        assert!(!Value::Int(0).is_null());
    }

    #[test]
    fn test_default() {
        let val: Value = Default::default();
        assert!(val.is_null());
    }

    #[test]
    fn test_array_conversion() {
        let arr = Value::from(vec![1i64, 2, 3]);
        assert_eq!(
            arr.as_array(),
            Some(&vec![Value::Int(1), Value::Int(2), Value::Int(3)])
        );
    }

    #[test]
    fn test_object_from_iterator() {
        let obj: Value = vec![("key1", 42i64), ("key2", 99i64)].into_iter().collect();
        let map = obj.as_object().unwrap();
        assert_eq!(map.get("key1"), Some(&Value::Int(42)));
        assert_eq!(map.get("key2"), Some(&Value::Int(99)));
    }

    #[test]
    fn test_object_handle_conversion() {
        let handle = ObjectHandle::new(123);
        let val = Value::from(handle);
        assert_eq!(val.as_object_ref(), Some(ObjectHandle::new(123)));
    }

    #[test]
    fn test_decimal_conversion() {
        let d = Decimal::new(314, 2); // 3.14
        let val = Value::from(d);
        assert_eq!(val.as_decimal(), Some(Decimal::new(314, 2)));
    }

    #[test]
    fn test_json_serialization_primitives() {
        assert_eq!(serde_json::to_string(&Value::Null).unwrap(), "null");
        assert_eq!(serde_json::to_string(&Value::Bool(true)).unwrap(), "true");
        assert_eq!(serde_json::to_string(&Value::Int(42)).unwrap(), "42");
        assert_eq!(serde_json::to_string(&Value::Float(3.14)).unwrap(), "3.14");
        assert_eq!(
            serde_json::to_string(&Value::String("hello".to_string())).unwrap(),
            "\"hello\""
        );
    }

    #[test]
    fn test_json_serialization_array() {
        let arr = Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
        assert_eq!(serde_json::to_string(&arr).unwrap(), "[1,2,3]");
    }

    #[test]
    fn test_json_serialization_object() {
        let mut map = HashMap::new();
        map.insert("a".to_string(), Value::Int(1));
        let obj = Value::Object(map);
        assert_eq!(serde_json::to_string(&obj).unwrap(), "{\"a\":1}");
    }

    #[test]
    fn test_json_deserialization() {
        assert_eq!(serde_json::from_str::<Value>("null").unwrap(), Value::Null);
        assert_eq!(
            serde_json::from_str::<Value>("true").unwrap(),
            Value::Bool(true)
        );
        assert_eq!(serde_json::from_str::<Value>("42").unwrap(), Value::Int(42));
        assert_eq!(
            serde_json::from_str::<Value>("\"hello\"").unwrap(),
            Value::String("hello".to_string())
        );
    }

    #[test]
    fn test_mutable_accessors() {
        let mut arr = Value::Array(vec![Value::Int(1)]);
        arr.as_array_mut().unwrap().push(Value::Int(2));
        assert_eq!(arr.as_array().unwrap().len(), 2);

        let mut obj: Value = vec![("key", 1i64)].into_iter().collect();
        obj.as_object_mut()
            .unwrap()
            .insert("key2".to_string(), Value::Int(2));
        assert_eq!(obj.as_object().unwrap().len(), 2);
    }

    #[test]
    fn test_accessor_returns_none_for_wrong_type() {
        let val = Value::Int(42);
        assert_eq!(val.as_bool(), None);
        assert_eq!(val.as_str(), None);
        assert_eq!(val.as_array(), None);
        assert_eq!(val.as_object(), None);
        assert_eq!(val.as_object_ref(), None);
    }
}
