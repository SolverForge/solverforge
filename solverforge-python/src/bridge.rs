//! Python Bridge Implementation
//!
//! Implements the `LanguageBridge` trait for Python, enabling the core solver
//! to interact with Python objects.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use solverforge_core::domain::{FieldType, PlanningAnnotation, PrimitiveType};
use solverforge_core::{
    ClassInfo, FieldInfo, FunctionHandle, LanguageBridge, ObjectHandle, SolverForgeError,
    SolverForgeResult, Value,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Python bridge that implements LanguageBridge for Python objects.
///
/// This bridge maintains a registry of Python objects and provides methods
/// to interact with them from Rust code.
pub struct PythonBridge {
    /// Registry of Python objects indexed by handle ID
    objects: Arc<Mutex<HashMap<u64, Py<PyAny>>>>,
    /// Registry of Python callables (functions) indexed by handle ID
    functions: Arc<Mutex<HashMap<u64, Py<PyAny>>>>,
    /// Registry of Python classes indexed by class name
    classes: Arc<Mutex<HashMap<String, Py<PyAny>>>>,
    /// Counter for generating unique handles
    next_handle: Arc<Mutex<u64>>,
}

impl PythonBridge {
    /// Create a new Python bridge.
    pub fn new() -> Self {
        Self {
            objects: Arc::new(Mutex::new(HashMap::new())),
            functions: Arc::new(Mutex::new(HashMap::new())),
            classes: Arc::new(Mutex::new(HashMap::new())),
            next_handle: Arc::new(Mutex::new(1)),
        }
    }

    /// Register a Python class for deserialization.
    ///
    /// When `deserialize_object` is called with a class name, it will look up
    /// this registry to instantiate the proper class.
    pub fn register_class(&self, name: &str, cls: Py<PyAny>) {
        self.classes.lock().unwrap().insert(name.to_string(), cls);
    }

    /// Get a registered Python class by name.
    pub fn get_class(&self, name: &str) -> Option<Py<PyAny>> {
        Python::attach(|py| {
            self.classes
                .lock()
                .unwrap()
                .get(name)
                .map(|cls| cls.clone_ref(py))
        })
    }

    /// Generate the next unique handle ID.
    fn next_id(&self) -> u64 {
        let mut next = self.next_handle.lock().unwrap();
        let id = *next;
        *next += 1;
        id
    }

    /// Register a Python object and return its handle.
    pub fn register_object(&self, obj: Py<PyAny>) -> ObjectHandle {
        let id = self.next_id();
        self.objects.lock().unwrap().insert(id, obj);
        ObjectHandle::new(id)
    }

    /// Get a Python object by its handle.
    pub fn get_py_object(&self, handle: ObjectHandle) -> Option<Py<PyAny>> {
        Python::attach(|py| {
            self.objects
                .lock()
                .unwrap()
                .get(&handle.id())
                .map(|obj| obj.clone_ref(py))
        })
    }

    /// Release a Python object from the registry.
    pub fn release_object(&self, handle: ObjectHandle) {
        self.objects.lock().unwrap().remove(&handle.id());
    }

    /// Convert a Python object to a Value.
    ///
    /// The `Bound<'_, PyAny>` already proves we hold the GIL, so no separate
    /// `Python` token is needed.
    pub fn py_to_value(obj: &Bound<'_, PyAny>) -> SolverForgeResult<Value> {
        if obj.is_none() {
            return Ok(Value::Null);
        }

        // Check for boolean first (before int, since bool is a subtype of int in Python)
        if let Ok(b) = obj.extract::<bool>() {
            return Ok(Value::Bool(b));
        }

        // Integer
        if let Ok(i) = obj.extract::<i64>() {
            return Ok(Value::Int(i));
        }

        // Float
        if let Ok(f) = obj.extract::<f64>() {
            return Ok(Value::Float(f));
        }

        // String
        if let Ok(s) = obj.extract::<String>() {
            return Ok(Value::String(s));
        }

        // List/tuple
        if let Ok(list) = obj.cast::<PyList>() {
            let mut arr = Vec::new();
            for item in list.iter() {
                arr.push(Self::py_to_value(&item)?);
            }
            return Ok(Value::Array(arr));
        }

        // Dict
        if let Ok(dict) = obj.cast::<PyDict>() {
            let mut map = HashMap::new();
            for (key, value) in dict.iter() {
                let key_str = key
                    .extract::<String>()
                    .map_err(|e| SolverForgeError::Bridge(format!("Dict key not string: {}", e)))?;
                map.insert(key_str, Self::py_to_value(&value)?);
            }
            return Ok(Value::Object(map));
        }

        // For other objects, try to convert via __dict__ or serialize
        if let Ok(dict) = obj.getattr("__dict__") {
            if let Ok(dict) = dict.cast::<PyDict>() {
                let mut map = HashMap::new();
                for (key, value) in dict.iter() {
                    if let Ok(key_str) = key.extract::<String>() {
                        // Skip private attributes
                        if !key_str.starts_with('_') {
                            map.insert(key_str, Self::py_to_value(&value)?);
                        }
                    }
                }
                return Ok(Value::Object(map));
            }
        }

        // Fallback: try repr
        let repr = obj
            .repr()
            .map(|s| s.to_string())
            .unwrap_or_else(|_| "<unknown>".to_string());
        Ok(Value::String(repr))
    }

    /// Convert a Value to a Python object.
    pub fn value_to_py<'py>(py: Python<'py>, value: &Value) -> PyResult<Bound<'py, PyAny>> {
        match value {
            Value::Null => Ok(py.None().into_bound(py)),
            Value::Bool(b) => Ok(b.into_pyobject(py)?.to_owned().into_any()),
            Value::Int(i) => Ok(i.into_pyobject(py)?.to_owned().into_any()),
            Value::Float(f) => Ok(f.into_pyobject(py)?.to_owned().into_any()),
            Value::Decimal(d) => {
                // Convert decimal to string for Python
                let s = d.to_string();
                Ok(s.into_pyobject(py)?.into_any())
            }
            Value::String(s) => Ok(s.into_pyobject(py)?.into_any()),
            Value::Array(arr) => {
                let list = PyList::empty(py);
                for item in arr {
                    list.append(Self::value_to_py(py, item)?)?;
                }
                Ok(list.into_any())
            }
            Value::Object(map) => {
                let dict = PyDict::new(py);
                for (k, v) in map {
                    dict.set_item(k, Self::value_to_py(py, v)?)?;
                }
                Ok(dict.into_any())
            }
            Value::ObjectRef(_handle) => {
                // Object references should be resolved separately
                Ok(py.None().into_bound(py))
            }
        }
    }

    /// Extract field type from Python type annotation.
    #[allow(clippy::only_used_in_recursion)]
    fn extract_field_type(py: Python<'_>, type_obj: &Bound<'_, PyAny>) -> FieldType {
        // Try to get __origin__ for generic types (e.g., List[str] -> list)
        let origin = type_obj.getattr("__origin__");

        // Get the type name (from origin if generic, otherwise directly)
        let (type_name, is_generic) = if let Ok(origin_type) = &origin {
            let name = origin_type
                .getattr("__name__")
                .map(|n| n.to_string())
                .unwrap_or_else(|_| "object".to_string());
            (name, true)
        } else {
            let name = type_obj
                .getattr("__name__")
                .map(|n| n.to_string())
                .or_else(|_| type_obj.repr().map(|r| r.to_string()))
                .unwrap_or_else(|_| "object".to_string());
            (name, false)
        };

        match type_name.as_str() {
            "bool" => FieldType::Primitive(PrimitiveType::Bool),
            "int" => FieldType::Primitive(PrimitiveType::Int),
            "float" => FieldType::Primitive(PrimitiveType::Double),
            "str" => FieldType::Primitive(PrimitiveType::String),
            "list" | "List" => {
                // Extract element type from __args__ if this is a generic type
                let element_type = if is_generic {
                    if let Ok(args) = type_obj.getattr("__args__") {
                        if let Ok(args_tuple) = args.cast::<pyo3::types::PyTuple>() {
                            if !args_tuple.is_empty() {
                                let first_arg = args_tuple.get_item(0).ok();
                                if let Some(arg) = first_arg {
                                    return FieldType::list(Self::extract_field_type(py, &arg));
                                }
                            }
                        }
                    }
                    // Couldn't extract, default to Any
                    FieldType::Primitive(PrimitiveType::String)
                } else {
                    // Plain list without type args
                    FieldType::Primitive(PrimitiveType::String)
                };
                FieldType::list(element_type)
            }
            _ => FieldType::object(type_name),
        }
    }
}

impl Default for PythonBridge {
    fn default() -> Self {
        Self::new()
    }
}

// Safety: PythonBridge is Send + Sync because it uses Arc<Mutex<...>> for interior mutability
// and all Python operations acquire the GIL
unsafe impl Send for PythonBridge {}
unsafe impl Sync for PythonBridge {}

impl LanguageBridge for PythonBridge {
    fn call_function(&self, func: FunctionHandle, args: &[Value]) -> SolverForgeResult<Value> {
        Python::attach(|py| {
            let functions = self.functions.lock().unwrap();
            let py_func = functions.get(&func.id()).ok_or_else(|| {
                SolverForgeError::Bridge(format!("Function not found: {:?}", func))
            })?;

            // Convert args to Python
            let py_args: Vec<Bound<'_, PyAny>> = args
                .iter()
                .map(|v| Self::value_to_py(py, v))
                .collect::<PyResult<Vec<_>>>()
                .map_err(|e| SolverForgeError::Bridge(format!("Failed to convert args: {}", e)))?;

            let py_tuple = pyo3::types::PyTuple::new(py, py_args)
                .map_err(|e| SolverForgeError::Bridge(format!("Failed to create tuple: {}", e)))?;

            // Call the function
            let result = py_func
                .call1(py, py_tuple)
                .map_err(|e| SolverForgeError::Bridge(format!("Function call failed: {}", e)))?;

            Self::py_to_value(result.bind(py))
        })
    }

    fn get_field(&self, obj: ObjectHandle, field: &str) -> SolverForgeResult<Value> {
        Python::attach(|py| {
            let py_obj = self
                .get_py_object(obj)
                .ok_or_else(|| SolverForgeError::Bridge(format!("Object not found: {:?}", obj)))?;

            let bound = py_obj.bind(py);

            // Try dict access first (for dict objects)
            if let Ok(dict) = bound.cast::<PyDict>() {
                if let Some(value) = dict.get_item(field).ok().flatten() {
                    return Self::py_to_value(&value);
                }
            }

            // Fall back to attribute access
            let value = bound.getattr(field).map_err(|e| {
                SolverForgeError::Bridge(format!("Failed to get field '{}': {}", field, e))
            })?;

            Self::py_to_value(&value)
        })
    }

    fn set_field(&self, obj: ObjectHandle, field: &str, value: Value) -> SolverForgeResult<()> {
        Python::attach(|py| {
            let py_obj = self
                .get_py_object(obj)
                .ok_or_else(|| SolverForgeError::Bridge(format!("Object not found: {:?}", obj)))?;

            let py_value = Self::value_to_py(py, &value)
                .map_err(|e| SolverForgeError::Bridge(format!("Failed to convert value: {}", e)))?;

            let bound = py_obj.bind(py);

            // Try dict access first (for dict objects)
            if let Ok(dict) = bound.cast::<PyDict>() {
                dict.set_item(field, py_value).map_err(|e| {
                    SolverForgeError::Bridge(format!("Failed to set field '{}': {}", field, e))
                })?;
                return Ok(());
            }

            // Fall back to attribute access
            bound.setattr(field, py_value).map_err(|e| {
                SolverForgeError::Bridge(format!("Failed to set field '{}': {}", field, e))
            })?;

            Ok(())
        })
    }

    fn serialize_object(&self, obj: ObjectHandle) -> SolverForgeResult<String> {
        Python::attach(|py| {
            let py_obj = self
                .get_py_object(obj)
                .ok_or_else(|| SolverForgeError::Bridge(format!("Object not found: {:?}", obj)))?;

            let value = Self::py_to_value(py_obj.bind(py))?;
            serde_json::to_string(&value)
                .map_err(|e| SolverForgeError::Serialization(e.to_string()))
        })
    }

    fn deserialize_object(&self, json: &str, class_name: &str) -> SolverForgeResult<ObjectHandle> {
        Python::attach(|py| {
            // Parse JSON to Value first
            let value: Value = serde_json::from_str(json)
                .map_err(|e| SolverForgeError::Serialization(e.to_string()))?;

            // Try to find the registered class and instantiate it
            if let Some(cls) = self.get_class(class_name) {
                // Class is registered - try to instantiate it
                let cls_bound = cls.bind(py);

                // If the value is an Object (dict), pass it as kwargs
                if let Value::Object(map) = &value {
                    let kwargs = PyDict::new(py);
                    for (k, v) in map {
                        let py_val = Self::value_to_py(py, v).map_err(|e| {
                            SolverForgeError::Bridge(format!(
                                "Failed to convert field '{}': {}",
                                k, e
                            ))
                        })?;
                        kwargs.set_item(k, py_val).map_err(|e| {
                            SolverForgeError::Bridge(format!("Failed to set kwarg '{}': {}", k, e))
                        })?;
                    }

                    let instance = cls_bound.call((), Some(&kwargs)).map_err(|e| {
                        SolverForgeError::Bridge(format!(
                            "Failed to instantiate class '{}': {}",
                            class_name, e
                        ))
                    })?;

                    return Ok(self.register_object(instance.unbind()));
                }
            }

            // No registered class or non-dict value - store the dict representation
            let py_obj = Self::value_to_py(py, &value)
                .map_err(|e| SolverForgeError::Bridge(format!("Failed to convert value: {}", e)))?;

            Ok(self.register_object(py_obj.unbind()))
        })
    }

    fn get_class_info(&self, obj: ObjectHandle) -> SolverForgeResult<ClassInfo> {
        Python::attach(|py| {
            let py_obj = self
                .get_py_object(obj)
                .ok_or_else(|| SolverForgeError::Bridge(format!("Object not found: {:?}", obj)))?;

            let py_obj = py_obj.bind(py);

            // Get class name
            let class = py_obj
                .getattr("__class__")
                .map_err(|e| SolverForgeError::Bridge(format!("Failed to get class: {}", e)))?;
            let class_name: String = class
                .getattr("__name__")
                .and_then(|n| n.extract())
                .unwrap_or_else(|_| "Unknown".to_string());

            let mut info = ClassInfo::new(class_name);

            // Check for __solverforge_annotations__ (set by our decorators)
            if let Ok(annotations) = py_obj.getattr("__solverforge_annotations__") {
                if let Ok(ann_list) = annotations.cast::<PyList>() {
                    for ann in ann_list.iter() {
                        if let Ok(ann_name) = ann.extract::<String>() {
                            match ann_name.as_str() {
                                "PlanningEntity" => {
                                    info = info.with_annotation(PlanningAnnotation::PlanningEntity);
                                }
                                "PlanningSolution" => {
                                    info =
                                        info.with_annotation(PlanningAnnotation::PlanningSolution);
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }

            // Get fields from __annotations__ (type hints)
            if let Ok(type_hints) = class.getattr("__annotations__") {
                if let Ok(hints_dict) = type_hints.cast::<PyDict>() {
                    for (name, type_obj) in hints_dict.iter() {
                        if let Ok(field_name) = name.extract::<String>() {
                            if !field_name.starts_with('_') {
                                let field_type = Self::extract_field_type(py, &type_obj);
                                let field_info = FieldInfo::new(field_name, field_type);
                                info = info.with_field(field_info);
                            }
                        }
                    }
                }
            }

            Ok(info)
        })
    }

    fn register_function(&self, func: ObjectHandle) -> SolverForgeResult<FunctionHandle> {
        // The function object should already be in our objects registry
        let py_obj = self
            .get_py_object(func)
            .ok_or_else(|| SolverForgeError::Bridge(format!("Object not found: {:?}", func)))?;

        // Verify it's callable
        Python::attach(|py| {
            let is_callable = py_obj.bind(py).is_callable();
            if !is_callable {
                return Err(SolverForgeError::Bridge(
                    "Object is not callable".to_string(),
                ));
            }
            Ok(())
        })?;

        // Register in functions map
        let id = func.id();
        self.functions.lock().unwrap().insert(id, py_obj);

        Ok(FunctionHandle::new(id))
    }

    fn clone_object(&self, obj: ObjectHandle) -> SolverForgeResult<ObjectHandle> {
        Python::attach(|py| {
            let py_obj = self
                .get_py_object(obj)
                .ok_or_else(|| SolverForgeError::Bridge(format!("Object not found: {:?}", obj)))?;

            // Try copy.deepcopy first
            let copy_module = py.import("copy").map_err(|e| {
                SolverForgeError::Bridge(format!("Failed to import copy module: {}", e))
            })?;

            let cloned = copy_module
                .call_method1("deepcopy", (py_obj.bind(py),))
                .map_err(|e| SolverForgeError::Bridge(format!("Failed to deep copy: {}", e)))?;

            Ok(self.register_object(cloned.unbind()))
        })
    }

    fn get_list_size(&self, obj: ObjectHandle) -> SolverForgeResult<usize> {
        Python::attach(|py| {
            let py_obj = self
                .get_py_object(obj)
                .ok_or_else(|| SolverForgeError::Bridge(format!("Object not found: {:?}", obj)))?;

            let len = py_obj
                .bind(py)
                .len()
                .map_err(|e| SolverForgeError::Bridge(format!("Failed to get length: {}", e)))?;

            Ok(len)
        })
    }

    fn get_list_item(&self, obj: ObjectHandle, index: usize) -> SolverForgeResult<Value> {
        Python::attach(|py| {
            let py_obj = self
                .get_py_object(obj)
                .ok_or_else(|| SolverForgeError::Bridge(format!("Object not found: {:?}", obj)))?;

            let item = py_obj.bind(py).get_item(index).map_err(|e| {
                SolverForgeError::Bridge(format!("Failed to get item at index {}: {}", index, e))
            })?;

            Self::py_to_value(&item)
        })
    }
}

/// PyO3 wrapper for PythonBridge to expose it to Python
#[pyclass(name = "Bridge")]
pub struct PyBridge {
    inner: Arc<PythonBridge>,
}

#[pymethods]
impl PyBridge {
    #[new]
    fn new() -> Self {
        Self {
            inner: Arc::new(PythonBridge::new()),
        }
    }

    /// Register a Python object and return its handle ID.
    fn register(&self, obj: Py<PyAny>) -> u64 {
        self.inner.register_object(obj).id()
    }

    /// Release a Python object by its handle ID.
    fn release(&self, handle_id: u64) {
        self.inner.release_object(ObjectHandle::new(handle_id));
    }

    /// Get a field value from an object.
    fn get_field(&self, handle_id: u64, field: &str) -> PyResult<Py<PyAny>> {
        let value = self
            .inner
            .get_field(ObjectHandle::new(handle_id), field)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        Python::attach(|py| PythonBridge::value_to_py(py, &value).map(|v| v.unbind()))
    }

    /// Set a field value on an object.
    fn set_field(&self, handle_id: u64, field: &str, value: Py<PyAny>) -> PyResult<()> {
        Python::attach(|py| {
            let val = PythonBridge::py_to_value(value.bind(py))
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

            self.inner
                .set_field(ObjectHandle::new(handle_id), field, val)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
        })
    }

    /// Serialize an object to JSON.
    fn serialize(&self, handle_id: u64) -> PyResult<String> {
        self.inner
            .serialize_object(ObjectHandle::new(handle_id))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }
}

impl PyBridge {
    /// Get the inner bridge for use in Rust code.
    pub fn bridge(&self) -> Arc<PythonBridge> {
        Arc::clone(&self.inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_creation() {
        let bridge = PythonBridge::new();
        assert!(bridge.objects.lock().unwrap().is_empty());
    }

    #[test]
    fn test_next_id() {
        let bridge = PythonBridge::new();
        assert_eq!(bridge.next_id(), 1);
        assert_eq!(bridge.next_id(), 2);
        assert_eq!(bridge.next_id(), 3);
    }

    #[test]
    fn test_register_and_get_object() {
        let bridge = PythonBridge::new();
        Python::attach(|py| {
            let obj = PyDict::new(py);
            obj.set_item("key", "value").unwrap();
            let handle = bridge.register_object(obj.into_any().unbind());

            let retrieved = bridge.get_py_object(handle).unwrap();
            let bound = retrieved.bind(py);
            let dict = bound.cast::<PyDict>().unwrap();
            assert_eq!(
                dict.get_item("key")
                    .unwrap()
                    .unwrap()
                    .extract::<String>()
                    .unwrap(),
                "value"
            );
        });
    }

    #[test]
    fn test_release_object() {
        let bridge = PythonBridge::new();
        Python::attach(|py| {
            let obj = py.None().into_bound(py);
            let handle = bridge.register_object(obj.unbind());
            assert!(bridge.get_py_object(handle).is_some());

            bridge.release_object(handle);
            assert!(bridge.get_py_object(handle).is_none());
        });
    }

    #[test]
    fn test_py_to_value_primitives() {
        Python::attach(|py| {
            // None
            let none = py.None();
            assert_eq!(
                PythonBridge::py_to_value(none.bind(py)).unwrap(),
                Value::Null
            );

            // Bool
            let b = true.into_pyobject(py).unwrap();
            assert_eq!(
                PythonBridge::py_to_value(b.as_any()).unwrap(),
                Value::Bool(true)
            );

            // Int
            let i = 42i64.into_pyobject(py).unwrap();
            assert_eq!(
                PythonBridge::py_to_value(i.as_any()).unwrap(),
                Value::Int(42)
            );

            // Float
            let f = 3.14f64.into_pyobject(py).unwrap();
            assert_eq!(
                PythonBridge::py_to_value(f.as_any()).unwrap(),
                Value::Float(3.14)
            );

            // String
            let s = "hello".into_pyobject(py).unwrap();
            assert_eq!(
                PythonBridge::py_to_value(s.as_any()).unwrap(),
                Value::String("hello".to_string())
            );
        });
    }

    #[test]
    fn test_py_to_value_list() {
        Python::attach(|py| {
            let list = PyList::new(py, vec![1i64, 2, 3]).unwrap();
            let value = PythonBridge::py_to_value(list.as_any()).unwrap();

            match value {
                Value::Array(arr) => {
                    assert_eq!(arr.len(), 3);
                    assert_eq!(arr[0], Value::Int(1));
                    assert_eq!(arr[1], Value::Int(2));
                    assert_eq!(arr[2], Value::Int(3));
                }
                _ => panic!("Expected Array"),
            }
        });
    }

    #[test]
    fn test_py_to_value_dict() {
        Python::attach(|py| {
            let dict = PyDict::new(py);
            dict.set_item("name", "test").unwrap();
            dict.set_item("value", 42).unwrap();

            let value = PythonBridge::py_to_value(dict.as_any()).unwrap();

            match value {
                Value::Object(map) => {
                    assert_eq!(map.get("name"), Some(&Value::String("test".to_string())));
                    assert_eq!(map.get("value"), Some(&Value::Int(42)));
                }
                _ => panic!("Expected Object"),
            }
        });
    }

    #[test]
    fn test_value_to_py_primitives() {
        Python::attach(|py| {
            // Null
            let py_none = PythonBridge::value_to_py(py, &Value::Null).unwrap();
            assert!(py_none.is_none());

            // Bool
            let py_bool = PythonBridge::value_to_py(py, &Value::Bool(true)).unwrap();
            assert!(py_bool.extract::<bool>().unwrap());

            // Int
            let py_int = PythonBridge::value_to_py(py, &Value::Int(42)).unwrap();
            assert_eq!(py_int.extract::<i64>().unwrap(), 42);

            // Float
            let py_float = PythonBridge::value_to_py(py, &Value::Float(3.14)).unwrap();
            assert!((py_float.extract::<f64>().unwrap() - 3.14).abs() < 0.001);

            // String
            let py_str =
                PythonBridge::value_to_py(py, &Value::String("hello".to_string())).unwrap();
            assert_eq!(py_str.extract::<String>().unwrap(), "hello");
        });
    }

    #[test]
    fn test_value_to_py_array() {
        Python::attach(|py| {
            let arr = Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
            let py_list = PythonBridge::value_to_py(py, &arr).unwrap();

            let list = py_list.cast::<PyList>().unwrap();
            assert_eq!(list.len(), 3);
            assert_eq!(list.get_item(0).unwrap().extract::<i64>().unwrap(), 1);
        });
    }

    #[test]
    fn test_value_to_py_object() {
        Python::attach(|py| {
            let mut map = std::collections::HashMap::new();
            map.insert("key".to_string(), Value::String("value".to_string()));
            let obj = Value::Object(map);

            let py_dict = PythonBridge::value_to_py(py, &obj).unwrap();
            let dict = py_dict.cast::<PyDict>().unwrap();

            assert_eq!(
                dict.get_item("key")
                    .unwrap()
                    .unwrap()
                    .extract::<String>()
                    .unwrap(),
                "value"
            );
        });
    }

    #[test]
    fn test_get_field() {
        let bridge = PythonBridge::new();
        Python::attach(|py| {
            // Create a simple class instance with attributes using py.run
            let locals = PyDict::new(py);
            py.run(
                c"class Obj:\n    pass\no = Obj()\no.name = 'test'\no.value = 42",
                None,
                Some(&locals),
            )
            .unwrap();
            let obj = locals.get_item("o").unwrap().unwrap();
            let handle = bridge.register_object(obj.unbind());

            let name = bridge.get_field(handle, "name").unwrap();
            assert_eq!(name, Value::String("test".to_string()));

            let value = bridge.get_field(handle, "value").unwrap();
            assert_eq!(value, Value::Int(42));
        });
    }

    #[test]
    fn test_set_field() {
        let bridge = PythonBridge::new();
        Python::attach(|py| {
            let locals = PyDict::new(py);
            py.run(
                c"class Obj:\n    pass\no = Obj()\no.value = 0",
                None,
                Some(&locals),
            )
            .unwrap();
            let obj = locals.get_item("o").unwrap().unwrap();
            let handle = bridge.register_object(obj.unbind());

            bridge.set_field(handle, "value", Value::Int(100)).unwrap();

            let value = bridge.get_field(handle, "value").unwrap();
            assert_eq!(value, Value::Int(100));
        });
    }

    #[test]
    fn test_serialize_object() {
        let bridge = PythonBridge::new();
        Python::attach(|py| {
            let dict = PyDict::new(py);
            dict.set_item("name", "test").unwrap();
            dict.set_item("count", 5).unwrap();

            let handle = bridge.register_object(dict.into_any().unbind());
            let json = bridge.serialize_object(handle).unwrap();

            assert!(json.contains("\"name\":\"test\"") || json.contains("\"name\": \"test\""));
            assert!(json.contains("\"count\":5") || json.contains("\"count\": 5"));
        });
    }

    #[test]
    fn test_deserialize_object() {
        let bridge = PythonBridge::new();
        let json = r#"{"name":"test","value":42}"#;
        let handle = bridge.deserialize_object(json, "TestClass").unwrap();

        let name = bridge.get_field(handle, "name").unwrap();
        assert_eq!(name, Value::String("test".to_string()));

        let value = bridge.get_field(handle, "value").unwrap();
        assert_eq!(value, Value::Int(42));
    }

    #[test]
    fn test_clone_object() {
        let bridge = PythonBridge::new();
        Python::attach(|py| {
            let dict = PyDict::new(py);
            dict.set_item("value", 42).unwrap();

            let handle = bridge.register_object(dict.into_any().unbind());
            let cloned = bridge.clone_object(handle).unwrap();

            // Different handles
            assert_ne!(handle.id(), cloned.id());

            // Same value
            let orig_val = bridge.get_field(handle, "value").unwrap();
            let clone_val = bridge.get_field(cloned, "value").unwrap();
            assert_eq!(orig_val, clone_val);
        });
    }

    #[test]
    fn test_get_list_size() {
        let bridge = PythonBridge::new();
        Python::attach(|py| {
            let list = PyList::new(py, vec![1, 2, 3, 4, 5]).unwrap();
            let handle = bridge.register_object(list.into_any().unbind());

            let size = bridge.get_list_size(handle).unwrap();
            assert_eq!(size, 5);
        });
    }

    #[test]
    fn test_get_list_item() {
        let bridge = PythonBridge::new();
        Python::attach(|py| {
            let list = PyList::new(py, vec![10, 20, 30]).unwrap();
            let handle = bridge.register_object(list.into_any().unbind());

            assert_eq!(bridge.get_list_item(handle, 0).unwrap(), Value::Int(10));
            assert_eq!(bridge.get_list_item(handle, 1).unwrap(), Value::Int(20));
            assert_eq!(bridge.get_list_item(handle, 2).unwrap(), Value::Int(30));
        });
    }

    #[test]
    fn test_register_and_call_function() {
        let bridge = PythonBridge::new();
        Python::attach(|py| {
            // Create a lambda that adds two numbers
            let locals = PyDict::new(py);
            py.run(c"f = lambda x, y: x + y", None, Some(&locals))
                .unwrap();
            let func = locals.get_item("f").unwrap().unwrap();
            let obj_handle = bridge.register_object(func.unbind());
            let func_handle = bridge.register_function(obj_handle).unwrap();

            let result = bridge
                .call_function(func_handle, &[Value::Int(3), Value::Int(4)])
                .unwrap();
            assert_eq!(result, Value::Int(7));
        });
    }

    #[test]
    fn test_call_function_with_strings() {
        let bridge = PythonBridge::new();
        Python::attach(|py| {
            let locals = PyDict::new(py);
            py.run(c"f = lambda a, b: a + ' ' + b", None, Some(&locals))
                .unwrap();
            let func = locals.get_item("f").unwrap().unwrap();
            let obj_handle = bridge.register_object(func.unbind());
            let func_handle = bridge.register_function(obj_handle).unwrap();

            let result = bridge
                .call_function(
                    func_handle,
                    &[
                        Value::String("Hello".to_string()),
                        Value::String("World".to_string()),
                    ],
                )
                .unwrap();
            assert_eq!(result, Value::String("Hello World".to_string()));
        });
    }

    #[test]
    fn test_extract_field_type_generic_list() {
        Python::attach(|py| {
            let locals = PyDict::new(py);
            py.run(
                c"from typing import List\nint_list = List[int]\nstr_list = List[str]",
                None,
                Some(&locals),
            )
            .unwrap();

            // Test List[int]
            let int_list_type = locals.get_item("int_list").unwrap().unwrap();
            let int_field_type = PythonBridge::extract_field_type(py, &int_list_type);
            match int_field_type {
                FieldType::List { element_type } => {
                    assert!(matches!(
                        *element_type,
                        FieldType::Primitive(PrimitiveType::Int)
                    ));
                }
                _ => panic!("Expected List type for List[int]"),
            }

            // Test List[str]
            let str_list_type = locals.get_item("str_list").unwrap().unwrap();
            let str_field_type = PythonBridge::extract_field_type(py, &str_list_type);
            match str_field_type {
                FieldType::List { element_type } => {
                    assert!(matches!(
                        *element_type,
                        FieldType::Primitive(PrimitiveType::String)
                    ));
                }
                _ => panic!("Expected List type for List[str]"),
            }
        });
    }

    #[test]
    fn test_register_class_and_deserialize() {
        let bridge = PythonBridge::new();
        Python::attach(|py| {
            // Create a simple dataclass-like class
            let locals = PyDict::new(py);
            py.run(
                c"class Person:\n    def __init__(self, name=None, age=None):\n        self.name = name\n        self.age = age",
                None,
                Some(&locals),
            )
            .unwrap();
            let person_class = locals.get_item("Person").unwrap().unwrap();

            // Register the class
            bridge.register_class("Person", person_class.unbind());

            // Deserialize with the registered class
            let json = r#"{"name":"Alice","age":30}"#;
            let handle = bridge.deserialize_object(json, "Person").unwrap();

            // Verify it's a proper Person instance
            let name = bridge.get_field(handle, "name").unwrap();
            assert_eq!(name, Value::String("Alice".to_string()));

            let age = bridge.get_field(handle, "age").unwrap();
            assert_eq!(age, Value::Int(30));
        });
    }

    #[test]
    fn test_get_class() {
        let bridge = PythonBridge::new();
        Python::attach(|py| {
            // Create and register a class
            let locals = PyDict::new(py);
            py.run(c"class TestClass: pass", None, Some(&locals))
                .unwrap();
            let cls = locals.get_item("TestClass").unwrap().unwrap();
            bridge.register_class("TestClass", cls.unbind());

            // Verify we can retrieve it
            let retrieved = bridge.get_class("TestClass");
            assert!(retrieved.is_some());

            // Non-existent class should return None
            assert!(bridge.get_class("NonExistent").is_none());
        });
    }
}
