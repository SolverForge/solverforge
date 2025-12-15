//! Python bindings for constraint streams.
//!
//! Constraint streams provide a fluent API for defining constraints.
//!
//! # Example
//!
//! ```python
//! @constraint_provider
//! def define_constraints(factory: ConstraintFactory):
//!     return [
//!         factory.for_each(Lesson)
//!             .filter(lambda lesson: lesson.room is None)
//!             .penalize(HardSoftScore.ONE_HARD)
//!             .as_constraint("Room required"),
//!     ]
//! ```

use pyo3::prelude::*;
use pyo3::types::PyType;
use solverforge_core::constraints::{Constraint, Joiner, StreamComponent};

use crate::joiners::PyJoiner;
use crate::lambda_analyzer::LambdaInfo;
use crate::score::{PyHardMediumSoftScore, PyHardSoftScore, PySimpleScore};

/// Factory for creating constraint streams.
#[pyclass(name = "ConstraintFactory")]
#[derive(Clone)]
pub struct PyConstraintFactory;

impl PyConstraintFactory {
    /// Create a new constraint factory (Rust API).
    pub fn create() -> Self {
        Self
    }

    /// Create a stream for a class by name (Rust API for tests).
    pub fn for_each_by_name(&self, class_name: &str) -> PyUniConstraintStream {
        PyUniConstraintStream::new(class_name.to_string(), false)
    }

    /// Create a unique pair stream by name (Rust API for tests).
    pub fn for_each_unique_pair_by_name(
        &self,
        class_name: &str,
        joiners: Vec<PyJoiner>,
    ) -> PyBiConstraintStream {
        PyBiConstraintStream::from_unique_pair(class_name.to_string(), joiners)
    }
}

#[pymethods]
impl PyConstraintFactory {
    #[new]
    fn new() -> Self {
        Self
    }

    /// Start a stream that matches every entity of the given class.
    fn for_each(&self, cls: &Bound<'_, PyType>) -> PyResult<PyUniConstraintStream> {
        let class_name: String = cls.getattr("__name__")?.extract()?;
        Ok(PyUniConstraintStream::new(class_name, false))
    }

    /// Start a stream that matches every entity including unassigned ones.
    fn for_each_including_unassigned(
        &self,
        cls: &Bound<'_, PyType>,
    ) -> PyResult<PyUniConstraintStream> {
        let class_name: String = cls.getattr("__name__")?.extract()?;
        Ok(PyUniConstraintStream::new(class_name, true))
    }

    /// Start a stream that matches every unique pair of entities.
    #[pyo3(signature = (cls, *joiners))]
    fn for_each_unique_pair(
        &self,
        cls: &Bound<'_, PyType>,
        joiners: Vec<PyJoiner>,
    ) -> PyResult<PyBiConstraintStream> {
        let class_name: String = cls.getattr("__name__")?.extract()?;
        Ok(PyBiConstraintStream::from_unique_pair(class_name, joiners))
    }

    fn __repr__(&self) -> &'static str {
        "ConstraintFactory()"
    }
}

/// A constraint stream with a single entity type.
#[pyclass(name = "UniConstraintStream")]
#[derive(Clone)]
pub struct PyUniConstraintStream {
    components: Vec<StreamComponent>,
    class_name: String,
    /// Stored predicates for later analysis.
    predicates: Vec<LambdaInfo>,
}

impl PyUniConstraintStream {
    /// Create a new stream (public for tests).
    pub fn new(class_name: String, include_unassigned: bool) -> Self {
        let component = if include_unassigned {
            StreamComponent::ForEachIncludingUnassigned {
                class_name: class_name.clone(),
            }
        } else {
            StreamComponent::ForEach {
                class_name: class_name.clone(),
            }
        };
        Self {
            components: vec![component],
            class_name,
            predicates: Vec::new(),
        }
    }

    /// Get stored predicates for analysis.
    #[allow(dead_code)]
    pub fn predicates(&self) -> &[LambdaInfo] {
        &self.predicates
    }

    /// Penalize with a weight and return a constraint (Rust API for tests).
    pub fn penalize_weight(&self, name: &str, weight: i32) -> PyConstraint {
        let weight_str = format!("{}hard", weight);
        let mut components = self.components.clone();
        components.push(StreamComponent::Penalize {
            weight: weight_str,
            scale_by: None,
        });
        PyConstraint {
            inner: Constraint::new(name).with_components(components),
        }
    }

    /// Reward with a weight and return a constraint (Rust API for tests).
    pub fn reward_weight(&self, name: &str, weight: i32) -> PyConstraint {
        let weight_str = format!("{}soft", weight);
        let mut components = self.components.clone();
        components.push(StreamComponent::Reward {
            weight: weight_str,
            scale_by: None,
        });
        PyConstraint {
            inner: Constraint::new(name).with_components(components),
        }
    }

    /// Filter entities based on a predicate (Rust API for tests).
    pub fn filter_with(&self, py: Python<'_>, predicate: Py<PyAny>) -> PyResult<Self> {
        // Analyze the predicate lambda with class hint
        let mut lambda_info = LambdaInfo::new(py, predicate, "filter")?;
        lambda_info = lambda_info.with_class_hint(&self.class_name);

        let wasm_func = lambda_info.to_wasm_function();

        let mut components = self.components.clone();
        components.push(StreamComponent::Filter {
            predicate: wasm_func,
        });

        let mut predicates = self.predicates.clone();
        predicates.push(lambda_info);

        Ok(Self {
            components,
            class_name: self.class_name.clone(),
            predicates,
        })
    }
}

#[pymethods]
impl PyUniConstraintStream {
    /// Filter entities based on a predicate.
    ///
    /// # Arguments
    /// * `predicate` - A lambda that takes an entity and returns a boolean
    ///
    /// # Example
    /// ```python
    /// stream.filter(lambda lesson: lesson.room is not None)
    /// ```
    fn filter(&self, py: Python<'_>, predicate: Py<PyAny>) -> PyResult<Self> {
        // Analyze the predicate lambda with class hint
        let mut lambda_info = LambdaInfo::new(py, predicate, "filter")?;
        lambda_info = lambda_info.with_class_hint(&self.class_name);

        let wasm_func = lambda_info.to_wasm_function();

        let mut components = self.components.clone();
        components.push(StreamComponent::Filter {
            predicate: wasm_func,
        });

        let mut predicates = self.predicates.clone();
        predicates.push(lambda_info);

        Ok(Self {
            components,
            class_name: self.class_name.clone(),
            predicates,
        })
    }

    /// Join with another entity type.
    #[pyo3(signature = (cls, *joiners))]
    fn join(
        &self,
        cls: &Bound<'_, PyType>,
        joiners: Vec<PyJoiner>,
    ) -> PyResult<PyBiConstraintStream> {
        let join_class_name: String = cls.getattr("__name__")?.extract()?;
        let rust_joiners: Vec<Joiner> = joiners.into_iter().map(|j| j.to_rust()).collect();

        let mut components = self.components.clone();
        components.push(StreamComponent::Join {
            class_name: join_class_name,
            joiners: rust_joiners,
        });

        Ok(PyBiConstraintStream {
            components,
            class_names: vec![self.class_name.clone()],
            predicates: Vec::new(),
        })
    }

    /// Filter if another entity exists matching the joiners.
    #[pyo3(signature = (cls, *joiners))]
    fn if_exists(&self, cls: &Bound<'_, PyType>, joiners: Vec<PyJoiner>) -> PyResult<Self> {
        let other_class_name: String = cls.getattr("__name__")?.extract()?;
        let rust_joiners: Vec<Joiner> = joiners.into_iter().map(|j| j.to_rust()).collect();

        let mut components = self.components.clone();
        components.push(StreamComponent::IfExists {
            class_name: other_class_name,
            joiners: rust_joiners,
        });

        Ok(Self {
            components,
            class_name: self.class_name.clone(),
            predicates: self.predicates.clone(),
        })
    }

    /// Filter if no other entity exists matching the joiners.
    #[pyo3(signature = (cls, *joiners))]
    fn if_not_exists(&self, cls: &Bound<'_, PyType>, joiners: Vec<PyJoiner>) -> PyResult<Self> {
        let other_class_name: String = cls.getattr("__name__")?.extract()?;
        let rust_joiners: Vec<Joiner> = joiners.into_iter().map(|j| j.to_rust()).collect();

        let mut components = self.components.clone();
        components.push(StreamComponent::IfNotExists {
            class_name: other_class_name,
            joiners: rust_joiners,
        });

        Ok(Self {
            components,
            class_name: self.class_name.clone(),
            predicates: self.predicates.clone(),
        })
    }

    /// Penalize matches with a simple score.
    fn penalize_simple(&self, score: &PySimpleScore) -> PyUniConstraintBuilder {
        let weight = format!("{}", score.to_rust());
        let mut components = self.components.clone();
        components.push(StreamComponent::Penalize {
            weight,
            scale_by: None,
        });

        PyUniConstraintBuilder { components }
    }

    /// Penalize matches with a hard/soft score.
    fn penalize(&self, score: &PyHardSoftScore) -> PyUniConstraintBuilder {
        let weight = format!("{}", score.to_rust());
        let mut components = self.components.clone();
        components.push(StreamComponent::Penalize {
            weight,
            scale_by: None,
        });

        PyUniConstraintBuilder { components }
    }

    /// Penalize matches with a hard/medium/soft score.
    fn penalize_hms(&self, score: &PyHardMediumSoftScore) -> PyUniConstraintBuilder {
        let weight = format!("{}", score.to_rust());
        let mut components = self.components.clone();
        components.push(StreamComponent::Penalize {
            weight,
            scale_by: None,
        });

        PyUniConstraintBuilder { components }
    }

    /// Reward matches with a hard/soft score.
    fn reward(&self, score: &PyHardSoftScore) -> PyUniConstraintBuilder {
        let weight = format!("{}", score.to_rust());
        let mut components = self.components.clone();
        components.push(StreamComponent::Reward {
            weight,
            scale_by: None,
        });

        PyUniConstraintBuilder { components }
    }

    fn __repr__(&self) -> String {
        format!(
            "UniConstraintStream(class='{}', components={})",
            self.class_name,
            self.components.len()
        )
    }
}

/// A constraint stream with two entity types.
#[pyclass(name = "BiConstraintStream")]
#[derive(Clone)]
pub struct PyBiConstraintStream {
    components: Vec<StreamComponent>,
    class_names: Vec<String>,
    /// Stored predicates for later analysis.
    predicates: Vec<LambdaInfo>,
}

impl PyBiConstraintStream {
    /// Create from unique pair (public for tests).
    pub fn from_unique_pair(class_name: String, joiners: Vec<PyJoiner>) -> Self {
        let rust_joiners: Vec<Joiner> = joiners.into_iter().map(|j| j.to_rust()).collect();
        let component = StreamComponent::ForEachUniquePair {
            class_name: class_name.clone(),
            joiners: rust_joiners,
        };
        Self {
            components: vec![component],
            class_names: vec![class_name],
            predicates: Vec::new(),
        }
    }

    /// Get stored predicates for analysis.
    #[allow(dead_code)]
    pub fn predicates(&self) -> &[LambdaInfo] {
        &self.predicates
    }

    /// Penalize with a weight and return a constraint (Rust API for tests).
    pub fn penalize_weight(&self, name: &str, weight: i32) -> PyConstraint {
        let weight_str = format!("{}hard", weight);
        let mut components = self.components.clone();
        components.push(StreamComponent::Penalize {
            weight: weight_str,
            scale_by: None,
        });
        PyConstraint {
            inner: Constraint::new(name).with_components(components),
        }
    }

    /// Reward with a weight and return a constraint (Rust API for tests).
    pub fn reward_weight(&self, name: &str, weight: i32) -> PyConstraint {
        let weight_str = format!("{}soft", weight);
        let mut components = self.components.clone();
        components.push(StreamComponent::Reward {
            weight: weight_str,
            scale_by: None,
        });
        PyConstraint {
            inner: Constraint::new(name).with_components(components),
        }
    }

    /// Filter pairs based on a predicate (Rust API for tests).
    pub fn filter_with(&self, py: Python<'_>, predicate: Py<PyAny>) -> PyResult<Self> {
        // Analyze the predicate lambda
        let lambda_info = LambdaInfo::new(py, predicate, "filter_bi")?;
        let wasm_func = lambda_info.to_wasm_function();

        let mut components = self.components.clone();
        components.push(StreamComponent::Filter {
            predicate: wasm_func,
        });

        let mut predicates = self.predicates.clone();
        predicates.push(lambda_info);

        Ok(Self {
            components,
            class_names: self.class_names.clone(),
            predicates,
        })
    }
}

#[pymethods]
impl PyBiConstraintStream {
    /// Filter pairs based on a predicate.
    ///
    /// # Arguments
    /// * `predicate` - A lambda that takes two entities and returns a boolean
    ///
    /// # Example
    /// ```python
    /// stream.filter(lambda a, b: a.room != b.room)
    /// ```
    fn filter(&self, py: Python<'_>, predicate: Py<PyAny>) -> PyResult<Self> {
        // Analyze the predicate lambda
        let lambda_info = LambdaInfo::new(py, predicate, "filter_bi")?;
        let wasm_func = lambda_info.to_wasm_function();

        let mut components = self.components.clone();
        components.push(StreamComponent::Filter {
            predicate: wasm_func,
        });

        let mut predicates = self.predicates.clone();
        predicates.push(lambda_info);

        Ok(Self {
            components,
            class_names: self.class_names.clone(),
            predicates,
        })
    }

    /// Penalize matches with a hard/soft score.
    fn penalize(&self, score: &PyHardSoftScore) -> PyBiConstraintBuilder {
        let weight = format!("{}", score.to_rust());
        let mut components = self.components.clone();
        components.push(StreamComponent::Penalize {
            weight,
            scale_by: None,
        });

        PyBiConstraintBuilder { components }
    }

    /// Reward matches with a hard/soft score.
    fn reward(&self, score: &PyHardSoftScore) -> PyBiConstraintBuilder {
        let weight = format!("{}", score.to_rust());
        let mut components = self.components.clone();
        components.push(StreamComponent::Reward {
            weight,
            scale_by: None,
        });

        PyBiConstraintBuilder { components }
    }

    fn __repr__(&self) -> String {
        format!(
            "BiConstraintStream(classes={:?}, components={})",
            self.class_names,
            self.components.len()
        )
    }
}

/// Builder for finalizing a uni-constraint.
#[pyclass(name = "UniConstraintBuilder")]
#[derive(Clone)]
pub struct PyUniConstraintBuilder {
    components: Vec<StreamComponent>,
}

#[pymethods]
impl PyUniConstraintBuilder {
    /// Finalize the constraint with a name.
    fn as_constraint(&self, name: &str) -> PyConstraint {
        PyConstraint {
            inner: Constraint::new(name).with_components(self.components.clone()),
        }
    }

    fn __repr__(&self) -> String {
        format!("UniConstraintBuilder(components={})", self.components.len())
    }
}

/// Builder for finalizing a bi-constraint.
#[pyclass(name = "BiConstraintBuilder")]
#[derive(Clone)]
pub struct PyBiConstraintBuilder {
    components: Vec<StreamComponent>,
}

#[pymethods]
impl PyBiConstraintBuilder {
    /// Finalize the constraint with a name.
    fn as_constraint(&self, name: &str) -> PyConstraint {
        PyConstraint {
            inner: Constraint::new(name).with_components(self.components.clone()),
        }
    }

    fn __repr__(&self) -> String {
        format!("BiConstraintBuilder(components={})", self.components.len())
    }
}

/// A finalized constraint.
#[pyclass(name = "Constraint")]
#[derive(Clone)]
pub struct PyConstraint {
    inner: Constraint,
}

#[pymethods]
impl PyConstraint {
    /// Get the constraint name (Python getter).
    #[getter]
    fn get_name(&self) -> &str {
        &self.inner.name
    }

    /// Get the number of stream components.
    fn component_count(&self) -> usize {
        self.inner.components.len()
    }

    /// Get the JSON representation.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "Constraint(name='{}', components={})",
            self.inner.name,
            self.inner.components.len()
        )
    }
}

impl PyConstraint {
    pub fn from_rust(inner: Constraint) -> Self {
        Self { inner }
    }

    pub fn to_rust(&self) -> Constraint {
        self.inner.clone()
    }

    /// Get the constraint name (Rust API).
    pub fn name(&self) -> &str {
        &self.inner.name
    }
}

/// Register stream classes with the Python module.
pub fn register_streams(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyConstraintFactory>()?;
    m.add_class::<PyUniConstraintStream>()?;
    m.add_class::<PyBiConstraintStream>()?;
    m.add_class::<PyUniConstraintBuilder>()?;
    m.add_class::<PyBiConstraintBuilder>()?;
    m.add_class::<PyConstraint>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pyo3::types::PyDict;

    fn init_python() {
        pyo3::prepare_freethreaded_python();
    }

    #[test]
    fn test_constraint_factory_creation() {
        let factory = PyConstraintFactory::new();
        assert_eq!(factory.__repr__(), "ConstraintFactory()");
    }

    #[test]
    fn test_uni_constraint_builder() {
        let stream = PyUniConstraintStream::new("Lesson".to_string(), false);
        assert!(stream.__repr__().contains("Lesson"));
        assert!(stream.__repr__().contains("components=1"));
    }

    #[test]
    fn test_constraint_to_json() {
        let constraint = PyConstraint {
            inner: Constraint::new("Test constraint"),
        };
        let json = constraint.to_json().unwrap();
        assert!(json.contains("Test constraint"));
    }

    #[test]
    fn test_uni_stream_filter_stores_predicate() {
        init_python();
        Python::with_gil(|py| {
            let locals = PyDict::new(py);
            py.run(c"f = lambda x: x.room is not None", None, Some(&locals))
                .unwrap();
            let func = locals.get_item("f").unwrap().unwrap();

            let stream = PyUniConstraintStream::new("Lesson".to_string(), false);
            let filtered = stream.filter(py, func.unbind()).unwrap();

            // Should have 2 components: ForEach + Filter
            assert_eq!(filtered.components.len(), 2);

            // Should have 1 predicate stored
            assert_eq!(filtered.predicates().len(), 1);

            // Predicate name should start with "filter_"
            assert!(filtered.predicates()[0].name.starts_with("filter_"));
        });
    }

    #[test]
    fn test_bi_stream_filter_stores_predicate() {
        init_python();
        Python::with_gil(|py| {
            let locals = PyDict::new(py);
            py.run(c"f = lambda a, b: a.id != b.id", None, Some(&locals))
                .unwrap();
            let func = locals.get_item("f").unwrap().unwrap();

            let stream = PyBiConstraintStream::from_unique_pair("Lesson".to_string(), vec![]);
            let filtered = stream.filter(py, func.unbind()).unwrap();

            // Should have 2 components: ForEachUniquePair + Filter
            assert_eq!(filtered.components.len(), 2);

            // Should have 1 predicate stored
            assert_eq!(filtered.predicates().len(), 1);

            // Predicate name should start with "filter_bi_"
            assert!(filtered.predicates()[0].name.starts_with("filter_bi_"));
        });
    }

    #[test]
    fn test_filter_unique_names() {
        init_python();
        Python::with_gil(|py| {
            let locals = PyDict::new(py);
            py.run(c"f = lambda x: x.room is not None", None, Some(&locals))
                .unwrap();
            let func = locals.get_item("f").unwrap().unwrap();

            let stream = PyUniConstraintStream::new("Lesson".to_string(), false);
            let filtered1 = stream.filter(py, func.clone().unbind()).unwrap();
            let filtered2 = filtered1.filter(py, func.unbind()).unwrap();

            // Should have unique names for each filter
            assert_ne!(
                filtered2.predicates()[0].name,
                filtered2.predicates()[1].name
            );
        });
    }
}
