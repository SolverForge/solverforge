//! GoBridge implementation of LanguageBridge
//!
//! This module provides the GoBridge struct that implements the LanguageBridge
//! trait for Go, enabling the core solver to interact with Go objects.

use crate::registry::{GoFunctionRef, GoObjectRef, HandleRegistry};
use serde::{Deserialize, Serialize};
use solverforge_core::domain::{FieldType, PlanningAnnotation};
use solverforge_core::{
    ClassInfo, FieldInfo, FunctionHandle, LanguageBridge, ObjectHandle, SolverForgeError,
    SolverForgeResult, Value,
};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Arc;

/// Intermediate structure for deserializing ClassInfo from Go
#[derive(Debug, Serialize, Deserialize)]
struct ClassInfoData {
    name: String,
    fields: Vec<FieldInfoData>,
    annotations: Vec<PlanningAnnotation>,
}

/// Intermediate structure for deserializing FieldInfo
#[derive(Debug, Serialize, Deserialize)]
struct FieldInfoData {
    name: String,
    field_type: FieldType,
    annotations: Vec<PlanningAnnotation>,
}

/// Callback function pointers for calling into Go from Rust
///
/// These function pointers allow Rust code to request operations on Go objects.
/// Each callback corresponds to a LanguageBridge method.
#[derive(Clone, Default)]
pub struct GoCallbacks {
    /// Get field from Go object: (go_ref_id, field_name) -> JSON value
    pub get_field_fn: Option<extern "C" fn(u64, *const c_char) -> *mut c_char>,

    /// Set field on Go object: (go_ref_id, field_name, JSON value) -> bool (success)
    pub set_field_fn: Option<extern "C" fn(u64, *const c_char, *const c_char) -> bool>,

    /// Call Go function: (go_ref_id, JSON args) -> JSON result
    pub call_func_fn: Option<extern "C" fn(u64, *const c_char) -> *mut c_char>,

    /// Serialize Go object: (go_ref_id) -> JSON
    pub serialize_fn: Option<extern "C" fn(u64) -> *mut c_char>,

    /// Deserialize JSON to Go object: (JSON, class_name) -> go_ref_id
    pub deserialize_fn: Option<extern "C" fn(*const c_char, *const c_char) -> u64>,

    /// Get class info from Go: (go_ref_id) -> JSON (ClassInfo)
    pub get_class_info_fn: Option<extern "C" fn(u64) -> *mut c_char>,

    /// Clone Go object: (go_ref_id) -> new_go_ref_id
    pub clone_fn: Option<extern "C" fn(u64) -> u64>,

    /// Get list size: (go_ref_id) -> size
    pub get_list_size_fn: Option<extern "C" fn(u64) -> usize>,

    /// Get list item: (go_ref_id, index) -> JSON value
    pub get_list_item_fn: Option<extern "C" fn(u64, usize) -> *mut c_char>,
}

/// GoBridge implements LanguageBridge for Go
///
/// This bridge maintains a registry of Go object/function references and
/// provides methods to interact with them via callbacks into Go code.
pub struct GoBridge {
    /// Handle registry for managing Go object/function references
    registry: Arc<HandleRegistry>,

    /// Callbacks for invoking operations on Go objects
    callbacks: Arc<GoCallbacks>,
}

impl GoBridge {
    /// Create a new GoBridge
    pub fn new() -> Self {
        Self {
            registry: Arc::new(HandleRegistry::new()),
            callbacks: Arc::new(GoCallbacks::default()),
        }
    }

    /// Create a new GoBridge with callbacks
    pub fn with_callbacks(callbacks: GoCallbacks) -> Self {
        Self {
            registry: Arc::new(HandleRegistry::new()),
            callbacks: Arc::new(callbacks),
        }
    }

    /// Set callbacks on an existing bridge
    pub fn set_callbacks(&mut self, callbacks: GoCallbacks) {
        self.callbacks = Arc::new(callbacks);
    }

    /// Get the registry
    pub fn registry(&self) -> &Arc<HandleRegistry> {
        &self.registry
    }

    /// Register a Go object and return its ObjectHandle
    pub fn register_object(&self, go_ref_id: u64) -> ObjectHandle {
        let handle_id = self.registry.register_object(go_ref_id);
        ObjectHandle::new(handle_id)
    }

    /// Register a Go object with class name and return its ObjectHandle
    pub fn register_object_with_class(&self, go_ref_id: u64, class_name: String) -> ObjectHandle {
        let handle_id = self
            .registry
            .register_object_with_class(go_ref_id, class_name);
        ObjectHandle::new(handle_id)
    }

    /// Get Go object reference by handle
    fn get_go_object(&self, handle: ObjectHandle) -> SolverForgeResult<GoObjectRef> {
        self.registry.get_object(handle.id()).ok_or_else(|| {
            SolverForgeError::Bridge(format!("Object handle {} not found", handle.id()))
        })
    }

    /// Get Go function reference by handle
    fn get_go_function(&self, handle: FunctionHandle) -> SolverForgeResult<GoFunctionRef> {
        self.registry.get_function(handle.id()).ok_or_else(|| {
            SolverForgeError::Bridge(format!("Function handle {} not found", handle.id()))
        })
    }

    /// Convert a JSON string from C to a Rust Value
    unsafe fn json_to_value(&self, json_ptr: *mut c_char) -> SolverForgeResult<Value> {
        if json_ptr.is_null() {
            return Err(SolverForgeError::Bridge(
                "Callback returned null JSON".to_string(),
            ));
        }

        let c_str = CStr::from_ptr(json_ptr);
        let json_str = c_str
            .to_str()
            .map_err(|e| SolverForgeError::Bridge(format!("Invalid UTF-8 from Go: {}", e)))?;

        let json_value: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| SolverForgeError::Serialization(e.to_string()))?;

        // Free the C string
        let _ = CString::from_raw(json_ptr);

        Ok(Value::from(json_value))
    }

    /// Convert a Rust Value to JSON for passing to Go
    fn value_to_json(&self, value: &Value) -> SolverForgeResult<CString> {
        let json_value = serde_json::to_value(value)
            .map_err(|e| SolverForgeError::Serialization(e.to_string()))?;
        let json_str = json_value.to_string();
        CString::new(json_str)
            .map_err(|e| SolverForgeError::Bridge(format!("CString conversion failed: {}", e)))
    }
}

impl Default for GoBridge {
    fn default() -> Self {
        Self::new()
    }
}

// Safety: GoBridge uses Arc<HandleRegistry> which is Send + Sync
unsafe impl Send for GoBridge {}
unsafe impl Sync for GoBridge {}

impl LanguageBridge for GoBridge {
    fn call_function(&self, func: FunctionHandle, args: &[Value]) -> SolverForgeResult<Value> {
        let func_ref = self.get_go_function(func)?;

        let callback = self
            .callbacks
            .call_func_fn
            .ok_or_else(|| SolverForgeError::Bridge("call_func callback not set".to_string()))?;

        // Serialize args to JSON
        let args_json = serde_json::to_value(args)
            .map_err(|e| SolverForgeError::Serialization(e.to_string()))?;
        let args_str = args_json.to_string();
        let c_args = CString::new(args_str)
            .map_err(|e| SolverForgeError::Bridge(format!("CString conversion: {}", e)))?;

        // Call into Go
        let result_ptr = callback(func_ref.go_ref_id, c_args.as_ptr());

        // Convert result back
        unsafe { self.json_to_value(result_ptr) }
    }

    fn get_field(&self, obj: ObjectHandle, field: &str) -> SolverForgeResult<Value> {
        let obj_ref = self.get_go_object(obj)?;

        let callback = self
            .callbacks
            .get_field_fn
            .ok_or_else(|| SolverForgeError::Bridge("get_field callback not set".to_string()))?;

        let c_field = CString::new(field)
            .map_err(|e| SolverForgeError::Bridge(format!("CString conversion: {}", e)))?;

        let result_ptr = callback(obj_ref.go_ref_id, c_field.as_ptr());

        unsafe { self.json_to_value(result_ptr) }
    }

    fn set_field(&self, obj: ObjectHandle, field: &str, value: Value) -> SolverForgeResult<()> {
        let obj_ref = self.get_go_object(obj)?;

        let callback = self
            .callbacks
            .set_field_fn
            .ok_or_else(|| SolverForgeError::Bridge("set_field callback not set".to_string()))?;

        let c_field = CString::new(field)
            .map_err(|e| SolverForgeError::Bridge(format!("CString conversion: {}", e)))?;

        let c_value = self.value_to_json(&value)?;

        let success = callback(obj_ref.go_ref_id, c_field.as_ptr(), c_value.as_ptr());

        if success {
            Ok(())
        } else {
            Err(SolverForgeError::Bridge(format!(
                "Failed to set field '{}' on object",
                field
            )))
        }
    }

    fn serialize_object(&self, obj: ObjectHandle) -> SolverForgeResult<String> {
        let obj_ref = self.get_go_object(obj)?;

        let callback = self
            .callbacks
            .serialize_fn
            .ok_or_else(|| SolverForgeError::Bridge("serialize callback not set".to_string()))?;

        let result_ptr = callback(obj_ref.go_ref_id);

        if result_ptr.is_null() {
            return Err(SolverForgeError::Bridge(
                "Serialize callback returned null".to_string(),
            ));
        }

        unsafe {
            let c_str = CStr::from_ptr(result_ptr);
            let result = c_str
                .to_str()
                .map_err(|e| SolverForgeError::Bridge(format!("Invalid UTF-8: {}", e)))?
                .to_string();

            // Free the C string
            let _ = CString::from_raw(result_ptr);

            Ok(result)
        }
    }

    fn deserialize_object(&self, json: &str, class_name: &str) -> SolverForgeResult<ObjectHandle> {
        let callback = self
            .callbacks
            .deserialize_fn
            .ok_or_else(|| SolverForgeError::Bridge("deserialize callback not set".to_string()))?;

        let c_json = CString::new(json)
            .map_err(|e| SolverForgeError::Bridge(format!("CString conversion: {}", e)))?;
        let c_class = CString::new(class_name)
            .map_err(|e| SolverForgeError::Bridge(format!("CString conversion: {}", e)))?;

        let go_ref_id = callback(c_json.as_ptr(), c_class.as_ptr());

        if go_ref_id == 0 {
            return Err(SolverForgeError::Bridge(
                "Deserialize callback returned 0 (failure)".to_string(),
            ));
        }

        Ok(self.register_object_with_class(go_ref_id, class_name.to_string()))
    }

    fn get_class_info(&self, obj: ObjectHandle) -> SolverForgeResult<ClassInfo> {
        let obj_ref = self.get_go_object(obj)?;

        let callback = self.callbacks.get_class_info_fn.ok_or_else(|| {
            SolverForgeError::Bridge("get_class_info callback not set".to_string())
        })?;

        let result_ptr = callback(obj_ref.go_ref_id);

        if result_ptr.is_null() {
            return Err(SolverForgeError::Bridge(
                "get_class_info callback returned null".to_string(),
            ));
        }

        unsafe {
            let c_str = CStr::from_ptr(result_ptr);
            let json_str = c_str
                .to_str()
                .map_err(|e| SolverForgeError::Bridge(format!("Invalid UTF-8: {}", e)))?;

            // Deserialize into intermediate structure
            let data: ClassInfoData = serde_json::from_str(json_str)
                .map_err(|e| SolverForgeError::Serialization(e.to_string()))?;

            // Free the C string
            let _ = CString::from_raw(result_ptr);

            // Build ClassInfo from data
            let mut class_info = ClassInfo::new(data.name);
            for field_data in data.fields {
                let mut field_info = FieldInfo::new(field_data.name, field_data.field_type);
                for annotation in field_data.annotations {
                    field_info = field_info.with_annotation(annotation);
                }
                class_info = class_info.with_field(field_info);
            }
            for annotation in data.annotations {
                class_info = class_info.with_annotation(annotation);
            }

            Ok(class_info)
        }
    }

    fn register_function(&self, func: ObjectHandle) -> SolverForgeResult<FunctionHandle> {
        let obj_ref = self.get_go_object(func)?;
        let handle_id = self.registry.register_function(obj_ref.go_ref_id);
        Ok(FunctionHandle::new(handle_id))
    }

    fn clone_object(&self, obj: ObjectHandle) -> SolverForgeResult<ObjectHandle> {
        let obj_ref = self.get_go_object(obj)?;

        let callback = self
            .callbacks
            .clone_fn
            .ok_or_else(|| SolverForgeError::Bridge("clone callback not set".to_string()))?;

        let new_go_ref_id = callback(obj_ref.go_ref_id);

        if new_go_ref_id == 0 {
            return Err(SolverForgeError::Bridge(
                "Clone callback returned 0 (failure)".to_string(),
            ));
        }

        // Preserve class name if it exists
        if let Some(class_name) = &obj_ref.class_name {
            Ok(self.register_object_with_class(new_go_ref_id, class_name.clone()))
        } else {
            Ok(self.register_object(new_go_ref_id))
        }
    }

    fn get_list_size(&self, obj: ObjectHandle) -> SolverForgeResult<usize> {
        let obj_ref = self.get_go_object(obj)?;

        let callback = self.callbacks.get_list_size_fn.ok_or_else(|| {
            SolverForgeError::Bridge("get_list_size callback not set".to_string())
        })?;

        let size = callback(obj_ref.go_ref_id);
        Ok(size)
    }

    fn get_list_item(&self, obj: ObjectHandle, index: usize) -> SolverForgeResult<Value> {
        let obj_ref = self.get_go_object(obj)?;

        let callback = self.callbacks.get_list_item_fn.ok_or_else(|| {
            SolverForgeError::Bridge("get_list_item callback not set".to_string())
        })?;

        let result_ptr = callback(obj_ref.go_ref_id, index);

        unsafe { self.json_to_value(result_ptr) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_creation() {
        let bridge = GoBridge::new();
        assert_eq!(bridge.registry.object_count(), 0);
        assert_eq!(bridge.registry.function_count(), 0);
    }

    #[test]
    fn test_register_object() {
        let bridge = GoBridge::new();
        let handle = bridge.register_object(100);
        assert_ne!(handle.id(), 0);
        assert_eq!(bridge.registry.object_count(), 1);
    }

    #[test]
    fn test_register_function() {
        let bridge = GoBridge::new();
        let obj_handle = bridge.register_object(100);
        let func_handle = bridge.register_function(obj_handle).unwrap();
        assert_ne!(func_handle.id(), 0);
        assert_eq!(bridge.registry.function_count(), 1);
    }

    #[test]
    fn test_register_object_with_class() {
        let bridge = GoBridge::new();
        let handle = bridge.register_object_with_class(100, "TestClass".to_string());
        assert_ne!(handle.id(), 0);
        let obj_ref = bridge.get_go_object(handle).unwrap();
        assert_eq!(obj_ref.class_name, Some("TestClass".to_string()));
    }

    #[test]
    fn test_callbacks_not_set_returns_error() {
        let bridge = GoBridge::new();
        let handle = bridge.register_object(100);

        // All methods should return error when callbacks not set
        assert!(bridge.get_field(handle, "test").is_err());
        assert!(bridge.set_field(handle, "test", Value::Null).is_err());
        assert!(bridge.serialize_object(handle).is_err());
        assert!(bridge.get_class_info(handle).is_err());
        assert!(bridge.clone_object(handle).is_err());
        assert!(bridge.get_list_size(handle).is_err());
        assert!(bridge.get_list_item(handle, 0).is_err());
    }
}
