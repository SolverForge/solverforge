//! Thread-local context for canonical entity deduplication during deserialization.
//!
//! When deserializing a solution from JSON, planning list variables contain copies
//! of entities rather than references to canonical entities from value range providers.
//! This causes WASM pointer mismatches that break Timefold's entity tracking.
//!
//! This module provides a thread-local registry that:
//! 1. Registers canonical entities (from value range providers) during Phase 1
//! 2. Allows lookup of canonical entities by (type, planning_id) during Phase 2
//!
//! The `EntityContextGuard` ensures proper cleanup via RAII.

use std::cell::RefCell;
use std::collections::HashMap;

use crate::Value;

thread_local! {
    static REGISTRY: RefCell<Option<HashMap<(String, String), Value>>> = const { RefCell::new(None) };
}

/// RAII guard that initializes and cleans up the entity context.
///
/// Create this at the start of deserialization; it will automatically
/// clean up the registry when dropped.
pub struct EntityContextGuard;

impl EntityContextGuard {
    /// Creates a new context guard, initializing the thread-local registry.
    pub fn new() -> Self {
        REGISTRY.with(|r| *r.borrow_mut() = Some(HashMap::new()));
        Self
    }
}

impl Default for EntityContextGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for EntityContextGuard {
    fn drop(&mut self) {
        REGISTRY.with(|r| *r.borrow_mut() = None);
    }
}

/// Registers a canonical entity in the thread-local registry.
///
/// Call this during Phase 1 (value range provider deserialization) to register
/// the canonical version of each entity.
///
/// # Arguments
/// * `type_name` - The entity type name (e.g., "Visit", "Employee")
/// * `planning_id` - The entity's planning ID value
/// * `entity` - The canonical entity Value to store
pub fn register_canonical(type_name: &str, planning_id: &Value, entity: Value) {
    REGISTRY.with(|r| {
        if let Some(ref mut map) = *r.borrow_mut() {
            map.insert((type_name.to_string(), id_to_string(planning_id)), entity);
        }
    });
}

/// Looks up a canonical entity by type and planning ID.
///
/// Call this during Phase 2 (entity collection deserialization) to resolve
/// planning list variable elements to their canonical versions.
///
/// # Arguments
/// * `type_name` - The entity type name to look up
/// * `planning_id` - The planning ID to look up
///
/// # Returns
/// The canonical entity Value if found, None otherwise
pub fn lookup_canonical(type_name: &str, planning_id: &Value) -> Option<Value> {
    REGISTRY.with(|r| {
        r.borrow().as_ref().and_then(|map| {
            map.get(&(type_name.to_string(), id_to_string(planning_id)))
                .cloned()
        })
    })
}

/// Converts a planning ID Value to a string key for the registry.
fn id_to_string(id: &Value) -> String {
    match id {
        Value::String(s) => s.clone(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Decimal(d) => d.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => format!("{:?}", id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_lookup() {
        let _guard = EntityContextGuard::new();

        let entity = Value::Object(
            [("id".to_string(), Value::String("e1".to_string()))]
                .into_iter()
                .collect(),
        );
        let id = Value::String("e1".to_string());

        register_canonical("TestEntity", &id, entity.clone());

        let found = lookup_canonical("TestEntity", &id);
        assert!(found.is_some());
        assert_eq!(found.unwrap(), entity);
    }

    #[test]
    fn test_lookup_without_context_returns_none() {
        let id = Value::String("e1".to_string());
        let found = lookup_canonical("TestEntity", &id);
        assert!(found.is_none());
    }

    #[test]
    fn test_lookup_wrong_type_returns_none() {
        let _guard = EntityContextGuard::new();

        let entity = Value::Object(
            [("id".to_string(), Value::String("e1".to_string()))]
                .into_iter()
                .collect(),
        );
        let id = Value::String("e1".to_string());

        register_canonical("TypeA", &id, entity);

        let found = lookup_canonical("TypeB", &id);
        assert!(found.is_none());
    }

    #[test]
    fn test_context_cleanup_on_drop() {
        let id = Value::String("e1".to_string());
        let entity = Value::Object(
            [("id".to_string(), Value::String("e1".to_string()))]
                .into_iter()
                .collect(),
        );

        {
            let _guard = EntityContextGuard::new();
            register_canonical("TestEntity", &id, entity);
            assert!(lookup_canonical("TestEntity", &id).is_some());
        }

        // After guard is dropped, lookup should return None
        assert!(lookup_canonical("TestEntity", &id).is_none());
    }

    #[test]
    fn test_int_id() {
        let _guard = EntityContextGuard::new();

        let entity = Value::Object([("id".to_string(), Value::Int(42))].into_iter().collect());
        let id = Value::Int(42);

        register_canonical("TestEntity", &id, entity.clone());

        let found = lookup_canonical("TestEntity", &id);
        assert!(found.is_some());
        assert_eq!(found.unwrap(), entity);
    }

    #[test]
    fn test_large_int_id() {
        let _guard = EntityContextGuard::new();

        let entity = Value::Object(
            [("id".to_string(), Value::Int(123456789))]
                .into_iter()
                .collect(),
        );
        let id = Value::Int(123456789);

        register_canonical("TestEntity", &id, entity.clone());

        let found = lookup_canonical("TestEntity", &id);
        assert!(found.is_some());
        assert_eq!(found.unwrap(), entity);
    }
}
