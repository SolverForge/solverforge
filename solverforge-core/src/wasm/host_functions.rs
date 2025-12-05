use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// WASM value types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WasmType {
    /// 32-bit integer
    I32,
    /// 64-bit integer
    I64,
    /// 32-bit float
    F32,
    /// 64-bit float
    F64,
    /// Pointer (i32 in WASM)
    Ptr,
    /// No return value
    Void,
}

/// Host function definition
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostFunctionDef {
    /// Function name (as it appears in imports)
    pub name: String,
    /// Parameter types
    pub params: Vec<WasmType>,
    /// Return type
    pub return_type: WasmType,
}

impl HostFunctionDef {
    pub fn new(name: impl Into<String>, params: Vec<WasmType>, return_type: WasmType) -> Self {
        Self {
            name: name.into(),
            params,
            return_type,
        }
    }
}

/// Registry of host functions available for import into WASM modules
/// Uses IndexMap to preserve insertion order for deterministic function indices
#[derive(Debug, Clone, Default)]
pub struct HostFunctionRegistry {
    functions: IndexMap<String, HostFunctionDef>,
}

impl HostFunctionRegistry {
    /// Create a new registry with standard SolverForge host functions
    pub fn with_standard_functions() -> Self {
        let mut registry = Self::default();

        // String operations
        registry.register(HostFunctionDef::new(
            "hstringEquals",
            vec![WasmType::Ptr, WasmType::Ptr],
            WasmType::I32,
        ));

        // List operations
        registry.register(HostFunctionDef::new(
            "hlistContainsString",
            vec![WasmType::Ptr, WasmType::Ptr],
            WasmType::I32,
        ));
        registry.register(HostFunctionDef::new("hnewList", vec![], WasmType::Ptr));

        registry.register(HostFunctionDef::new(
            "hgetItem",
            vec![WasmType::Ptr, WasmType::I32],
            WasmType::Ptr,
        ));

        registry.register(HostFunctionDef::new(
            "hsize",
            vec![WasmType::Ptr],
            WasmType::I32,
        ));

        registry.register(HostFunctionDef::new(
            "happend",
            vec![WasmType::Ptr, WasmType::Ptr],
            WasmType::Void,
        ));

        registry.register(HostFunctionDef::new(
            "hsetItem",
            vec![WasmType::Ptr, WasmType::I32, WasmType::Ptr],
            WasmType::Void,
        ));

        registry.register(HostFunctionDef::new(
            "hinsert",
            vec![WasmType::Ptr, WasmType::I32, WasmType::Ptr],
            WasmType::Void,
        ));

        registry.register(HostFunctionDef::new(
            "hremove",
            vec![WasmType::Ptr, WasmType::I32],
            WasmType::Void,
        ));

        // Schedule operations
        registry.register(HostFunctionDef::new(
            "hparseSchedule",
            vec![WasmType::I32, WasmType::Ptr], // (length: i32, json: ptr) -> ptr
            WasmType::Ptr,
        ));

        registry.register(HostFunctionDef::new(
            "hscheduleString",
            vec![WasmType::Ptr],
            WasmType::Ptr,
        ));

        // Math operations
        registry.register(HostFunctionDef::new(
            "hround",
            vec![WasmType::F32],
            WasmType::I32,
        ));

        registry
    }

    /// Create an empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a host function
    pub fn register(&mut self, def: HostFunctionDef) {
        self.functions.insert(def.name.clone(), def);
    }

    /// Look up a host function by name
    pub fn lookup(&self, name: &str) -> Option<&HostFunctionDef> {
        self.functions.get(name)
    }

    /// Get all registered function names
    pub fn function_names(&self) -> Vec<&str> {
        self.functions.keys().map(|s| s.as_str()).collect()
    }

    /// Get the number of registered functions
    pub fn len(&self) -> usize {
        self.functions.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.functions.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_registry() {
        let registry = HostFunctionRegistry::new();
        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());
    }

    #[test]
    fn test_register_function() {
        let mut registry = HostFunctionRegistry::new();
        let func = HostFunctionDef::new(
            "test_func",
            vec![WasmType::I32, WasmType::Ptr],
            WasmType::I32,
        );

        registry.register(func.clone());
        assert_eq!(registry.len(), 1);

        let looked_up = registry.lookup("test_func");
        assert!(looked_up.is_some());
        assert_eq!(looked_up.unwrap(), &func);
    }

    #[test]
    fn test_lookup_nonexistent() {
        let registry = HostFunctionRegistry::new();
        assert!(registry.lookup("nonexistent").is_none());
    }

    #[test]
    fn test_standard_functions() {
        let registry = HostFunctionRegistry::with_standard_functions();

        // Should have the standard set of functions
        assert!(registry.len() > 0);

        // Check for key functions
        assert!(registry.lookup("hstringEquals").is_some());
        assert!(registry.lookup("hnewList").is_some());
        assert!(registry.lookup("hgetItem").is_some());
        assert!(registry.lookup("hsize").is_some());
        assert!(registry.lookup("happend").is_some());
        assert!(registry.lookup("hparseSchedule").is_some());
        assert!(registry.lookup("hscheduleString").is_some());
        assert!(registry.lookup("hround").is_some());
    }

    #[test]
    fn test_hstring_equals_signature() {
        let registry = HostFunctionRegistry::with_standard_functions();
        let func = registry.lookup("hstringEquals").unwrap();

        assert_eq!(func.name, "hstringEquals");
        assert_eq!(func.params, vec![WasmType::Ptr, WasmType::Ptr]);
        assert_eq!(func.return_type, WasmType::I32);
    }

    #[test]
    fn test_function_names() {
        let mut registry = HostFunctionRegistry::new();
        registry.register(HostFunctionDef::new("func1", vec![], WasmType::Void));
        registry.register(HostFunctionDef::new("func2", vec![], WasmType::Void));

        let names = registry.function_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"func1"));
        assert!(names.contains(&"func2"));
    }

    #[test]
    fn test_wasm_type_serialization() {
        let wasm_type = WasmType::I32;
        let json = serde_json::to_string(&wasm_type).unwrap();
        let deserialized: WasmType = serde_json::from_str(&json).unwrap();
        assert_eq!(wasm_type, deserialized);
    }
}
