//! Type inference for Python domain classes.
//!
//! This module provides functions to infer types from Python domain classes
//! using type hints and annotations. It works with the class registry to
//! look up field types and register discovered classes.

use super::registry::{register_class, CLASS_REGISTRY};
use pyo3::prelude::*;
use solverforge_core::wasm::Expression;

/// Infer the class type of an expression.
/// For FieldAccess, looks up the field type from the parent class.
/// Returns the class name if determinable.
pub(crate) fn infer_expression_class(
    py: Python<'_>,
    expr: &Expression,
    default_class: &str,
) -> PyResult<Option<String>> {
    match expr {
        Expression::FieldAccess {
            class_name,
            field_name,
            ..
        } => {
            // Look up the field type from the class
            get_field_type_and_register(py, class_name, field_name)
        }
        Expression::Param { index } if *index == 0 => {
            // Parameter 0 is typically self, use the default class
            Ok(Some(default_class.to_string()))
        }
        _ => Ok(None),
    }
}

/// Recursively extract the concrete class name and type from a possibly nested generic type.
///
/// Handles patterns like:
/// - `SomeClass` -> ("SomeClass", SomeClass)
/// - `Optional[SomeClass]` -> ("SomeClass", SomeClass)
/// - `ClassVar[Optional[SomeClass]]` -> ("SomeClass", SomeClass)
/// - `list[SomeClass]` -> ("SomeClass", SomeClass)
fn extract_concrete_class_from_type<'py>(
    field_type: &Bound<'py, PyAny>,
) -> Option<(String, Bound<'py, PyAny>)> {
    // First check if it's a generic type with __origin__ (like Optional[X], ClassVar[X], list[X])
    // We must check this BEFORE __name__ because Optional[X] has __name__ = "Optional"
    if field_type.getattr("__origin__").is_ok() {
        // It's a generic type - look at __args__ to find the concrete inner type
        if let Ok(args) = field_type.getattr("__args__") {
            if let Ok(args_tuple) = args.cast::<pyo3::types::PyTuple>() {
                for arg in args_tuple.iter() {
                    // Skip NoneType args (from Optional = Union[T, None])
                    if let Ok(arg_name) = arg.getattr("__name__") {
                        if let Ok(name) = arg_name.extract::<String>() {
                            if name == "NoneType" {
                                continue;
                            }
                        }
                    }
                    // Recursively try to extract from this arg
                    if let Some(result) = extract_concrete_class_from_type(&arg) {
                        return Some(result);
                    }
                }
            }
        }
        return None;
    }

    // No __origin__, check if it's a simple class with __name__
    if let Ok(type_name) = field_type.getattr("__name__") {
        if let Ok(name) = type_name.extract::<String>() {
            // Skip NoneType
            if name != "NoneType" {
                log::debug!("Found concrete class: {}", name);
                return Some((name, field_type.clone()));
            }
        }
    }

    None
}

/// Look up a field's type from a class and register it if found.
/// Returns the class name of the field type.
pub(crate) fn get_field_type_and_register(
    py: Python<'_>,
    class_name: &str,
    field_name: &str,
) -> PyResult<Option<String>> {
    log::debug!(
        "get_field_type_and_register: looking up {}.{}",
        class_name,
        field_name
    );

    // Clone class reference while holding lock, then release before Python operations
    let class_ref: Option<Py<PyAny>> = {
        let registry = CLASS_REGISTRY.read().unwrap();
        if let Some(ref map) = *registry {
            map.get(class_name).map(|c| c.clone_ref(py))
        } else {
            None
        }
    };

    // Now do Python operations without holding the lock
    let field_info: Option<(String, Py<PyAny>)> = if let Some(class) = class_ref {
        let class_bound = class.bind(py);

        // Get type hints from the class
        if let Ok(get_type_hints) = py
            .import("typing")
            .and_then(|m| m.getattr("get_type_hints"))
        {
            if let Ok(hints) = get_type_hints.call1((&class_bound,)) {
                if let Ok(field_type) = hints.get_item(field_name) {
                    log::debug!(
                        "Found field type for {}.{}: {:?}",
                        class_name,
                        field_name,
                        field_type
                    );
                    // Use recursive extraction to handle nested generics
                    if let Some((name, inner_type)) = extract_concrete_class_from_type(&field_type)
                    {
                        log::debug!("Extracted concrete type: {}", name);
                        Some((name, inner_type.unbind()))
                    } else {
                        None
                    }
                } else {
                    log::debug!("Field {} not found in hints for {}", field_name, class_name);
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        log::debug!("Class {} not in registry", class_name);
        None
    };

    // Register the discovered class outside the read lock
    if let Some((ref field_class_name, ref field_class)) = field_info {
        let field_bound = field_class.bind(py);
        register_class(py, field_class_name, field_bound);
        log::debug!("Registered field class: {}", field_class_name);
        return Ok(Some(field_class_name.clone()));
    }

    Ok(None)
}

/// Infer the item type from a collection expression using Python type hints.
///
/// For a FieldAccess like Param(0).visits on class Vehicle, this inspects
/// the type hints of the Vehicle class to determine the item type (e.g., Visit).
pub(crate) fn infer_item_type(py: Python<'_>, collection_expr: &Expression) -> PyResult<String> {
    match collection_expr {
        Expression::FieldAccess {
            object: _,
            class_name,
            field_name,
        } => {
            // Clone class reference while holding lock, then release before Python operations
            let class_ref: Option<Py<PyAny>> = {
                let registry = CLASS_REGISTRY.read().unwrap();
                if let Some(ref map) = *registry {
                    map.get(class_name).map(|c| c.clone_ref(py))
                } else {
                    None
                }
            };

            // Now do Python operations without holding the lock
            let item_info: Option<(String, Py<PyAny>)> = if let Some(class) = &class_ref {
                let class_bound = class.bind(py);

                // Get type hints from the class
                if let Ok(get_type_hints) = py
                    .import("typing")
                    .and_then(|m| m.getattr("get_type_hints"))
                {
                    if let Ok(hints) = get_type_hints.call1((&class_bound,)) {
                        if let Ok(field_type) = hints.get_item(field_name) {
                            // field_type is something like typing.List[Visit]
                            // Extract the inner type
                            if let Ok(args) = field_type.getattr("__args__") {
                                if let Ok(args_len) = args.len() {
                                    if args_len > 0 {
                                        if let Ok(item_type) = args.get_item(0) {
                                            if let Ok(item_name) = item_type.getattr("__name__") {
                                                if let Ok(item_class_name) =
                                                    item_name.extract::<String>()
                                                {
                                                    Some((
                                                        item_class_name,
                                                        item_type.clone().unbind(),
                                                    ))
                                                } else {
                                                    None
                                                }
                                            } else {
                                                None
                                            }
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            // Register the discovered class (outside the read lock)
            if let Some((ref item_class_name, ref item_class)) = item_info {
                let item_bound = item_class.bind(py);
                register_class(py, item_class_name, item_bound);
                return Ok(item_class_name.clone());
            }

            // If type hints don't work, try field annotations on the instance
            let fallback_info: Option<(String, Option<Py<PyAny>>)> = if let Some(class) = &class_ref
            {
                let class_bound = class.bind(py);

                // Try __annotations__ directly
                if let Ok(annotations) = class_bound.getattr("__annotations__") {
                    if let Ok(field_type) = annotations.get_item(field_name) {
                        // Try to get __name__ from the type (simple class reference)
                        if let Ok(item_name) = field_type.getattr("__name__") {
                            if let Ok(item_class_name) = item_name.extract::<String>() {
                                Some((item_class_name, Some(field_type.clone().unbind())))
                            } else {
                                None
                            }
                        // Try to extract from typing generic
                        } else if let Ok(_origin) = field_type.getattr("__origin__") {
                            if let Ok(args) = field_type.getattr("__args__") {
                                if let Ok(args_len) = args.len() {
                                    if args_len > 0 {
                                        if let Ok(item_type) = args.get_item(0) {
                                            if let Ok(item_name) = item_type.getattr("__name__") {
                                                if let Ok(item_class_name) =
                                                    item_name.extract::<String>()
                                                {
                                                    Some((
                                                        item_class_name,
                                                        Some(item_type.clone().unbind()),
                                                    ))
                                                } else {
                                                    None
                                                }
                                            } else {
                                                None
                                            }
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            if let Some((ref item_class_name, ref maybe_class)) = fallback_info {
                if let Some(ref item_class) = maybe_class {
                    let item_bound = item_class.bind(py);
                    register_class(py, item_class_name, item_bound);
                }
                return Ok(item_class_name.clone());
            }

            Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Cannot infer item type for field '{}.{}' - ensure it has type hints",
                class_name, field_name
            )))
        }
        _ => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "Cannot infer item type from complex collection expression",
        )),
    }
}

/// Create a sum expression over a collection.
///
/// Constructs a Sum expression with:
/// - collection: The collection being iterated
/// - item_var_name: Name of the loop variable
/// - item_param_index: The parameter index assigned to the loop variable
/// - accumulator_expr: Expression being accumulated (uses loop variable as Param with item_param_index)
pub(crate) fn create_sum_over_collection(
    accumulated: Expression,
    loop_var: &str,
    collection: Expression,
    loop_var_param_index: u32,
    item_class_name: &str,
) -> Expression {
    Expression::Sum {
        collection: Box::new(collection),
        item_var_name: loop_var.to_string(),
        item_param_index: loop_var_param_index,
        item_class_name: item_class_name.to_string(),
        accumulator_expr: Box::new(accumulated),
    }
}
