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

use crate::collectors::PyCollector;
use crate::joiners::PyJoiner;
use crate::lambda_analyzer::LambdaInfo;
use crate::score::{
    PyHardMediumSoftDecimalScore, PyHardMediumSoftScore, PyHardSoftDecimalScore, PyHardSoftScore,
    PySimpleScore,
};

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

    /// Group items by a key and aggregate with a collector.
    ///
    /// # Arguments
    /// * `key_mapper` - A lambda that extracts the grouping key
    /// * `collector` - A collector that aggregates the grouped items
    ///
    /// # Returns
    /// A BiConstraintStream with (key, aggregated_value) tuples
    ///
    /// # Example
    /// ```python
    /// factory.for_each(Shift)
    ///     .group_by(lambda shift: shift.employee, ConstraintCollectors.count())
    ///     .filter(lambda employee, count: count > 5)
    ///     .penalize(HardSoftScore.ONE_HARD)
    ///     .as_constraint("Too many shifts")
    /// ```
    fn group_by(
        &self,
        py: Python<'_>,
        key_mapper: Py<PyAny>,
        collector: &PyCollector,
    ) -> PyResult<PyBiConstraintStream> {
        let key_info = LambdaInfo::new(py, key_mapper, "group_by_key")?;
        let key_wasm = key_info.to_wasm_function();

        let mut components = self.components.clone();
        components.push(StreamComponent::GroupBy {
            keys: vec![key_wasm],
            aggregators: vec![collector.to_rust()],
        });

        Ok(PyBiConstraintStream {
            components,
            class_names: vec![self.class_name.clone()],
            predicates: Vec::new(),
        })
    }

    /// Group items with only a collector (no key).
    ///
    /// # Arguments
    /// * `collector` - A collector that aggregates all items
    ///
    /// # Returns
    /// A UniConstraintStream with the aggregated value
    ///
    /// # Example
    /// ```python
    /// factory.for_each(Shift)
    ///     .group_by_collector(ConstraintCollectors.count())
    ///     .filter(lambda count: count > 100)
    ///     .penalize(HardSoftScore.ONE_HARD)
    ///     .as_constraint("Too many shifts total")
    /// ```
    fn group_by_collector(&self, collector: &PyCollector) -> PyUniConstraintStream {
        let mut components = self.components.clone();
        components.push(StreamComponent::GroupBy {
            keys: vec![],
            aggregators: vec![collector.to_rust()],
        });

        PyUniConstraintStream {
            components,
            class_name: "".to_string(), // No longer entity-based
            predicates: Vec::new(),
        }
    }

    /// Group items by two keys and aggregate with a collector.
    ///
    /// # Arguments
    /// * `key_mapper_a` - First key extractor
    /// * `key_mapper_b` - Second key extractor
    /// * `collector` - A collector that aggregates the grouped items
    ///
    /// # Returns
    /// A TriConstraintStream with (key_a, key_b, aggregated_value) tuples
    fn group_by_two_keys(
        &self,
        py: Python<'_>,
        key_mapper_a: Py<PyAny>,
        key_mapper_b: Py<PyAny>,
        collector: &PyCollector,
    ) -> PyResult<PyTriConstraintStream> {
        let key_a_info = LambdaInfo::new(py, key_mapper_a, "group_by_key_a")?;
        let key_b_info = LambdaInfo::new(py, key_mapper_b, "group_by_key_b")?;

        let mut components = self.components.clone();
        components.push(StreamComponent::GroupBy {
            keys: vec![key_a_info.to_wasm_function(), key_b_info.to_wasm_function()],
            aggregators: vec![collector.to_rust()],
        });

        Ok(PyTriConstraintStream {
            components,
            class_names: vec![self.class_name.clone()],
            predicates: Vec::new(),
        })
    }

    /// Flatten the last element of each tuple using a mapping function.
    ///
    /// Takes each element and applies a mapping that turns it into an Iterable,
    /// then flattens to create one tuple per item in the iterable.
    ///
    /// # Arguments
    /// * `flattening_function` - Function that extracts an iterable from each element
    ///
    /// # Returns
    /// A UniConstraintStream with one tuple per flattened item
    fn flatten_last(
        &self,
        py: Python<'_>,
        flattening_function: Py<PyAny>,
    ) -> PyResult<PyUniConstraintStream> {
        let lambda_info = LambdaInfo::new(py, flattening_function, "flatten_last")?;
        let mut components = self.components.clone();
        components.push(StreamComponent::FlattenLast {
            map: Some(lambda_info.to_wasm_function()),
        });
        Ok(PyUniConstraintStream {
            components,
            class_name: self.class_name.clone(),
            predicates: Vec::new(),
        })
    }

    /// Add to the stream all instances of a class not yet present in it.
    ///
    /// Adds entities that are not matched by the stream, useful for including
    /// unassigned entities in constraints.
    ///
    /// # Arguments
    /// * `cls` - The class of instances to complement with
    ///
    /// # Returns
    /// A UniConstraintStream including the complement
    fn complement(&self, cls: &Bound<'_, PyType>) -> PyResult<PyUniConstraintStream> {
        let class_name = cls
            .name()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?
            .to_string();
        let mut components = self.components.clone();
        components.push(StreamComponent::Complement { class_name });
        Ok(PyUniConstraintStream {
            components,
            class_name: self.class_name.clone(),
            predicates: Vec::new(),
        })
    }

    /// Penalize matches with a simple score.
    #[pyo3(signature = (score, match_weigher=None))]
    fn penalize_simple(
        &self,
        py: Python<'_>,
        score: &PySimpleScore,
        match_weigher: Option<Py<PyAny>>,
    ) -> PyResult<PyUniConstraintBuilder> {
        let weight = format!("{}", score.to_rust());
        let mut components = self.components.clone();

        let scale_by = if let Some(weigher) = match_weigher {
            let lambda_info = LambdaInfo::new(py, weigher, "match_weigher")
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
            Some(lambda_info.to_wasm_function())
        } else {
            None
        };

        components.push(StreamComponent::Penalize { weight, scale_by });

        Ok(PyUniConstraintBuilder { components })
    }

    /// Penalize matches with a hard/soft score.
    ///
    /// Args:
    ///     score: Base penalty score (HardSoftScore)
    ///     match_weigher: Optional lambda to calculate penalty weight per match
    #[pyo3(signature = (score, match_weigher=None))]
    fn penalize(
        &self,
        py: Python<'_>,
        score: &PyHardSoftScore,
        match_weigher: Option<Py<PyAny>>,
    ) -> PyResult<PyUniConstraintBuilder> {
        let weight = format!("{}", score.to_rust());
        let mut components = self.components.clone();

        let scale_by = if let Some(weigher) = match_weigher {
            let lambda_info = LambdaInfo::new(py, weigher, "match_weigher")
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
            Some(lambda_info.to_wasm_function())
        } else {
            None
        };

        components.push(StreamComponent::Penalize { weight, scale_by });

        Ok(PyUniConstraintBuilder { components })
    }

    /// Penalize matches with a hard/medium/soft score.
    #[pyo3(signature = (score, match_weigher=None))]
    fn penalize_hms(
        &self,
        py: Python<'_>,
        score: &PyHardMediumSoftScore,
        match_weigher: Option<Py<PyAny>>,
    ) -> PyResult<PyUniConstraintBuilder> {
        let weight = format!("{}", score.to_rust());
        let mut components = self.components.clone();

        let scale_by = if let Some(weigher) = match_weigher {
            let lambda_info = LambdaInfo::new(py, weigher, "match_weigher")
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
            Some(lambda_info.to_wasm_function())
        } else {
            None
        };

        components.push(StreamComponent::Penalize { weight, scale_by });

        Ok(PyUniConstraintBuilder { components })
    }

    /// Penalize matches with a hard/soft decimal score.
    #[pyo3(signature = (score, match_weigher=None))]
    fn penalize_decimal(
        &self,
        py: Python<'_>,
        score: &PyHardSoftDecimalScore,
        match_weigher: Option<Py<PyAny>>,
    ) -> PyResult<PyUniConstraintBuilder> {
        let weight = score.to_string_repr();
        let mut components = self.components.clone();

        let scale_by = if let Some(weigher) = match_weigher {
            let lambda_info = LambdaInfo::new(py, weigher, "match_weigher")
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
            Some(lambda_info.to_wasm_function())
        } else {
            None
        };

        components.push(StreamComponent::Penalize { weight, scale_by });

        Ok(PyUniConstraintBuilder { components })
    }

    /// Penalize matches with a hard/medium/soft decimal score.
    #[pyo3(signature = (score, match_weigher=None))]
    fn penalize_hms_decimal(
        &self,
        py: Python<'_>,
        score: &PyHardMediumSoftDecimalScore,
        match_weigher: Option<Py<PyAny>>,
    ) -> PyResult<PyUniConstraintBuilder> {
        let weight = score.to_string_repr();
        let mut components = self.components.clone();

        let scale_by = if let Some(weigher) = match_weigher {
            let lambda_info = LambdaInfo::new(py, weigher, "match_weigher")
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
            Some(lambda_info.to_wasm_function())
        } else {
            None
        };

        components.push(StreamComponent::Penalize { weight, scale_by });

        Ok(PyUniConstraintBuilder { components })
    }

    /// Reward matches with a hard/soft score.
    #[pyo3(signature = (score, match_weigher=None))]
    fn reward(
        &self,
        py: Python<'_>,
        score: &PyHardSoftScore,
        match_weigher: Option<Py<PyAny>>,
    ) -> PyResult<PyUniConstraintBuilder> {
        let weight = format!("{}", score.to_rust());
        let mut components = self.components.clone();

        let scale_by = if let Some(weigher) = match_weigher {
            let lambda_info = LambdaInfo::new(py, weigher, "match_weigher")
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
            Some(lambda_info.to_wasm_function())
        } else {
            None
        };

        components.push(StreamComponent::Reward { weight, scale_by });

        Ok(PyUniConstraintBuilder { components })
    }

    /// Reward matches with a hard/soft decimal score.
    #[pyo3(signature = (score, match_weigher=None))]
    fn reward_decimal(
        &self,
        py: Python<'_>,
        score: &PyHardSoftDecimalScore,
        match_weigher: Option<Py<PyAny>>,
    ) -> PyResult<PyUniConstraintBuilder> {
        let weight = score.to_string_repr();
        let mut components = self.components.clone();

        let scale_by = if let Some(weigher) = match_weigher {
            let lambda_info = LambdaInfo::new(py, weigher, "match_weigher")
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
            Some(lambda_info.to_wasm_function())
        } else {
            None
        };

        components.push(StreamComponent::Reward { weight, scale_by });

        Ok(PyUniConstraintBuilder { components })
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

    /// Join with another entity type.
    ///
    /// # Arguments
    /// * `cls` - The class to join with
    /// * `joiners` - Optional joiners to filter the join
    ///
    /// # Example
    /// ```python
    /// factory.for_each(Lesson)
    ///     .join(Room)
    ///     .join(Timeslot)  # Creates a TriConstraintStream
    /// ```
    #[pyo3(signature = (cls, *joiners))]
    fn join(
        &self,
        cls: &Bound<'_, PyType>,
        joiners: Vec<PyJoiner>,
    ) -> PyResult<PyTriConstraintStream> {
        let join_class_name: String = cls.getattr("__name__")?.extract()?;
        let rust_joiners: Vec<Joiner> = joiners.into_iter().map(|j| j.to_rust()).collect();

        let mut components = self.components.clone();
        components.push(StreamComponent::Join {
            class_name: join_class_name.clone(),
            joiners: rust_joiners,
        });

        let mut class_names = self.class_names.clone();
        class_names.push(join_class_name);

        Ok(PyTriConstraintStream {
            components,
            class_names,
            predicates: Vec::new(),
        })
    }

    /// Group pairs by a key extracted from both entities and aggregate with a collector.
    ///
    /// # Arguments
    /// * `key_mapper` - A lambda that takes two entities and extracts a grouping key
    /// * `collector` - A collector that aggregates the grouped pairs
    ///
    /// # Returns
    /// A BiConstraintStream with (key, aggregated_value) tuples
    ///
    /// # Example
    /// ```python
    /// factory.for_each_unique_pair(Lesson)
    ///     .group_by(lambda a, b: (a.room, b.room), ConstraintCollectors.count())
    ///     .penalize(HardSoftScore.ONE_HARD)
    ///     .as_constraint("Room pair conflicts")
    /// ```
    fn group_by(
        &self,
        py: Python<'_>,
        key_mapper: Py<PyAny>,
        collector: &PyCollector,
    ) -> PyResult<PyBiConstraintStream> {
        let key_info = LambdaInfo::new(py, key_mapper, "group_by_bi_key")?;
        let key_wasm = key_info.to_wasm_function();

        let mut components = self.components.clone();
        components.push(StreamComponent::GroupBy {
            keys: vec![key_wasm],
            aggregators: vec![collector.to_rust()],
        });

        Ok(PyBiConstraintStream {
            components,
            class_names: self.class_names.clone(),
            predicates: Vec::new(),
        })
    }

    /// Group pairs with only a collector (no key).
    ///
    /// # Arguments
    /// * `collector` - A collector that aggregates all pairs
    ///
    /// # Returns
    /// A UniConstraintStream with the aggregated value
    fn group_by_collector(&self, collector: &PyCollector) -> PyUniConstraintStream {
        let mut components = self.components.clone();
        components.push(StreamComponent::GroupBy {
            keys: vec![],
            aggregators: vec![collector.to_rust()],
        });

        PyUniConstraintStream {
            components,
            class_name: "".to_string(),
            predicates: Vec::new(),
        }
    }

    /// Flatten the last element of each tuple using a mapping function.
    ///
    /// Takes the second element of each pair and applies a mapping that turns
    /// it into an Iterable, then flattens to create one tuple per item.
    ///
    /// # Arguments
    /// * `flattening_function` - Function that extracts an iterable from B
    ///
    /// # Returns
    /// A BiConstraintStream with one tuple per flattened item
    fn flatten_last(
        &self,
        py: Python<'_>,
        flattening_function: Py<PyAny>,
    ) -> PyResult<PyBiConstraintStream> {
        let lambda_info = LambdaInfo::new(py, flattening_function, "flatten_last")?;
        let mut components = self.components.clone();
        components.push(StreamComponent::FlattenLast {
            map: Some(lambda_info.to_wasm_function()),
        });
        Ok(PyBiConstraintStream {
            components,
            class_names: self.class_names.clone(),
            predicates: Vec::new(),
        })
    }

    /// Add to the stream all instances of a class not yet present in it.
    ///
    /// # Arguments
    /// * `cls` - The class of instances to complement with
    /// * `padding` - Optional function to generate the B value for complemented instances
    #[pyo3(signature = (cls, padding=None))]
    fn complement(
        &self,
        py: Python<'_>,
        cls: &Bound<'_, PyType>,
        padding: Option<Py<PyAny>>,
    ) -> PyResult<PyBiConstraintStream> {
        let class_name = cls
            .name()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?
            .to_string();
        let mut components = self.components.clone();
        components.push(StreamComponent::Complement { class_name });

        // If padding is provided, add a map component for it
        if let Some(pad_fn) = padding {
            let lambda_info = LambdaInfo::new(py, pad_fn, "complement_padding")?;
            components.push(StreamComponent::Map {
                mappers: vec![lambda_info.to_wasm_function()],
            });
        }

        Ok(PyBiConstraintStream {
            components,
            class_names: self.class_names.clone(),
            predicates: Vec::new(),
        })
    }

    /// Penalize matches with a hard/soft score.
    #[pyo3(signature = (score, match_weigher=None))]
    fn penalize(
        &self,
        py: Python<'_>,
        score: &PyHardSoftScore,
        match_weigher: Option<Py<PyAny>>,
    ) -> PyResult<PyBiConstraintBuilder> {
        let weight = format!("{}", score.to_rust());
        let mut components = self.components.clone();

        let scale_by = if let Some(weigher) = match_weigher {
            let lambda_info = LambdaInfo::new(py, weigher, "match_weigher")
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
            Some(lambda_info.to_wasm_function())
        } else {
            None
        };

        components.push(StreamComponent::Penalize { weight, scale_by });

        Ok(PyBiConstraintBuilder { components })
    }

    /// Reward matches with a hard/soft score.
    #[pyo3(signature = (score, match_weigher=None))]
    fn reward(
        &self,
        py: Python<'_>,
        score: &PyHardSoftScore,
        match_weigher: Option<Py<PyAny>>,
    ) -> PyResult<PyBiConstraintBuilder> {
        let weight = format!("{}", score.to_rust());
        let mut components = self.components.clone();

        let scale_by = if let Some(weigher) = match_weigher {
            let lambda_info = LambdaInfo::new(py, weigher, "match_weigher")
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
            Some(lambda_info.to_wasm_function())
        } else {
            None
        };

        components.push(StreamComponent::Reward { weight, scale_by });

        Ok(PyBiConstraintBuilder { components })
    }

    /// Penalize matches with a hard/soft decimal score.
    #[pyo3(signature = (score, match_weigher=None))]
    fn penalize_decimal(
        &self,
        py: Python<'_>,
        score: &PyHardSoftDecimalScore,
        match_weigher: Option<Py<PyAny>>,
    ) -> PyResult<PyBiConstraintBuilder> {
        let weight = score.to_string_repr();
        let mut components = self.components.clone();

        let scale_by = if let Some(weigher) = match_weigher {
            let lambda_info = LambdaInfo::new(py, weigher, "match_weigher")
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
            Some(lambda_info.to_wasm_function())
        } else {
            None
        };

        components.push(StreamComponent::Penalize { weight, scale_by });

        Ok(PyBiConstraintBuilder { components })
    }

    /// Penalize matches with a hard/medium/soft decimal score.
    #[pyo3(signature = (score, match_weigher=None))]
    fn penalize_hms_decimal(
        &self,
        py: Python<'_>,
        score: &PyHardMediumSoftDecimalScore,
        match_weigher: Option<Py<PyAny>>,
    ) -> PyResult<PyBiConstraintBuilder> {
        let weight = score.to_string_repr();
        let mut components = self.components.clone();

        let scale_by = if let Some(weigher) = match_weigher {
            let lambda_info = LambdaInfo::new(py, weigher, "match_weigher")
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
            Some(lambda_info.to_wasm_function())
        } else {
            None
        };

        components.push(StreamComponent::Penalize { weight, scale_by });

        Ok(PyBiConstraintBuilder { components })
    }

    /// Reward matches with a hard/soft decimal score.
    #[pyo3(signature = (score, match_weigher=None))]
    fn reward_decimal(
        &self,
        py: Python<'_>,
        score: &PyHardSoftDecimalScore,
        match_weigher: Option<Py<PyAny>>,
    ) -> PyResult<PyBiConstraintBuilder> {
        let weight = score.to_string_repr();
        let mut components = self.components.clone();

        let scale_by = if let Some(weigher) = match_weigher {
            let lambda_info = LambdaInfo::new(py, weigher, "match_weigher")
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
            Some(lambda_info.to_wasm_function())
        } else {
            None
        };

        components.push(StreamComponent::Reward { weight, scale_by });

        Ok(PyBiConstraintBuilder { components })
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

/// A constraint stream with three entity types.
#[pyclass(name = "TriConstraintStream")]
#[derive(Clone)]
pub struct PyTriConstraintStream {
    components: Vec<StreamComponent>,
    class_names: Vec<String>,
    /// Stored predicates for later analysis.
    predicates: Vec<LambdaInfo>,
}

impl PyTriConstraintStream {
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

    /// Filter triples based on a predicate (Rust API for tests).
    pub fn filter_with(&self, py: Python<'_>, predicate: Py<PyAny>) -> PyResult<Self> {
        let lambda_info = LambdaInfo::new(py, predicate, "filter_tri")?;
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
impl PyTriConstraintStream {
    /// Filter triples based on a predicate.
    ///
    /// # Arguments
    /// * `predicate` - A lambda that takes three entities and returns a boolean
    ///
    /// # Example
    /// ```python
    /// stream.filter(lambda a, b, c: a.room != b.room and b.room != c.room)
    /// ```
    fn filter(&self, py: Python<'_>, predicate: Py<PyAny>) -> PyResult<Self> {
        let lambda_info = LambdaInfo::new(py, predicate, "filter_tri")?;
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

    /// Flatten the last element of each tuple using a mapping function.
    ///
    /// Takes the third element of each triple and applies a mapping that turns
    /// it into an Iterable, then flattens to create one tuple per item.
    ///
    /// # Arguments
    /// * `flattening_function` - Function that extracts an iterable from C
    ///
    /// # Returns
    /// A TriConstraintStream with one tuple per flattened item
    fn flatten_last(
        &self,
        py: Python<'_>,
        flattening_function: Py<PyAny>,
    ) -> PyResult<PyTriConstraintStream> {
        let lambda_info = LambdaInfo::new(py, flattening_function, "flatten_last")?;
        let mut components = self.components.clone();
        components.push(StreamComponent::FlattenLast {
            map: Some(lambda_info.to_wasm_function()),
        });
        Ok(PyTriConstraintStream {
            components,
            class_names: self.class_names.clone(),
            predicates: Vec::new(),
        })
    }

    /// Add to the stream all instances of a class not yet present in it.
    ///
    /// # Arguments
    /// * `cls` - The class of instances to complement with
    /// * `padding_b` - Optional function to generate the B value for complemented instances
    /// * `padding_c` - Optional function to generate the C value for complemented instances
    #[pyo3(signature = (cls, padding_b=None, padding_c=None))]
    fn complement(
        &self,
        py: Python<'_>,
        cls: &Bound<'_, PyType>,
        padding_b: Option<Py<PyAny>>,
        padding_c: Option<Py<PyAny>>,
    ) -> PyResult<PyTriConstraintStream> {
        let class_name = cls
            .name()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?
            .to_string();
        let mut components = self.components.clone();
        components.push(StreamComponent::Complement { class_name });

        // If padding functions are provided, add map components for them
        let mut mappers = Vec::new();
        if let Some(pad_fn) = padding_b {
            let lambda_info = LambdaInfo::new(py, pad_fn, "complement_padding_b")?;
            mappers.push(lambda_info.to_wasm_function());
        }
        if let Some(pad_fn) = padding_c {
            let lambda_info = LambdaInfo::new(py, pad_fn, "complement_padding_c")?;
            mappers.push(lambda_info.to_wasm_function());
        }
        if !mappers.is_empty() {
            components.push(StreamComponent::Map { mappers });
        }

        Ok(PyTriConstraintStream {
            components,
            class_names: self.class_names.clone(),
            predicates: Vec::new(),
        })
    }

    /// Penalize matches with a hard/soft score.
    #[pyo3(signature = (score, match_weigher=None))]
    fn penalize(
        &self,
        py: Python<'_>,
        score: &PyHardSoftScore,
        match_weigher: Option<Py<PyAny>>,
    ) -> PyResult<PyTriConstraintBuilder> {
        let weight = format!("{}", score.to_rust());
        let mut components = self.components.clone();

        let scale_by = if let Some(weigher) = match_weigher {
            let lambda_info = LambdaInfo::new(py, weigher, "match_weigher")
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
            Some(lambda_info.to_wasm_function())
        } else {
            None
        };

        components.push(StreamComponent::Penalize { weight, scale_by });

        Ok(PyTriConstraintBuilder { components })
    }

    /// Reward matches with a hard/soft score.
    #[pyo3(signature = (score, match_weigher=None))]
    fn reward(
        &self,
        py: Python<'_>,
        score: &PyHardSoftScore,
        match_weigher: Option<Py<PyAny>>,
    ) -> PyResult<PyTriConstraintBuilder> {
        let weight = format!("{}", score.to_rust());
        let mut components = self.components.clone();

        let scale_by = if let Some(weigher) = match_weigher {
            let lambda_info = LambdaInfo::new(py, weigher, "match_weigher")
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
            Some(lambda_info.to_wasm_function())
        } else {
            None
        };

        components.push(StreamComponent::Reward { weight, scale_by });

        Ok(PyTriConstraintBuilder { components })
    }

    /// Penalize matches with a hard/soft decimal score.
    #[pyo3(signature = (score, match_weigher=None))]
    fn penalize_decimal(
        &self,
        py: Python<'_>,
        score: &PyHardSoftDecimalScore,
        match_weigher: Option<Py<PyAny>>,
    ) -> PyResult<PyTriConstraintBuilder> {
        let weight = score.to_string_repr();
        let mut components = self.components.clone();

        let scale_by = if let Some(weigher) = match_weigher {
            let lambda_info = LambdaInfo::new(py, weigher, "match_weigher")
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
            Some(lambda_info.to_wasm_function())
        } else {
            None
        };

        components.push(StreamComponent::Penalize { weight, scale_by });

        Ok(PyTriConstraintBuilder { components })
    }

    /// Penalize matches with a hard/medium/soft decimal score.
    #[pyo3(signature = (score, match_weigher=None))]
    fn penalize_hms_decimal(
        &self,
        py: Python<'_>,
        score: &PyHardMediumSoftDecimalScore,
        match_weigher: Option<Py<PyAny>>,
    ) -> PyResult<PyTriConstraintBuilder> {
        let weight = score.to_string_repr();
        let mut components = self.components.clone();

        let scale_by = if let Some(weigher) = match_weigher {
            let lambda_info = LambdaInfo::new(py, weigher, "match_weigher")
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
            Some(lambda_info.to_wasm_function())
        } else {
            None
        };

        components.push(StreamComponent::Penalize { weight, scale_by });

        Ok(PyTriConstraintBuilder { components })
    }

    /// Reward matches with a hard/soft decimal score.
    #[pyo3(signature = (score, match_weigher=None))]
    fn reward_decimal(
        &self,
        py: Python<'_>,
        score: &PyHardSoftDecimalScore,
        match_weigher: Option<Py<PyAny>>,
    ) -> PyResult<PyTriConstraintBuilder> {
        let weight = score.to_string_repr();
        let mut components = self.components.clone();

        let scale_by = if let Some(weigher) = match_weigher {
            let lambda_info = LambdaInfo::new(py, weigher, "match_weigher")
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
            Some(lambda_info.to_wasm_function())
        } else {
            None
        };

        components.push(StreamComponent::Reward { weight, scale_by });

        Ok(PyTriConstraintBuilder { components })
    }

    fn __repr__(&self) -> String {
        format!(
            "TriConstraintStream(classes={:?}, components={})",
            self.class_names,
            self.components.len()
        )
    }
}

/// Builder for finalizing a tri-constraint.
#[pyclass(name = "TriConstraintBuilder")]
#[derive(Clone)]
pub struct PyTriConstraintBuilder {
    components: Vec<StreamComponent>,
}

#[pymethods]
impl PyTriConstraintBuilder {
    /// Finalize the constraint with a name.
    fn as_constraint(&self, name: &str) -> PyConstraint {
        PyConstraint {
            inner: Constraint::new(name).with_components(self.components.clone()),
        }
    }

    fn __repr__(&self) -> String {
        format!("TriConstraintBuilder(components={})", self.components.len())
    }
}

/// A constraint stream with four entity types.
#[pyclass(name = "QuadConstraintStream")]
#[derive(Clone)]
pub struct PyQuadConstraintStream {
    components: Vec<StreamComponent>,
    class_names: Vec<String>,
    predicates: Vec<LambdaInfo>,
}

impl PyQuadConstraintStream {
    #[allow(dead_code)]
    pub fn predicates(&self) -> &[LambdaInfo] {
        &self.predicates
    }
}

#[pymethods]
impl PyQuadConstraintStream {
    /// Filter quads based on a predicate.
    fn filter(&self, py: Python<'_>, predicate: Py<PyAny>) -> PyResult<Self> {
        let lambda_info = LambdaInfo::new(py, predicate, "filter_quad")?;
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

    /// Flatten the last element of each tuple using a mapping function.
    fn flatten_last(
        &self,
        py: Python<'_>,
        flattening_function: Py<PyAny>,
    ) -> PyResult<PyQuadConstraintStream> {
        let lambda_info = LambdaInfo::new(py, flattening_function, "flatten_last")?;
        let mut components = self.components.clone();
        components.push(StreamComponent::FlattenLast {
            map: Some(lambda_info.to_wasm_function()),
        });
        Ok(PyQuadConstraintStream {
            components,
            class_names: self.class_names.clone(),
            predicates: Vec::new(),
        })
    }

    /// Add to the stream all instances of a class not yet present in it.
    #[pyo3(signature = (cls, padding_b=None, padding_c=None, padding_d=None))]
    fn complement(
        &self,
        py: Python<'_>,
        cls: &Bound<'_, PyType>,
        padding_b: Option<Py<PyAny>>,
        padding_c: Option<Py<PyAny>>,
        padding_d: Option<Py<PyAny>>,
    ) -> PyResult<PyQuadConstraintStream> {
        let class_name = cls
            .name()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?
            .to_string();
        let mut components = self.components.clone();
        components.push(StreamComponent::Complement { class_name });

        let mut mappers = Vec::new();
        if let Some(pad_fn) = padding_b {
            let lambda_info = LambdaInfo::new(py, pad_fn, "complement_padding_b")?;
            mappers.push(lambda_info.to_wasm_function());
        }
        if let Some(pad_fn) = padding_c {
            let lambda_info = LambdaInfo::new(py, pad_fn, "complement_padding_c")?;
            mappers.push(lambda_info.to_wasm_function());
        }
        if let Some(pad_fn) = padding_d {
            let lambda_info = LambdaInfo::new(py, pad_fn, "complement_padding_d")?;
            mappers.push(lambda_info.to_wasm_function());
        }
        if !mappers.is_empty() {
            components.push(StreamComponent::Map { mappers });
        }

        Ok(PyQuadConstraintStream {
            components,
            class_names: self.class_names.clone(),
            predicates: Vec::new(),
        })
    }

    /// Penalize matches with a hard/soft score.
    #[pyo3(signature = (score, match_weigher=None))]
    fn penalize(
        &self,
        py: Python<'_>,
        score: &PyHardSoftScore,
        match_weigher: Option<Py<PyAny>>,
    ) -> PyResult<PyQuadConstraintBuilder> {
        let weight = format!("{}", score.to_rust());
        let mut components = self.components.clone();

        let scale_by = if let Some(weigher) = match_weigher {
            let lambda_info = LambdaInfo::new(py, weigher, "match_weigher")
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
            Some(lambda_info.to_wasm_function())
        } else {
            None
        };

        components.push(StreamComponent::Penalize { weight, scale_by });

        Ok(PyQuadConstraintBuilder { components })
    }

    /// Reward matches with a hard/soft score.
    #[pyo3(signature = (score, match_weigher=None))]
    fn reward(
        &self,
        py: Python<'_>,
        score: &PyHardSoftScore,
        match_weigher: Option<Py<PyAny>>,
    ) -> PyResult<PyQuadConstraintBuilder> {
        let weight = format!("{}", score.to_rust());
        let mut components = self.components.clone();

        let scale_by = if let Some(weigher) = match_weigher {
            let lambda_info = LambdaInfo::new(py, weigher, "match_weigher")
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
            Some(lambda_info.to_wasm_function())
        } else {
            None
        };

        components.push(StreamComponent::Reward { weight, scale_by });

        Ok(PyQuadConstraintBuilder { components })
    }

    /// Penalize matches with a hard/soft decimal score.
    #[pyo3(signature = (score, match_weigher=None))]
    fn penalize_decimal(
        &self,
        py: Python<'_>,
        score: &PyHardSoftDecimalScore,
        match_weigher: Option<Py<PyAny>>,
    ) -> PyResult<PyQuadConstraintBuilder> {
        let weight = score.to_string_repr();
        let mut components = self.components.clone();

        let scale_by = if let Some(weigher) = match_weigher {
            let lambda_info = LambdaInfo::new(py, weigher, "match_weigher")
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
            Some(lambda_info.to_wasm_function())
        } else {
            None
        };

        components.push(StreamComponent::Penalize { weight, scale_by });

        Ok(PyQuadConstraintBuilder { components })
    }

    /// Reward matches with a hard/soft decimal score.
    #[pyo3(signature = (score, match_weigher=None))]
    fn reward_decimal(
        &self,
        py: Python<'_>,
        score: &PyHardSoftDecimalScore,
        match_weigher: Option<Py<PyAny>>,
    ) -> PyResult<PyQuadConstraintBuilder> {
        let weight = score.to_string_repr();
        let mut components = self.components.clone();

        let scale_by = if let Some(weigher) = match_weigher {
            let lambda_info = LambdaInfo::new(py, weigher, "match_weigher")
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
            Some(lambda_info.to_wasm_function())
        } else {
            None
        };

        components.push(StreamComponent::Reward { weight, scale_by });

        Ok(PyQuadConstraintBuilder { components })
    }

    fn __repr__(&self) -> String {
        format!(
            "QuadConstraintStream(classes={:?}, components={})",
            self.class_names,
            self.components.len()
        )
    }
}

/// Builder for finalizing a quad-constraint.
#[pyclass(name = "QuadConstraintBuilder")]
#[derive(Clone)]
pub struct PyQuadConstraintBuilder {
    components: Vec<StreamComponent>,
}

#[pymethods]
impl PyQuadConstraintBuilder {
    /// Finalize the constraint with a name.
    fn as_constraint(&self, name: &str) -> PyConstraint {
        PyConstraint {
            inner: Constraint::new(name).with_components(self.components.clone()),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "QuadConstraintBuilder(components={})",
            self.components.len()
        )
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
    m.add_class::<PyTriConstraintStream>()?;
    m.add_class::<PyQuadConstraintStream>()?;
    m.add_class::<PyUniConstraintBuilder>()?;
    m.add_class::<PyBiConstraintBuilder>()?;
    m.add_class::<PyTriConstraintBuilder>()?;
    m.add_class::<PyQuadConstraintBuilder>()?;
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

    #[test]
    fn test_bi_stream_join_to_tri() {
        init_python();
        Python::with_gil(|py| {
            let locals = PyDict::new(py);
            // Create a mock class
            py.run(c"class Timeslot:\n    pass", None, Some(&locals))
                .unwrap();
            let timeslot_cls = locals.get_item("Timeslot").unwrap().unwrap();

            let bi_stream = PyBiConstraintStream::from_unique_pair("Lesson".to_string(), vec![]);
            let tri_stream = bi_stream
                .join(timeslot_cls.downcast::<PyType>().unwrap(), vec![])
                .unwrap();

            // Should have 2 components: ForEachUniquePair + Join
            assert_eq!(tri_stream.components.len(), 2);

            // Should have 2 class names tracked (original + joined)
            assert_eq!(tri_stream.class_names.len(), 2);
            assert_eq!(tri_stream.class_names[0], "Lesson");
            assert_eq!(tri_stream.class_names[1], "Timeslot");

            // Repr should show TriConstraintStream
            assert!(tri_stream.__repr__().contains("TriConstraintStream"));
        });
    }

    #[test]
    fn test_tri_stream_filter_stores_predicate() {
        init_python();
        Python::with_gil(|py| {
            let locals = PyDict::new(py);
            py.run(
                c"class Room:\n    pass\nf = lambda a, b, c: a.id != b.id and b.id != c.id",
                None,
                Some(&locals),
            )
            .unwrap();
            let room_cls = locals.get_item("Room").unwrap().unwrap();
            let func = locals.get_item("f").unwrap().unwrap();

            // Create bi stream, then join to get tri stream
            let bi_stream = PyBiConstraintStream::from_unique_pair("Lesson".to_string(), vec![]);
            let tri_stream = bi_stream
                .join(room_cls.downcast::<PyType>().unwrap(), vec![])
                .unwrap();
            let filtered = tri_stream.filter(py, func.unbind()).unwrap();

            // Should have 3 components: ForEachUniquePair + Join + Filter
            assert_eq!(filtered.components.len(), 3);

            // Should have 1 predicate stored
            assert_eq!(filtered.predicates().len(), 1);

            // Predicate name should start with "filter_tri_"
            assert!(filtered.predicates()[0].name.starts_with("filter_tri_"));
        });
    }

    #[test]
    fn test_tri_stream_penalize_and_constraint() {
        init_python();
        Python::with_gil(|py| {
            let locals = PyDict::new(py);
            py.run(c"class Room:\n    pass", None, Some(&locals))
                .unwrap();
            let room_cls = locals.get_item("Room").unwrap().unwrap();

            // Create tri stream
            let bi_stream = PyBiConstraintStream::from_unique_pair("Lesson".to_string(), vec![]);
            let tri_stream = bi_stream
                .join(room_cls.downcast::<PyType>().unwrap(), vec![])
                .unwrap();

            // Use penalize_weight (Rust API)
            let constraint = tri_stream.penalize_weight("Triple conflict", 1);

            assert_eq!(constraint.name(), "Triple conflict");
            assert!(constraint.to_json().unwrap().contains("1hard"));
        });
    }

    #[test]
    fn test_tri_stream_repr() {
        init_python();
        Python::with_gil(|py| {
            let locals = PyDict::new(py);
            py.run(c"class Room:\n    pass", None, Some(&locals))
                .unwrap();
            let room_cls = locals.get_item("Room").unwrap().unwrap();

            let bi_stream = PyBiConstraintStream::from_unique_pair("Lesson".to_string(), vec![]);
            let tri_stream = bi_stream
                .join(room_cls.downcast::<PyType>().unwrap(), vec![])
                .unwrap();

            let repr = tri_stream.__repr__();
            assert!(repr.contains("TriConstraintStream"));
            assert!(repr.contains("Lesson"));
            assert!(repr.contains("Room"));
            assert!(repr.contains("components=2"));
        });
    }

    #[test]
    fn test_uni_stream_group_by() {
        init_python();
        Python::with_gil(|py| {
            let locals = PyDict::new(py);
            py.run(
                c"key_mapper = lambda shift: shift.employee",
                None,
                Some(&locals),
            )
            .unwrap();
            let key_mapper = locals.get_item("key_mapper").unwrap().unwrap();

            let stream = PyUniConstraintStream::new("Shift".to_string(), false);
            let collector = crate::collectors::PyConstraintCollectors::count_rust();
            let grouped = stream
                .group_by(py, key_mapper.unbind(), &collector)
                .unwrap();

            // Should have 2 components: ForEach + GroupBy
            assert_eq!(grouped.components.len(), 2);

            // Should be a BiConstraintStream
            assert!(grouped.__repr__().contains("BiConstraintStream"));
        });
    }

    #[test]
    fn test_uni_stream_group_by_collector_only() {
        init_python();
        Python::with_gil(|_py| {
            let stream = PyUniConstraintStream::new("Shift".to_string(), false);
            let collector = crate::collectors::PyConstraintCollectors::count_rust();
            let grouped = stream.group_by_collector(&collector);

            // Should have 2 components: ForEach + GroupBy
            assert_eq!(grouped.components.len(), 2);

            // Should be a UniConstraintStream
            assert!(grouped.__repr__().contains("UniConstraintStream"));
        });
    }

    #[test]
    fn test_uni_stream_group_by_two_keys() {
        init_python();
        Python::with_gil(|py| {
            let locals = PyDict::new(py);
            py.run(
                c"key_a = lambda s: s.employee\nkey_b = lambda s: s.day",
                None,
                Some(&locals),
            )
            .unwrap();
            let key_a = locals.get_item("key_a").unwrap().unwrap();
            let key_b = locals.get_item("key_b").unwrap().unwrap();

            let stream = PyUniConstraintStream::new("Shift".to_string(), false);
            let collector = crate::collectors::PyConstraintCollectors::count_rust();
            let grouped = stream
                .group_by_two_keys(py, key_a.unbind(), key_b.unbind(), &collector)
                .unwrap();

            // Should have 2 components: ForEach + GroupBy
            assert_eq!(grouped.components.len(), 2);

            // Should be a TriConstraintStream
            assert!(grouped.__repr__().contains("TriConstraintStream"));
        });
    }

    #[test]
    fn test_bi_stream_group_by() {
        init_python();
        Python::with_gil(|py| {
            let locals = PyDict::new(py);
            py.run(c"key_mapper = lambda a, b: a.room", None, Some(&locals))
                .unwrap();
            let key_mapper = locals.get_item("key_mapper").unwrap().unwrap();

            let stream = PyBiConstraintStream::from_unique_pair("Lesson".to_string(), vec![]);
            let collector = crate::collectors::PyConstraintCollectors::count_rust();
            let grouped = stream
                .group_by(py, key_mapper.unbind(), &collector)
                .unwrap();

            // Should have 2 components: ForEachUniquePair + GroupBy
            assert_eq!(grouped.components.len(), 2);

            // Should be a BiConstraintStream
            assert!(grouped.__repr__().contains("BiConstraintStream"));
        });
    }

    #[test]
    fn test_bi_stream_group_by_collector_only() {
        init_python();
        Python::with_gil(|_py| {
            let stream = PyBiConstraintStream::from_unique_pair("Lesson".to_string(), vec![]);
            let collector = crate::collectors::PyConstraintCollectors::count_rust();
            let grouped = stream.group_by_collector(&collector);

            // Should have 2 components: ForEachUniquePair + GroupBy
            assert_eq!(grouped.components.len(), 2);

            // Should be a UniConstraintStream
            assert!(grouped.__repr__().contains("UniConstraintStream"));
        });
    }

    #[test]
    fn test_group_by_with_sum_collector() {
        init_python();
        Python::with_gil(|py| {
            let locals = PyDict::new(py);
            py.run(
                c"key_mapper = lambda s: s.employee\nsum_mapper = lambda s: s.hours",
                None,
                Some(&locals),
            )
            .unwrap();
            let key_mapper = locals.get_item("key_mapper").unwrap().unwrap();
            let sum_mapper = locals.get_item("sum_mapper").unwrap().unwrap();

            let stream = PyUniConstraintStream::new("Shift".to_string(), false);
            let collector =
                crate::collectors::PyConstraintCollectors::sum_rust(py, sum_mapper.unbind())
                    .unwrap();
            let grouped = stream
                .group_by(py, key_mapper.unbind(), &collector)
                .unwrap();

            // Should have 2 components: ForEach + GroupBy
            assert_eq!(grouped.components.len(), 2);

            // Can chain with penalize
            let constraint = grouped.penalize_weight("Too many hours", 1);
            assert_eq!(constraint.name(), "Too many hours");
        });
    }

    #[test]
    fn test_group_by_chain_with_filter() {
        init_python();
        Python::with_gil(|py| {
            let locals = PyDict::new(py);
            py.run(
                c"key_mapper = lambda s: s.employee\nfilter_pred = lambda emp, count: count > 5",
                None,
                Some(&locals),
            )
            .unwrap();
            let key_mapper = locals.get_item("key_mapper").unwrap().unwrap();
            let filter_pred = locals.get_item("filter_pred").unwrap().unwrap();

            let stream = PyUniConstraintStream::new("Shift".to_string(), false);
            let collector = crate::collectors::PyConstraintCollectors::count_rust();
            let grouped = stream
                .group_by(py, key_mapper.unbind(), &collector)
                .unwrap();
            let filtered = grouped.filter(py, filter_pred.unbind()).unwrap();

            // Should have 3 components: ForEach + GroupBy + Filter
            assert_eq!(filtered.components.len(), 3);
        });
    }
}
