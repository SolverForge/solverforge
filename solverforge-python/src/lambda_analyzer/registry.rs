//! Class registry for Python domain class introspection.
//!
//! This module provides registration and lookup of Python classes decorated with
//! @planning_entity or @planning_solution, enabling method body analysis for inlining.

use pyo3::prelude::*;
use std::collections::HashMap;
use std::sync::RwLock;

/// Global registry for domain classes that can be introspected.
///
/// This stores references to Python classes decorated with @planning_entity
/// or @planning_solution, enabling method body analysis for inlining.
pub(crate) static CLASS_REGISTRY: RwLock<Option<HashMap<String, Py<PyAny>>>> = RwLock::new(None);

/// Register a Python class for method introspection.
///
/// Called by @planning_entity and @planning_solution decorators.
pub fn register_class(py: Python<'_>, class_name: &str, class: &Bound<'_, PyAny>) {
    let mut registry = CLASS_REGISTRY.write().unwrap();
    if registry.is_none() {
        *registry = Some(HashMap::new());
    }
    if let Some(ref mut map) = *registry {
        map.insert(class_name.to_string(), class.clone().unbind());
        log::debug!("Registered class '{}' for method introspection", class_name);
    }
    drop(registry);

    // Also store on the class itself for access during solving
    let _ = class.setattr("__solverforge_class_name__", class_name);
    let _ = py; // suppress unused warning
}

/// Look up a method from a registered domain class.
///
/// Returns the method object if found, or None if the class/method doesn't exist.
///
/// # Arguments
/// * `py` - Python interpreter
/// * `class_name` - Name of the class (e.g., "Vehicle")
/// * `method_name` - Name of the method (e.g., "calculate_total_demand")
///
/// # Returns
/// * `Some(Py<PyAny>)` - The method callable if found
/// * `None` - If class or method not found
pub fn get_method_from_class(
    py: Python<'_>,
    class_name: &str,
    method_name: &str,
) -> Option<Py<PyAny>> {
    let registry = CLASS_REGISTRY.read().unwrap();

    if let Some(ref map) = *registry {
        if let Some(class) = map.get(class_name) {
            let class_bound = class.bind(py);

            // Try to get the method from the class
            if let Ok(method) = class_bound.getattr(method_name) {
                // Check if it's actually a method/function (not a class attribute)
                let inspect = py.import("inspect").ok()?;
                let is_method = inspect
                    .call_method1("isfunction", (&method,))
                    .ok()?
                    .extract::<bool>()
                    .ok()?;
                let is_method_descriptor = inspect
                    .call_method1("ismethod", (&method,))
                    .ok()?
                    .extract::<bool>()
                    .ok()?;

                if is_method || is_method_descriptor {
                    log::debug!("Found method '{}' on class '{}'", method_name, class_name);
                    return Some(method.unbind());
                }
            }
        }
    }

    log::debug!(
        "Method '{}' not found on class '{}' in registry",
        method_name,
        class_name
    );
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use pyo3::types::PyDict;

    fn init_python() {
        pyo3::Python::initialize();
    }

    #[test]
    fn test_register_and_get_method_from_class() {
        init_python();
        Python::attach(|py| {
            // Define a simple class with a method
            let locals = PyDict::new(py);
            py.run(
                c"class Vehicle:\n    def get_capacity(self):\n        return self.capacity",
                None,
                Some(&locals),
            )
            .unwrap();
            let vehicle_class = locals.get_item("Vehicle").unwrap().unwrap();

            // Register the class
            register_class(py, "Vehicle", &vehicle_class);

            // Look up the method
            let method = get_method_from_class(py, "Vehicle", "get_capacity");
            assert!(method.is_some());
        });
    }

    #[test]
    fn test_get_method_from_unregistered_class() {
        init_python();
        Python::attach(|py| {
            // Should return None for unregistered class
            let method = get_method_from_class(py, "UnknownClass", "some_method");
            assert!(method.is_none());
        });
    }

    #[test]
    fn test_get_nonexistent_method() {
        init_python();
        Python::attach(|py| {
            // Define a class without the method we'll look for
            let locals = PyDict::new(py);
            py.run(c"class Vehicle:\n    capacity = 100", None, Some(&locals))
                .unwrap();
            let vehicle_class = locals.get_item("Vehicle").unwrap().unwrap();

            register_class(py, "Vehicle", &vehicle_class);

            // Should return None for non-existent method
            let method = get_method_from_class(py, "Vehicle", "nonexistent_method");
            assert!(method.is_none());
        });
    }
}
