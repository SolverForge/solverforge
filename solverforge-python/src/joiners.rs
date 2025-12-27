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
    /// is not supported. Uses "Entity" as default class since joiners are created
    /// before stream context is available.
    pub fn new(py: Python<'_>, callable: Py<PyAny>, prefix: &str) -> PyResult<Self> {
        // Joiners use "Entity" as default - specific class context comes from stream
        let info = LambdaInfo::new(py, callable, prefix, "Entity")?;
        Ok(Self { info })
    }

    /// Convert to WasmFunction reference.
    pub fn to_wasm_function(&self) -> WasmFunction {
        self.info.to_wasm_function()
    }
}

/// Wrapper for a Joiner that can be passed to stream methods.
#[pyclass(name = "Joiner")]
#[derive(Clone)]
pub struct PyJoiner {
    inner: Joiner,
    /// Stored lambdas for later analysis (used by tests).
    #[allow(dead_code)]
    lambdas: Vec<JoinerLambda>,
}

impl PyJoiner {
    pub fn to_rust(&self) -> Joiner {
        self.inner.clone()
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
}
