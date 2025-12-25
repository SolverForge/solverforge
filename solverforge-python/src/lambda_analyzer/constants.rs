//! Class constant resolution for lambda analysis.
//!
//! This module handles Python class constants (class-level attributes with literal values).
//! When we see `self.FOO` or `cls.FOO`, we check if FOO is a class constant and inline
//! its value as a literal expression instead of creating a FieldAccess.
//!
//! # Example
//!
//! ```python
//! class Location:
//!     _AVERAGE_SPEED_KMPH = 50  # Class constant
//!
//!     def calculate_time(self):
//!         return distance / self._AVERAGE_SPEED_KMPH  # Should inline as IntLiteral(50)
//! ```

use pyo3::prelude::*;
use solverforge_core::wasm::Expression;

use super::registry::CLASS_REGISTRY;

/// Check if an attribute is a class constant and return its value as an Expression.
///
/// This checks the Python class to see if the attribute is:
/// 1. A class-level attribute (not instance field from __init__)
/// 2. A simple literal value (int, float, str, bool)
///
/// Returns None if it's an instance field, complex value, or not found.
///
/// # Arguments
/// * `py` - Python interpreter
/// * `class_name` - Name of the class (e.g., "Location")
/// * `attr_name` - Name of the attribute (e.g., "_AVERAGE_SPEED_KMPH")
pub fn get_class_constant(
    py: Python<'_>,
    class_name: &str,
    attr_name: &str,
) -> PyResult<Option<Expression>> {
    // Get the class from registry
    let class_ref: Option<Py<PyAny>> = {
        let registry = CLASS_REGISTRY.read().unwrap();
        if let Some(ref map) = *registry {
            map.get(class_name).map(|c| c.clone_ref(py))
        } else {
            None
        }
    };

    let class = match class_ref {
        Some(c) => c,
        None => {
            log::trace!(
                "Class '{}' not in registry for constant lookup of '{}'",
                class_name,
                attr_name
            );
            return Ok(None);
        }
    };

    let class_bound = class.bind(py);

    // Check if the attribute exists on the class
    let attr_value = match class_bound.getattr(attr_name) {
        Ok(val) => val,
        Err(_) => {
            log::trace!(
                "Attribute '{}' not found on class '{}'",
                attr_name,
                class_name
            );
            return Ok(None);
        }
    };

    // Skip if it's a callable (method), property, or other descriptor
    if attr_value.is_callable() {
        log::trace!(
            "Attribute '{}.{}' is callable, not a constant",
            class_name,
            attr_name
        );
        return Ok(None);
    }

    // Check if it's a property descriptor
    let inspect = py.import("inspect")?;
    let is_data_descriptor: bool = inspect
        .call_method1("isdatadescriptor", (&attr_value,))?
        .extract()?;
    if is_data_descriptor {
        log::trace!(
            "Attribute '{}.{}' is a data descriptor, not a constant",
            class_name,
            attr_name
        );
        return Ok(None);
    }

    // Check if this attribute is defined in __annotations__ (dataclass instance field)
    // Dataclass fields are instance fields, not class constants
    if let Ok(annotations) = class_bound.getattr("__annotations__") {
        if let Ok(contains) = annotations.call_method1("__contains__", (attr_name,)) {
            if contains.extract::<bool>().unwrap_or(false) {
                log::trace!(
                    "Attribute '{}.{}' is in __annotations__, treating as instance field",
                    class_name,
                    attr_name
                );
                return Ok(None);
            }
        }
    }

    // Now try to extract the literal value
    // Check for int first (before bool, since bool is subclass of int in Python)
    let builtins = py.import("builtins")?;
    let bool_type = builtins.getattr("bool")?;
    let int_type = builtins.getattr("int")?;
    let float_type = builtins.getattr("float")?;
    let str_type = builtins.getattr("str")?;

    // Check bool first (isinstance check, since True/False are ints too)
    if attr_value.is_instance(&bool_type)? {
        let value: bool = attr_value.extract()?;
        log::debug!(
            "Resolved class constant {}.{} = {} (bool)",
            class_name,
            attr_name,
            value
        );
        return Ok(Some(Expression::BoolLiteral { value }));
    }

    // Check int (after bool)
    if attr_value.is_instance(&int_type)? {
        let value: i64 = attr_value.extract()?;
        log::debug!(
            "Resolved class constant {}.{} = {} (int)",
            class_name,
            attr_name,
            value
        );
        return Ok(Some(Expression::IntLiteral { value }));
    }

    // Check float
    if attr_value.is_instance(&float_type)? {
        let value: f64 = attr_value.extract()?;
        log::debug!(
            "Resolved class constant {}.{} = {} (float)",
            class_name,
            attr_name,
            value
        );
        return Ok(Some(Expression::FloatLiteral { value }));
    }

    // Check str
    if attr_value.is_instance(&str_type)? {
        let value: String = attr_value.extract()?;
        log::debug!(
            "Resolved class constant {}.{} = \"{}\" (str)",
            class_name,
            attr_name,
            value
        );
        return Ok(Some(Expression::StringLiteral { value }));
    }

    log::trace!(
        "Attribute '{}.{}' has non-literal type, treating as instance field",
        class_name,
        attr_name
    );
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pyo3::types::PyDict;

    #[test]
    fn test_get_class_constant_int() {
        pyo3::Python::initialize();
        Python::attach(|py| {
            // Create a class with an int constant
            let locals = PyDict::new(py);
            py.run(
                c"
class TestClass:
    MY_CONSTANT = 42
",
                None,
                Some(&locals),
            )
            .unwrap();

            let class = locals.get_item("TestClass").unwrap().unwrap();
            super::super::register_class(py, "TestClass", &class);

            let result = get_class_constant(py, "TestClass", "MY_CONSTANT").unwrap();
            assert!(matches!(result, Some(Expression::IntLiteral { value: 42 })));
        });
    }

    #[test]
    fn test_get_class_constant_float() {
        pyo3::Python::initialize();
        Python::attach(|py| {
            let locals = PyDict::new(py);
            py.run(
                c"
class TestClass:
    PI = 3.14159
",
                None,
                Some(&locals),
            )
            .unwrap();

            let class = locals.get_item("TestClass").unwrap().unwrap();
            super::super::register_class(py, "TestClass", &class);

            let result = get_class_constant(py, "TestClass", "PI").unwrap();
            match result {
                Some(Expression::FloatLiteral { value }) => {
                    assert!((value - 3.14159).abs() < 0.0001);
                }
                _ => panic!("Expected FloatLiteral"),
            }
        });
    }

    #[test]
    fn test_get_class_constant_str() {
        pyo3::Python::initialize();
        Python::attach(|py| {
            let locals = PyDict::new(py);
            py.run(
                c"
class TestClass:
    NAME = 'hello'
",
                None,
                Some(&locals),
            )
            .unwrap();

            let class = locals.get_item("TestClass").unwrap().unwrap();
            super::super::register_class(py, "TestClass", &class);

            let result = get_class_constant(py, "TestClass", "NAME").unwrap();
            assert!(
                matches!(result, Some(Expression::StringLiteral { value }) if value == "hello")
            );
        });
    }

    #[test]
    fn test_get_class_constant_bool() {
        pyo3::Python::initialize();
        Python::attach(|py| {
            let locals = PyDict::new(py);
            py.run(
                c"
class TestClass:
    ENABLED = True
",
                None,
                Some(&locals),
            )
            .unwrap();

            let class = locals.get_item("TestClass").unwrap().unwrap();
            super::super::register_class(py, "TestClass", &class);

            let result = get_class_constant(py, "TestClass", "ENABLED").unwrap();
            assert!(matches!(
                result,
                Some(Expression::BoolLiteral { value: true })
            ));
        });
    }

    #[test]
    fn test_get_class_constant_skips_methods() {
        pyo3::Python::initialize();
        Python::attach(|py| {
            let locals = PyDict::new(py);
            py.run(
                c"
class TestClass:
    def my_method(self):
        return 42
",
                None,
                Some(&locals),
            )
            .unwrap();

            let class = locals.get_item("TestClass").unwrap().unwrap();
            super::super::register_class(py, "TestClass", &class);

            let result = get_class_constant(py, "TestClass", "my_method").unwrap();
            assert!(result.is_none());
        });
    }

    #[test]
    fn test_get_class_constant_skips_dataclass_fields() {
        pyo3::Python::initialize();
        Python::attach(|py| {
            let locals = PyDict::new(py);
            py.run(
                c"
from dataclasses import dataclass

@dataclass
class TestClass:
    my_field: int  # This is an instance field, not a class constant
    MY_CONSTANT: int = 42  # ClassVar would make this a constant, but annotation means instance
",
                None,
                Some(&locals),
            )
            .unwrap();

            let class = locals.get_item("TestClass").unwrap().unwrap();
            super::super::register_class(py, "TestClass", &class);

            // my_field is in __annotations__, should return None
            let result = get_class_constant(py, "TestClass", "my_field").unwrap();
            assert!(result.is_none());
        });
    }

    #[test]
    fn test_get_class_constant_unknown_class() {
        pyo3::Python::initialize();
        Python::attach(|py| {
            let result = get_class_constant(py, "NonExistentClass", "CONSTANT").unwrap();
            assert!(result.is_none());
        });
    }

    #[test]
    fn test_get_class_constant_unknown_attr() {
        pyo3::Python::initialize();
        Python::attach(|py| {
            let locals = PyDict::new(py);
            py.run(
                c"
class TestClass:
    pass
",
                None,
                Some(&locals),
            )
            .unwrap();

            let class = locals.get_item("TestClass").unwrap().unwrap();
            super::super::register_class(py, "TestClass", &class);

            let result = get_class_constant(py, "TestClass", "NON_EXISTENT").unwrap();
            assert!(result.is_none());
        });
    }
}
