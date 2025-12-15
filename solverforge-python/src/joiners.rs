//! Python bindings for constraint stream joiners.
//!
//! Joiners are used to specify how entities should be matched in constraint streams.
//!
//! # Example
//!
//! ```python
//! factory.for_each_unique_pair(Lesson, Joiners.equal(lambda l: l.timeslot))
//! ```

use pyo3::prelude::*;
use solverforge_core::constraints::{Joiner, WasmFunction};

use crate::lambda_analyzer::LambdaInfo;

/// Stored lambda for joiner mapping functions.
///
/// Wraps a `LambdaInfo` for use in joiners.
#[derive(Clone)]
pub struct JoinerLambda {
    info: LambdaInfo,
}

impl JoinerLambda {
    /// Create a new JoinerLambda from a Python callable.
    ///
    /// This analyzes the lambda immediately and returns an error if the pattern
    /// is not supported.
    pub fn new(py: Python<'_>, callable: Py<PyAny>, prefix: &str) -> PyResult<Self> {
        let info = LambdaInfo::new(py, callable, prefix)?;
        Ok(Self { info })
    }

    /// Create with a class hint for type inference.
    #[allow(dead_code)]
    pub fn with_class_hint(mut self, class_name: impl Into<String>) -> Self {
        self.info = self.info.with_class_hint(class_name);
        self
    }

    /// Convert to WasmFunction reference.
    pub fn to_wasm_function(&self) -> WasmFunction {
        self.info.to_wasm_function()
    }

    /// Get the stored lambda info.
    #[allow(dead_code)]
    pub fn info(&self) -> &LambdaInfo {
        &self.info
    }
}

/// Wrapper for a Joiner that can be passed to stream methods.
#[pyclass(name = "Joiner")]
#[derive(Clone)]
pub struct PyJoiner {
    inner: Joiner,
    /// Stored lambdas for later analysis.
    lambdas: Vec<JoinerLambda>,
}

impl PyJoiner {
    pub fn to_rust(&self) -> Joiner {
        self.inner.clone()
    }

    /// Get stored lambdas for analysis.
    #[allow(dead_code)]
    pub fn lambdas(&self) -> &[JoinerLambda] {
        &self.lambdas
    }
}

#[pymethods]
impl PyJoiner {
    fn __repr__(&self) -> String {
        match &self.inner {
            Joiner::Equal { .. } => "Joiner.equal(...)".to_string(),
            Joiner::LessThan { .. } => "Joiner.less_than(...)".to_string(),
            Joiner::LessThanOrEqual { .. } => "Joiner.less_than_or_equal(...)".to_string(),
            Joiner::GreaterThan { .. } => "Joiner.greater_than(...)".to_string(),
            Joiner::GreaterThanOrEqual { .. } => "Joiner.greater_than_or_equal(...)".to_string(),
            Joiner::Overlapping { .. } => "Joiner.overlapping(...)".to_string(),
            Joiner::Filtering { .. } => "Joiner.filtering(...)".to_string(),
        }
    }
}

/// Static methods for creating joiners.
#[pyclass(name = "Joiners")]
pub struct PyJoiners;

impl PyJoiners {
    /// Create an equal joiner (Rust API for tests).
    pub fn equal_joiner(py: Python<'_>, mapping: Py<PyAny>) -> PyResult<PyJoiner> {
        let lambda = JoinerLambda::new(py, mapping, "equal_map")?;
        let wasm_func = lambda.to_wasm_function();

        Ok(PyJoiner {
            inner: Joiner::Equal {
                map: Some(wasm_func),
                left_map: None,
                right_map: None,
                relation_predicate: None,
                hasher: None,
            },
            lambdas: vec![lambda],
        })
    }
}

#[pymethods]
impl PyJoiners {
    /// Create a joiner that matches entities with equal values.
    ///
    /// # Arguments
    /// * `mapping` - A function that extracts the value to compare
    ///
    /// # Example
    /// ```python
    /// Joiners.equal(lambda lesson: lesson.timeslot)
    /// ```
    #[staticmethod]
    fn equal(py: Python<'_>, mapping: Py<PyAny>) -> PyResult<PyJoiner> {
        let lambda = JoinerLambda::new(py, mapping, "equal_map")?;
        let wasm_func = lambda.to_wasm_function();

        Ok(PyJoiner {
            inner: Joiner::Equal {
                map: Some(wasm_func),
                left_map: None,
                right_map: None,
                relation_predicate: None,
                hasher: None,
            },
            lambdas: vec![lambda],
        })
    }

    /// Create a joiner that matches entities where left < right.
    ///
    /// # Arguments
    /// * `mapping` - A function that extracts the value to compare
    #[staticmethod]
    fn less_than(py: Python<'_>, mapping: Py<PyAny>) -> PyResult<PyJoiner> {
        let lambda = JoinerLambda::new(py, mapping, "less_than_map")?;
        let wasm_func = lambda.to_wasm_function();

        Ok(PyJoiner {
            inner: Joiner::LessThan {
                map: Some(wasm_func),
                left_map: None,
                right_map: None,
                comparator: WasmFunction::new("compare"),
            },
            lambdas: vec![lambda],
        })
    }

    /// Create a joiner that matches entities where left <= right.
    ///
    /// # Arguments
    /// * `mapping` - A function that extracts the value to compare
    #[staticmethod]
    fn less_than_or_equal(py: Python<'_>, mapping: Py<PyAny>) -> PyResult<PyJoiner> {
        let lambda = JoinerLambda::new(py, mapping, "less_than_or_equal_map")?;
        let wasm_func = lambda.to_wasm_function();

        Ok(PyJoiner {
            inner: Joiner::LessThanOrEqual {
                map: Some(wasm_func),
                left_map: None,
                right_map: None,
                comparator: WasmFunction::new("compare"),
            },
            lambdas: vec![lambda],
        })
    }

    /// Create a joiner that matches entities where left > right.
    ///
    /// # Arguments
    /// * `mapping` - A function that extracts the value to compare
    #[staticmethod]
    fn greater_than(py: Python<'_>, mapping: Py<PyAny>) -> PyResult<PyJoiner> {
        let lambda = JoinerLambda::new(py, mapping, "greater_than_map")?;
        let wasm_func = lambda.to_wasm_function();

        Ok(PyJoiner {
            inner: Joiner::GreaterThan {
                map: Some(wasm_func),
                left_map: None,
                right_map: None,
                comparator: WasmFunction::new("compare"),
            },
            lambdas: vec![lambda],
        })
    }

    /// Create a joiner that matches entities where left >= right.
    ///
    /// # Arguments
    /// * `mapping` - A function that extracts the value to compare
    #[staticmethod]
    fn greater_than_or_equal(py: Python<'_>, mapping: Py<PyAny>) -> PyResult<PyJoiner> {
        let lambda = JoinerLambda::new(py, mapping, "greater_than_or_equal_map")?;
        let wasm_func = lambda.to_wasm_function();

        Ok(PyJoiner {
            inner: Joiner::GreaterThanOrEqual {
                map: Some(wasm_func),
                left_map: None,
                right_map: None,
                comparator: WasmFunction::new("compare"),
            },
            lambdas: vec![lambda],
        })
    }

    /// Create a joiner that matches entities with overlapping intervals.
    ///
    /// # Arguments
    /// * `start_mapping` - A function that extracts the interval start
    /// * `end_mapping` - A function that extracts the interval end
    #[staticmethod]
    fn overlapping(
        py: Python<'_>,
        start_mapping: Py<PyAny>,
        end_mapping: Py<PyAny>,
    ) -> PyResult<PyJoiner> {
        let start_lambda = JoinerLambda::new(py, start_mapping, "overlapping_start")?;
        let end_lambda = JoinerLambda::new(py, end_mapping, "overlapping_end")?;

        Ok(PyJoiner {
            inner: Joiner::Overlapping {
                start_map: Some(start_lambda.to_wasm_function()),
                end_map: Some(end_lambda.to_wasm_function()),
                left_start_map: None,
                left_end_map: None,
                right_start_map: None,
                right_end_map: None,
                comparator: Some(WasmFunction::new("compare")),
            },
            lambdas: vec![start_lambda, end_lambda],
        })
    }

    /// Create a filtering joiner with a bi-predicate.
    ///
    /// # Arguments
    /// * `predicate` - A function that takes two entities and returns a boolean
    #[staticmethod]
    fn filtering(py: Python<'_>, predicate: Py<PyAny>) -> PyResult<PyJoiner> {
        let lambda = JoinerLambda::new(py, predicate, "filter")?;

        Ok(PyJoiner {
            inner: Joiner::Filtering {
                filter: lambda.to_wasm_function(),
            },
            lambdas: vec![lambda],
        })
    }
}

/// Register joiner classes with the Python module.
pub fn register_joiners(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyJoiner>()?;
    m.add_class::<PyJoiners>()?;
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
    fn test_joiner_repr() {
        let joiner = PyJoiner {
            inner: Joiner::Equal {
                map: None,
                left_map: None,
                right_map: None,
                relation_predicate: None,
                hasher: None,
            },
            lambdas: vec![],
        };
        assert!(joiner.__repr__().contains("equal"));
    }

    #[test]
    fn test_joiner_equal_creates_lambda() {
        init_python();
        Python::with_gil(|py| {
            let locals = PyDict::new(py);
            py.run(c"f = lambda x: x.timeslot", None, Some(&locals))
                .unwrap();
            let func = locals.get_item("f").unwrap().unwrap();

            let joiner = PyJoiners::equal(py, func.unbind()).unwrap();

            assert_eq!(joiner.lambdas.len(), 1);
            assert!(joiner.lambdas[0].info().name.starts_with("equal_map_"));

            match &joiner.inner {
                Joiner::Equal { map, .. } => {
                    assert!(map.is_some());
                }
                _ => panic!("Expected Equal joiner"),
            }
        });
    }

    #[test]
    fn test_joiner_less_than() {
        init_python();
        Python::with_gil(|py| {
            let locals = PyDict::new(py);
            py.run(c"f = lambda x: x.start_time", None, Some(&locals))
                .unwrap();
            let func = locals.get_item("f").unwrap().unwrap();

            let joiner = PyJoiners::less_than(py, func.unbind()).unwrap();

            assert_eq!(joiner.lambdas.len(), 1);
            assert!(joiner.__repr__().contains("less_than"));
        });
    }

    #[test]
    fn test_joiner_overlapping_creates_two_lambdas() {
        init_python();
        Python::with_gil(|py| {
            let locals = PyDict::new(py);
            py.run(c"start = lambda x: x.start_time", None, Some(&locals))
                .unwrap();
            py.run(c"end = lambda x: x.end_time", None, Some(&locals))
                .unwrap();

            let start_func = locals.get_item("start").unwrap().unwrap();
            let end_func = locals.get_item("end").unwrap().unwrap();

            let joiner =
                PyJoiners::overlapping(py, start_func.unbind(), end_func.unbind()).unwrap();

            assert_eq!(joiner.lambdas.len(), 2);
            assert!(joiner.lambdas[0]
                .info()
                .name
                .starts_with("overlapping_start_"));
            assert!(joiner.lambdas[1]
                .info()
                .name
                .starts_with("overlapping_end_"));
        });
    }

    #[test]
    fn test_joiner_filtering() {
        init_python();
        Python::with_gil(|py| {
            let locals = PyDict::new(py);
            py.run(c"f = lambda a, b: a.id != b.id", None, Some(&locals))
                .unwrap();
            let func = locals.get_item("f").unwrap().unwrap();

            let joiner = PyJoiners::filtering(py, func.unbind()).unwrap();

            assert_eq!(joiner.lambdas.len(), 1);
            assert_eq!(joiner.lambdas[0].info().param_count, 2);
            assert!(joiner.__repr__().contains("filtering"));
        });
    }

    #[test]
    fn test_joiner_error_on_unsupported_lambda() {
        init_python();
        Python::with_gil(|py| {
            let locals = PyDict::new(py);
            // Lambda with external reference - not supported
            py.run(
                c"external = 42\nf = lambda x: x.value + external",
                None,
                Some(&locals),
            )
            .unwrap();
            let func = locals.get_item("f").unwrap().unwrap();

            let result = PyJoiners::equal(py, func.unbind());

            // Should fail because 'external' is not a parameter
            assert!(result.is_err());
        });
    }
}
