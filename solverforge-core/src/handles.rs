//! Handle types for language-specific object and function references
//!
//! This module defines opaque handle types that allow the core library to
//! reference objects and functions in the host language without knowing
//! their concrete types.

use serde::{Deserialize, Serialize};

/// Opaque handle to a language-specific object
///
/// This is used to reference objects in the host language without
/// needing to know their concrete type in the core library. The handle
/// is typically an index into a registry maintained by the language bridge.
///
/// # Example
///
/// ```
/// use solverforge_core::ObjectHandle;
///
/// let handle = ObjectHandle::new(42);
/// assert_eq!(handle.id(), 42);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ObjectHandle(pub u64);

impl ObjectHandle {
    /// Create a new object handle from a raw identifier
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw identifier
    pub fn id(&self) -> u64 {
        self.0
    }
}

impl From<u64> for ObjectHandle {
    fn from(id: u64) -> Self {
        Self::new(id)
    }
}

/// Opaque handle to a language-specific callable (function/lambda)
///
/// This is used to reference functions or lambdas in the host language
/// that can be called from the core library. Like [`ObjectHandle`], this
/// is typically an index into a registry maintained by the language bridge.
///
/// # Example
///
/// ```
/// use solverforge_core::FunctionHandle;
///
/// let handle = FunctionHandle::new(1);
/// assert_eq!(handle.id(), 1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FunctionHandle(pub u64);

impl FunctionHandle {
    /// Create a new function handle from a raw identifier
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw identifier
    pub fn id(&self) -> u64 {
        self.0
    }
}

impl From<u64> for FunctionHandle {
    fn from(id: u64) -> Self {
        Self::new(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_object_handle_creation() {
        let handle = ObjectHandle::new(42);
        assert_eq!(handle.id(), 42);
        assert_eq!(handle.0, 42);
    }

    #[test]
    fn test_object_handle_equality() {
        let h1 = ObjectHandle::new(1);
        let h2 = ObjectHandle::new(1);
        let h3 = ObjectHandle::new(2);

        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
    }

    #[test]
    fn test_object_handle_hash() {
        let mut set = HashSet::new();
        set.insert(ObjectHandle::new(1));
        set.insert(ObjectHandle::new(2));
        set.insert(ObjectHandle::new(1)); // duplicate

        assert_eq!(set.len(), 2);
        assert!(set.contains(&ObjectHandle::new(1)));
        assert!(set.contains(&ObjectHandle::new(2)));
    }

    #[test]
    fn test_object_handle_from_u64() {
        let handle: ObjectHandle = 123u64.into();
        assert_eq!(handle.id(), 123);
    }

    #[test]
    fn test_function_handle_creation() {
        let handle = FunctionHandle::new(42);
        assert_eq!(handle.id(), 42);
        assert_eq!(handle.0, 42);
    }

    #[test]
    fn test_function_handle_equality() {
        let h1 = FunctionHandle::new(1);
        let h2 = FunctionHandle::new(1);
        let h3 = FunctionHandle::new(2);

        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
    }

    #[test]
    fn test_function_handle_hash() {
        let mut set = HashSet::new();
        set.insert(FunctionHandle::new(1));
        set.insert(FunctionHandle::new(2));
        set.insert(FunctionHandle::new(1)); // duplicate

        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_function_handle_from_u64() {
        let handle: FunctionHandle = 456u64.into();
        assert_eq!(handle.id(), 456);
    }

    #[test]
    fn test_handle_copy() {
        let h1 = ObjectHandle::new(1);
        let h2 = h1; // Copy, not move
        assert_eq!(h1, h2); // h1 still valid

        let f1 = FunctionHandle::new(1);
        let f2 = f1;
        assert_eq!(f1, f2);
    }

    #[test]
    fn test_handle_debug() {
        let obj = ObjectHandle::new(42);
        let func = FunctionHandle::new(99);

        assert_eq!(format!("{:?}", obj), "ObjectHandle(42)");
        assert_eq!(format!("{:?}", func), "FunctionHandle(99)");
    }
}
