//! Python bindings for constraint collectors.
//!
//! Collectors are used in groupBy operations to aggregate values.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use rust_decimal::Decimal;
use solverforge_core::constraints::Collector;
use std::collections::HashMap;

use crate::lambda_analyzer::LambdaInfo;

/// A Python-wrapped collector for use in groupBy operations.
#[pyclass(name = "Collector")]
#[derive(Clone)]
pub struct PyCollector {
    inner: Collector,
}

impl PyCollector {
    pub fn to_rust(&self) -> Collector {
        self.inner.clone()
    }

    pub fn from_rust(inner: Collector) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCollector {
    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }
}

/// Factory class for creating collectors.
///
/// This class provides static methods to create various collector types
/// for use in groupBy operations.
///
/// # Example
/// ```python
/// from solverforge import ConstraintCollectors
///
/// # Count all items
/// counter = ConstraintCollectors.count()
///
/// # Sum values
/// summer = ConstraintCollectors.sum(lambda item: item.value)
/// ```
#[pyclass(name = "ConstraintCollectors")]
pub struct PyConstraintCollectors;

impl PyConstraintCollectors {
    /// Count the number of items in a group (Rust API).
    pub fn count_rust() -> PyCollector {
        PyCollector {
            inner: Collector::count(),
        }
    }

    /// Count distinct items in a group (Rust API).
    pub fn count_distinct_rust() -> PyCollector {
        PyCollector {
            inner: Collector::count_distinct(),
        }
    }

    /// Sum values in a group (Rust API).
    pub fn sum_rust(py: Python<'_>, mapper: Py<PyAny>) -> PyResult<PyCollector> {
        let lambda_info = LambdaInfo::new(py, mapper, "sum")?;
        let wasm_func = lambda_info.to_wasm_function();
        Ok(PyCollector {
            inner: Collector::sum(wasm_func),
        })
    }
}

#[pymethods]
impl PyConstraintCollectors {
    #[new]
    fn new() -> Self {
        Self
    }

    /// Count the number of items in a group.
    #[staticmethod]
    fn count() -> PyCollector {
        PyCollector {
            inner: Collector::count(),
        }
    }

    /// Count distinct items in a group.
    #[staticmethod]
    fn count_distinct() -> PyCollector {
        PyCollector {
            inner: Collector::count_distinct(),
        }
    }

    /// Count items after mapping them.
    #[staticmethod]
    fn count_with_map(py: Python<'_>, mapper: Py<PyAny>) -> PyResult<PyCollector> {
        let lambda_info = LambdaInfo::new(py, mapper, "count_map")?;
        let wasm_func = lambda_info.to_wasm_function();
        Ok(PyCollector {
            inner: Collector::count_with_map(wasm_func),
        })
    }

    /// Sum values in a group.
    ///
    /// # Arguments
    /// * `mapper` - A lambda that extracts the numeric value to sum
    ///
    /// # Example
    /// ```python
    /// ConstraintCollectors.sum(lambda shift: shift.hours)
    /// ```
    #[staticmethod]
    fn sum(py: Python<'_>, mapper: Py<PyAny>) -> PyResult<PyCollector> {
        let lambda_info = LambdaInfo::new(py, mapper, "sum")?;
        let wasm_func = lambda_info.to_wasm_function();
        Ok(PyCollector {
            inner: Collector::sum(wasm_func),
        })
    }

    /// Calculate average of values in a group.
    ///
    /// # Arguments
    /// * `mapper` - A lambda that extracts the numeric value to average
    #[staticmethod]
    fn average(py: Python<'_>, mapper: Py<PyAny>) -> PyResult<PyCollector> {
        let lambda_info = LambdaInfo::new(py, mapper, "average")?;
        let wasm_func = lambda_info.to_wasm_function();
        Ok(PyCollector {
            inner: Collector::average(wasm_func),
        })
    }

    /// Find minimum value in a group.
    ///
    /// # Arguments
    /// * `mapper` - A lambda that extracts the value to compare
    /// * `comparator` - A lambda that compares two values (returns negative, 0, or positive)
    #[staticmethod]
    fn min(py: Python<'_>, mapper: Py<PyAny>, comparator: Py<PyAny>) -> PyResult<PyCollector> {
        let map_info = LambdaInfo::new(py, mapper, "min_map")?;
        let cmp_info = LambdaInfo::new(py, comparator, "min_cmp")?;
        Ok(PyCollector {
            inner: Collector::min(map_info.to_wasm_function(), cmp_info.to_wasm_function()),
        })
    }

    /// Find maximum value in a group.
    ///
    /// # Arguments
    /// * `mapper` - A lambda that extracts the value to compare
    /// * `comparator` - A lambda that compares two values (returns negative, 0, or positive)
    #[staticmethod]
    fn max(py: Python<'_>, mapper: Py<PyAny>, comparator: Py<PyAny>) -> PyResult<PyCollector> {
        let map_info = LambdaInfo::new(py, mapper, "max_map")?;
        let cmp_info = LambdaInfo::new(py, comparator, "max_cmp")?;
        Ok(PyCollector {
            inner: Collector::max(map_info.to_wasm_function(), cmp_info.to_wasm_function()),
        })
    }

    /// Collect items into a list.
    #[staticmethod]
    fn to_list() -> PyCollector {
        PyCollector {
            inner: Collector::to_list(),
        }
    }

    /// Collect mapped values into a list.
    ///
    /// # Arguments
    /// * `mapper` - A lambda that extracts the value to collect
    #[staticmethod]
    fn to_list_with_map(py: Python<'_>, mapper: Py<PyAny>) -> PyResult<PyCollector> {
        let lambda_info = LambdaInfo::new(py, mapper, "to_list_map")?;
        Ok(PyCollector {
            inner: Collector::to_list_with_map(lambda_info.to_wasm_function()),
        })
    }

    /// Collect items into a set (unique values only).
    #[staticmethod]
    fn to_set() -> PyCollector {
        PyCollector {
            inner: Collector::to_set(),
        }
    }

    /// Collect mapped values into a set.
    ///
    /// # Arguments
    /// * `mapper` - A lambda that extracts the value to collect
    #[staticmethod]
    fn to_set_with_map(py: Python<'_>, mapper: Py<PyAny>) -> PyResult<PyCollector> {
        let lambda_info = LambdaInfo::new(py, mapper, "to_set_map")?;
        Ok(PyCollector {
            inner: Collector::to_set_with_map(lambda_info.to_wasm_function()),
        })
    }

    /// Calculate load balance for a group.
    ///
    /// Used for fair distribution constraints.
    ///
    /// # Arguments
    /// * `mapper` - A lambda that extracts the balancing key
    #[staticmethod]
    fn load_balance(py: Python<'_>, mapper: Py<PyAny>) -> PyResult<PyCollector> {
        let lambda_info = LambdaInfo::new(py, mapper, "load_balance")?;
        Ok(PyCollector {
            inner: Collector::load_balance(lambda_info.to_wasm_function()),
        })
    }

    /// Calculate load balance with custom load function.
    ///
    /// # Arguments
    /// * `mapper` - A lambda that extracts the balancing key
    /// * `load` - A lambda that extracts the load value
    #[staticmethod]
    fn load_balance_with_load(
        py: Python<'_>,
        mapper: Py<PyAny>,
        load: Py<PyAny>,
    ) -> PyResult<PyCollector> {
        let map_info = LambdaInfo::new(py, mapper, "load_balance_map")?;
        let load_info = LambdaInfo::new(py, load, "load_balance_load")?;
        Ok(PyCollector {
            inner: Collector::load_balance_with_load(
                map_info.to_wasm_function(),
                load_info.to_wasm_function(),
            ),
        })
    }

    /// Compose multiple collectors into a single result.
    ///
    /// # Arguments
    /// * `collectors` - List of collectors to compose
    /// * `combiner` - Lambda that combines the results into a single value
    ///
    /// # Example
    /// ```python
    /// ConstraintCollectors.compose(
    ///     [ConstraintCollectors.count(), ConstraintCollectors.sum(lambda x: x.value)],
    ///     lambda count, total: total / count if count > 0 else 0
    /// )
    /// ```
    #[staticmethod]
    fn compose(
        py: Python<'_>,
        collectors: Vec<PyCollector>,
        combiner: Py<PyAny>,
    ) -> PyResult<PyCollector> {
        let rust_collectors: Vec<Collector> = collectors.iter().map(|c| c.to_rust()).collect();
        let combiner_info = LambdaInfo::new(py, combiner, "compose_combiner")?;
        Ok(PyCollector {
            inner: Collector::compose(rust_collectors, combiner_info.to_wasm_function()),
        })
    }

    /// Apply a collector conditionally based on a predicate.
    ///
    /// # Arguments
    /// * `predicate` - Lambda that returns true if the item should be collected
    /// * `collector` - The collector to apply when predicate is true
    ///
    /// # Example
    /// ```python
    /// ConstraintCollectors.conditionally(
    ///     lambda shift: shift.is_overtime,
    ///     ConstraintCollectors.count()
    /// )
    /// ```
    #[staticmethod]
    fn conditionally(
        py: Python<'_>,
        predicate: Py<PyAny>,
        collector: PyCollector,
    ) -> PyResult<PyCollector> {
        let pred_info = LambdaInfo::new(py, predicate, "conditionally_pred")?;
        Ok(PyCollector {
            inner: Collector::conditionally(pred_info.to_wasm_function(), collector.to_rust()),
        })
    }

    fn __repr__(&self) -> &'static str {
        "ConstraintCollectors()"
    }
}

/// Load balance result containing fairness metrics.
///
/// This is the result type from the load_balance collector.
/// Use `unfairness()` to get the fairness measure.
#[pyclass(name = "LoadBalance")]
#[derive(Clone)]
pub struct PyLoadBalance {
    loads: HashMap<String, i64>,
    unfairness_value: Decimal,
}

impl PyLoadBalance {
    pub fn new(loads: HashMap<String, i64>, unfairness: Decimal) -> Self {
        Self {
            loads,
            unfairness_value: unfairness,
        }
    }

    pub fn from_raw(loads: HashMap<String, i64>) -> Self {
        // Calculate unfairness from loads
        // Unfairness = standard deviation of load distribution
        let n = loads.len() as f64;
        if n == 0.0 {
            return Self {
                loads,
                unfairness_value: Decimal::ZERO,
            };
        }

        let total: i64 = loads.values().sum();
        let mean = total as f64 / n;

        let variance: f64 = loads
            .values()
            .map(|&load| {
                let diff = load as f64 - mean;
                diff * diff
            })
            .sum::<f64>()
            / n;

        let std_dev = variance.sqrt();

        // Scale to 6 decimal places as per Timefold spec
        let unfairness = Decimal::from_f64_retain(std_dev)
            .unwrap_or(Decimal::ZERO)
            .round_dp(6);

        Self {
            loads,
            unfairness_value: unfairness,
        }
    }
}

#[pymethods]
impl PyLoadBalance {
    /// Get the loads for each balanced item.
    ///
    /// Returns a dictionary mapping item keys to their total load.
    fn loads<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        for (key, value) in &self.loads {
            dict.set_item(key, *value)?;
        }
        Ok(dict)
    }

    /// Get the unfairness measure.
    ///
    /// Returns a float representing how unfairly the load is distributed.
    /// Zero means perfectly balanced; higher values mean more imbalance.
    fn unfairness(&self) -> f64 {
        self.unfairness_value
            .to_string()
            .parse::<f64>()
            .unwrap_or(0.0)
    }

    fn __repr__(&self) -> String {
        format!(
            "LoadBalance(items={}, unfairness={})",
            self.loads.len(),
            self.unfairness_value
        )
    }
}

/// Register collector classes with the Python module.
pub fn register_collectors(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyCollector>()?;
    m.add_class::<PyConstraintCollectors>()?;
    m.add_class::<PyLoadBalance>()?;
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
    fn test_count_collector() {
        let collector = PyConstraintCollectors::count();
        let rust_collector = collector.to_rust();
        assert!(matches!(rust_collector, Collector::Count { .. }));
    }

    #[test]
    fn test_count_distinct_collector() {
        let collector = PyConstraintCollectors::count_distinct();
        let rust_collector = collector.to_rust();
        match rust_collector {
            Collector::Count { distinct, .. } => {
                assert_eq!(distinct, Some(true));
            }
            _ => panic!("Expected Count collector"),
        }
    }

    #[test]
    fn test_sum_collector() {
        init_python();
        Python::with_gil(|py| {
            let locals = PyDict::new(py);
            py.run(c"f = lambda x: x.hours", None, Some(&locals))
                .unwrap();
            let func = locals.get_item("f").unwrap().unwrap();

            let collector = PyConstraintCollectors::sum(py, func.unbind()).unwrap();
            let rust_collector = collector.to_rust();
            assert!(matches!(rust_collector, Collector::Sum { .. }));
        });
    }

    #[test]
    fn test_average_collector() {
        init_python();
        Python::with_gil(|py| {
            let locals = PyDict::new(py);
            py.run(c"f = lambda x: x.score", None, Some(&locals))
                .unwrap();
            let func = locals.get_item("f").unwrap().unwrap();

            let collector = PyConstraintCollectors::average(py, func.unbind()).unwrap();
            let rust_collector = collector.to_rust();
            assert!(matches!(rust_collector, Collector::Average { .. }));
        });
    }

    #[test]
    fn test_to_list_collector() {
        let collector = PyConstraintCollectors::to_list();
        let rust_collector = collector.to_rust();
        assert!(matches!(rust_collector, Collector::ToList { map: None }));
    }

    #[test]
    fn test_to_set_collector() {
        let collector = PyConstraintCollectors::to_set();
        let rust_collector = collector.to_rust();
        assert!(matches!(rust_collector, Collector::ToSet { map: None }));
    }

    #[test]
    fn test_load_balance_collector() {
        init_python();
        Python::with_gil(|py| {
            let locals = PyDict::new(py);
            py.run(c"f = lambda shift: shift.employee", None, Some(&locals))
                .unwrap();
            let func = locals.get_item("f").unwrap().unwrap();

            let collector = PyConstraintCollectors::load_balance(py, func.unbind()).unwrap();
            let rust_collector = collector.to_rust();
            assert!(matches!(rust_collector, Collector::LoadBalance { .. }));
        });
    }

    #[test]
    fn test_collector_repr() {
        let collector = PyConstraintCollectors::count();
        let repr = collector.__repr__();
        assert!(repr.contains("Count"));
    }

    #[test]
    fn test_compose_collector() {
        init_python();
        Python::with_gil(|py| {
            let locals = PyDict::new(py);
            py.run(c"combiner = lambda a, b: a + b", None, Some(&locals))
                .unwrap();
            let combiner = locals.get_item("combiner").unwrap().unwrap();

            let collectors = vec![
                PyConstraintCollectors::count(),
                PyConstraintCollectors::count_distinct(),
            ];

            let collector =
                PyConstraintCollectors::compose(py, collectors, combiner.unbind()).unwrap();
            let rust_collector = collector.to_rust();
            assert!(matches!(rust_collector, Collector::Compose { .. }));
        });
    }

    #[test]
    fn test_conditionally_collector() {
        init_python();
        Python::with_gil(|py| {
            let locals = PyDict::new(py);
            py.run(c"pred = lambda x: x.active", None, Some(&locals))
                .unwrap();
            let pred = locals.get_item("pred").unwrap().unwrap();

            let inner = PyConstraintCollectors::count();
            let collector =
                PyConstraintCollectors::conditionally(py, pred.unbind(), inner).unwrap();
            let rust_collector = collector.to_rust();
            assert!(matches!(rust_collector, Collector::Conditionally { .. }));
        });
    }
}
