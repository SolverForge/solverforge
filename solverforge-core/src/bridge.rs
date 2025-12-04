use crate::domain::{FieldType, PlanningAnnotation};
use crate::{FunctionHandle, ObjectHandle, SolverForgeResult, Value};

pub trait LanguageBridge: Send + Sync {
    fn call_function(&self, func: FunctionHandle, args: &[Value]) -> SolverForgeResult<Value>;

    fn get_field(&self, obj: ObjectHandle, field: &str) -> SolverForgeResult<Value>;

    fn set_field(&self, obj: ObjectHandle, field: &str, value: Value) -> SolverForgeResult<()>;

    fn serialize_object(&self, obj: ObjectHandle) -> SolverForgeResult<String>;

    fn deserialize_object(&self, json: &str, class_name: &str) -> SolverForgeResult<ObjectHandle>;

    fn get_class_info(&self, obj: ObjectHandle) -> SolverForgeResult<ClassInfo>;

    fn register_function(&self, func: ObjectHandle) -> SolverForgeResult<FunctionHandle>;

    fn clone_object(&self, obj: ObjectHandle) -> SolverForgeResult<ObjectHandle>;

    fn get_list_size(&self, obj: ObjectHandle) -> SolverForgeResult<usize>;

    fn get_list_item(&self, obj: ObjectHandle, index: usize) -> SolverForgeResult<Value>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassInfo {
    pub name: String,
    pub fields: Vec<FieldInfo>,
    pub annotations: Vec<PlanningAnnotation>,
}

impl ClassInfo {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            fields: Vec::new(),
            annotations: Vec::new(),
        }
    }

    pub fn with_field(mut self, field: FieldInfo) -> Self {
        self.fields.push(field);
        self
    }

    pub fn with_annotation(mut self, annotation: PlanningAnnotation) -> Self {
        self.annotations.push(annotation);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldInfo {
    pub name: String,
    pub field_type: FieldType,
    pub annotations: Vec<PlanningAnnotation>,
}

impl FieldInfo {
    pub fn new(name: impl Into<String>, field_type: FieldType) -> Self {
        Self {
            name: name.into(),
            field_type,
            annotations: Vec::new(),
        }
    }

    pub fn with_annotation(mut self, annotation: PlanningAnnotation) -> Self {
        self.annotations.push(annotation);
        self
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::domain::PrimitiveType;
    use crate::SolverForgeError;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    pub struct MockBridge {
        objects: Arc<Mutex<HashMap<u64, Value>>>,
        next_handle: Arc<Mutex<u64>>,
    }

    impl MockBridge {
        pub fn new() -> Self {
            Self {
                objects: Arc::new(Mutex::new(HashMap::new())),
                next_handle: Arc::new(Mutex::new(1)),
            }
        }

        pub fn store_object(&self, value: Value) -> ObjectHandle {
            let mut objects = self.objects.lock().unwrap();
            let mut next = self.next_handle.lock().unwrap();
            let handle = *next;
            *next += 1;
            objects.insert(handle, value);
            ObjectHandle::new(handle)
        }

        pub fn get_object(&self, handle: ObjectHandle) -> Option<Value> {
            self.objects.lock().unwrap().get(&handle.id()).cloned()
        }
    }

    impl LanguageBridge for MockBridge {
        fn call_function(&self, _func: FunctionHandle, args: &[Value]) -> SolverForgeResult<Value> {
            Ok(args.first().cloned().unwrap_or(Value::Null))
        }

        fn get_field(&self, obj: ObjectHandle, field: &str) -> SolverForgeResult<Value> {
            let value = self
                .get_object(obj)
                .ok_or_else(|| SolverForgeError::Bridge(format!("Object not found: {:?}", obj)))?;

            match value {
                Value::Object(map) => Ok(map.get(field).cloned().unwrap_or(Value::Null)),
                _ => Err(SolverForgeError::Bridge("Not an object".to_string())),
            }
        }

        fn set_field(&self, obj: ObjectHandle, field: &str, value: Value) -> SolverForgeResult<()> {
            let mut objects = self.objects.lock().unwrap();
            let stored = objects
                .get_mut(&obj.id())
                .ok_or_else(|| SolverForgeError::Bridge(format!("Object not found: {:?}", obj)))?;

            match stored {
                Value::Object(map) => {
                    map.insert(field.to_string(), value);
                    Ok(())
                }
                _ => Err(SolverForgeError::Bridge("Not an object".to_string())),
            }
        }

        fn serialize_object(&self, obj: ObjectHandle) -> SolverForgeResult<String> {
            let value = self
                .get_object(obj)
                .ok_or_else(|| SolverForgeError::Bridge(format!("Object not found: {:?}", obj)))?;

            serde_json::to_string(&value)
                .map_err(|e| SolverForgeError::Serialization(e.to_string()))
        }

        fn deserialize_object(
            &self,
            json: &str,
            _class_name: &str,
        ) -> SolverForgeResult<ObjectHandle> {
            let value: Value = serde_json::from_str(json)
                .map_err(|e| SolverForgeError::Serialization(e.to_string()))?;

            Ok(self.store_object(value))
        }

        fn get_class_info(&self, _obj: ObjectHandle) -> SolverForgeResult<ClassInfo> {
            Ok(ClassInfo::new("MockClass")
                .with_field(FieldInfo::new(
                    "id",
                    FieldType::Primitive(PrimitiveType::String),
                ))
                .with_annotation(PlanningAnnotation::PlanningEntity))
        }

        fn register_function(&self, func: ObjectHandle) -> SolverForgeResult<FunctionHandle> {
            Ok(FunctionHandle::new(func.id()))
        }

        fn clone_object(&self, obj: ObjectHandle) -> SolverForgeResult<ObjectHandle> {
            let value = self
                .get_object(obj)
                .ok_or_else(|| SolverForgeError::Bridge(format!("Object not found: {:?}", obj)))?;

            Ok(self.store_object(value))
        }

        fn get_list_size(&self, obj: ObjectHandle) -> SolverForgeResult<usize> {
            let value = self
                .get_object(obj)
                .ok_or_else(|| SolverForgeError::Bridge(format!("Object not found: {:?}", obj)))?;

            match value {
                Value::Array(arr) => Ok(arr.len()),
                _ => Err(SolverForgeError::Bridge("Not an array".to_string())),
            }
        }

        fn get_list_item(&self, obj: ObjectHandle, index: usize) -> SolverForgeResult<Value> {
            let value = self
                .get_object(obj)
                .ok_or_else(|| SolverForgeError::Bridge(format!("Object not found: {:?}", obj)))?;

            match value {
                Value::Array(arr) => arr.get(index).cloned().ok_or_else(|| {
                    SolverForgeError::Bridge(format!("Index out of bounds: {}", index))
                }),
                _ => Err(SolverForgeError::Bridge("Not an array".to_string())),
            }
        }
    }

    #[test]
    fn test_class_info() {
        let info = ClassInfo::new("Lesson")
            .with_annotation(PlanningAnnotation::PlanningEntity)
            .with_field(FieldInfo::new(
                "id",
                FieldType::Primitive(PrimitiveType::String),
            ));

        assert_eq!(info.name, "Lesson");
        assert_eq!(info.fields.len(), 1);
        assert_eq!(info.annotations.len(), 1);
    }

    #[test]
    fn test_field_info() {
        let field = FieldInfo::new("room", FieldType::object("Room")).with_annotation(
            PlanningAnnotation::planning_variable(vec!["rooms".to_string()]),
        );

        assert_eq!(field.name, "room");
        assert_eq!(field.annotations.len(), 1);
    }

    #[test]
    fn test_mock_bridge_store_and_get() {
        let bridge = MockBridge::new();

        let mut map = HashMap::new();
        map.insert("name".to_string(), Value::String("Test".to_string()));
        let obj = bridge.store_object(Value::Object(map));

        let value = bridge.get_field(obj, "name").unwrap();
        assert_eq!(value, Value::String("Test".to_string()));
    }

    #[test]
    fn test_mock_bridge_set_field() {
        let bridge = MockBridge::new();

        let mut map = HashMap::new();
        map.insert("value".to_string(), Value::Int(0));
        let obj = bridge.store_object(Value::Object(map));

        bridge.set_field(obj, "value", Value::Int(42)).unwrap();

        let value = bridge.get_field(obj, "value").unwrap();
        assert_eq!(value, Value::Int(42));
    }

    #[test]
    fn test_mock_bridge_serialize() {
        let bridge = MockBridge::new();

        let mut map = HashMap::new();
        map.insert("x".to_string(), Value::Int(1));
        let obj = bridge.store_object(Value::Object(map));

        let json = bridge.serialize_object(obj).unwrap();
        assert!(json.contains("\"x\":1"));
    }

    #[test]
    fn test_mock_bridge_deserialize() {
        let bridge = MockBridge::new();

        let json = r#"{"name":"Test","value":42}"#;
        let obj = bridge.deserialize_object(json, "TestClass").unwrap();

        let name = bridge.get_field(obj, "name").unwrap();
        assert_eq!(name, Value::String("Test".to_string()));
    }

    #[test]
    fn test_mock_bridge_clone() {
        let bridge = MockBridge::new();

        let mut map = HashMap::new();
        map.insert("id".to_string(), Value::Int(1));
        let obj = bridge.store_object(Value::Object(map));

        let cloned = bridge.clone_object(obj).unwrap();
        assert_ne!(obj.id(), cloned.id());

        let value = bridge.get_field(cloned, "id").unwrap();
        assert_eq!(value, Value::Int(1));
    }

    #[test]
    fn test_mock_bridge_list_operations() {
        let bridge = MockBridge::new();

        let arr = Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
        let obj = bridge.store_object(arr);

        assert_eq!(bridge.get_list_size(obj).unwrap(), 3);
        assert_eq!(bridge.get_list_item(obj, 0).unwrap(), Value::Int(1));
        assert_eq!(bridge.get_list_item(obj, 2).unwrap(), Value::Int(3));
    }

    #[test]
    fn test_mock_bridge_call_function() {
        let bridge = MockBridge::new();

        let func = FunctionHandle::new(1);
        let result = bridge.call_function(func, &[Value::Int(42)]).unwrap();
        assert_eq!(result, Value::Int(42));
    }
}
