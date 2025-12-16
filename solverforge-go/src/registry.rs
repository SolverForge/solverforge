//! Handle registry for managing Go object references
//!
//! This module provides a registry that maps handle IDs to Go object reference IDs.
//! The actual Go objects are stored in Go's registry; we only store their reference IDs here.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Metadata about a Go object
#[derive(Debug, Clone)]
pub struct GoObjectRef {
    /// Reference ID in Go's registry
    pub go_ref_id: u64,
    /// Optional cached class name
    pub class_name: Option<String>,
}

impl GoObjectRef {
    /// Create a new GoObjectRef
    pub fn new(go_ref_id: u64) -> Self {
        Self {
            go_ref_id,
            class_name: None,
        }
    }

    /// Create a new GoObjectRef with a class name
    pub fn with_class(go_ref_id: u64, class_name: String) -> Self {
        Self {
            go_ref_id,
            class_name: Some(class_name),
        }
    }
}

/// Metadata about a Go function
#[derive(Debug, Clone)]
pub struct GoFunctionRef {
    /// Reference ID in Go's registry
    pub go_ref_id: u64,
}

impl GoFunctionRef {
    /// Create a new GoFunctionRef
    pub fn new(go_ref_id: u64) -> Self {
        Self { go_ref_id }
    }
}

/// Registry for managing object and function handles
#[derive(Debug)]
pub struct HandleRegistry {
    /// Map of ObjectHandle ID -> Go object reference
    objects: Arc<Mutex<HashMap<u64, GoObjectRef>>>,
    /// Map of FunctionHandle ID -> Go function reference
    functions: Arc<Mutex<HashMap<u64, GoFunctionRef>>>,
    /// Counter for generating unique handle IDs
    next_handle: Arc<Mutex<u64>>,
}

impl HandleRegistry {
    /// Create a new HandleRegistry
    pub fn new() -> Self {
        Self {
            objects: Arc::new(Mutex::new(HashMap::new())),
            functions: Arc::new(Mutex::new(HashMap::new())),
            next_handle: Arc::new(Mutex::new(1)),
        }
    }

    /// Generate the next unique handle ID
    pub fn next_id(&self) -> u64 {
        let mut next = self.next_handle.lock().unwrap();
        let id = *next;
        *next += 1;
        id
    }

    /// Register a Go object and return its handle ID
    pub fn register_object(&self, go_ref_id: u64) -> u64 {
        let handle_id = self.next_id();
        self.objects
            .lock()
            .unwrap()
            .insert(handle_id, GoObjectRef::new(go_ref_id));
        handle_id
    }

    /// Register a Go object with a class name and return its handle ID
    pub fn register_object_with_class(&self, go_ref_id: u64, class_name: String) -> u64 {
        let handle_id = self.next_id();
        self.objects
            .lock()
            .unwrap()
            .insert(handle_id, GoObjectRef::with_class(go_ref_id, class_name));
        handle_id
    }

    /// Get a Go object reference by handle ID
    pub fn get_object(&self, handle_id: u64) -> Option<GoObjectRef> {
        self.objects.lock().unwrap().get(&handle_id).cloned()
    }

    /// Release an object from the registry
    pub fn release_object(&self, handle_id: u64) -> Option<GoObjectRef> {
        self.objects.lock().unwrap().remove(&handle_id)
    }

    /// Register a Go function and return its handle ID
    pub fn register_function(&self, go_ref_id: u64) -> u64 {
        let handle_id = self.next_id();
        self.functions
            .lock()
            .unwrap()
            .insert(handle_id, GoFunctionRef::new(go_ref_id));
        handle_id
    }

    /// Get a Go function reference by handle ID
    pub fn get_function(&self, handle_id: u64) -> Option<GoFunctionRef> {
        self.functions.lock().unwrap().get(&handle_id).cloned()
    }

    /// Release a function from the registry
    pub fn release_function(&self, handle_id: u64) -> Option<GoFunctionRef> {
        self.functions.lock().unwrap().remove(&handle_id)
    }

    /// Get the number of registered objects
    pub fn object_count(&self) -> usize {
        self.objects.lock().unwrap().len()
    }

    /// Get the number of registered functions
    pub fn function_count(&self) -> usize {
        self.functions.lock().unwrap().len()
    }

    /// Clear all registrations
    pub fn clear(&self) {
        self.objects.lock().unwrap().clear();
        self.functions.lock().unwrap().clear();
    }
}

impl Default for HandleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Safety: HandleRegistry uses Arc<Mutex<...>> for interior mutability
unsafe impl Send for HandleRegistry {}
unsafe impl Sync for HandleRegistry {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_object_lifecycle() {
        let registry = HandleRegistry::new();

        // Register an object
        let handle_id = registry.register_object(100);
        assert_ne!(handle_id, 0);
        assert_eq!(registry.object_count(), 1);

        // Get the object
        let obj_ref = registry.get_object(handle_id).unwrap();
        assert_eq!(obj_ref.go_ref_id, 100);

        // Release the object
        let released = registry.release_object(handle_id).unwrap();
        assert_eq!(released.go_ref_id, 100);
        assert_eq!(registry.object_count(), 0);
    }

    #[test]
    fn test_registry_function_lifecycle() {
        let registry = HandleRegistry::new();

        // Register a function
        let handle_id = registry.register_function(200);
        assert_ne!(handle_id, 0);
        assert_eq!(registry.function_count(), 1);

        // Get the function
        let func_ref = registry.get_function(handle_id).unwrap();
        assert_eq!(func_ref.go_ref_id, 200);

        // Release the function
        let released = registry.release_function(handle_id).unwrap();
        assert_eq!(released.go_ref_id, 200);
        assert_eq!(registry.function_count(), 0);
    }

    #[test]
    fn test_registry_with_class_name() {
        let registry = HandleRegistry::new();

        let handle_id = registry.register_object_with_class(100, "TestClass".to_string());
        let obj_ref = registry.get_object(handle_id).unwrap();

        assert_eq!(obj_ref.go_ref_id, 100);
        assert_eq!(obj_ref.class_name.unwrap(), "TestClass");
    }

    #[test]
    fn test_registry_clear() {
        let registry = HandleRegistry::new();

        registry.register_object(100);
        registry.register_function(200);

        assert_eq!(registry.object_count(), 1);
        assert_eq!(registry.function_count(), 1);

        registry.clear();

        assert_eq!(registry.object_count(), 0);
        assert_eq!(registry.function_count(), 0);
    }
}
