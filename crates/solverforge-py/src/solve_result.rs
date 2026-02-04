//! Solve result type for Python API.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use solverforge_dynamic::DynamicSolution;

use crate::convert::dynamic_to_py;

/// Result of solving a problem.
#[pyclass]
pub struct PySolveResult {
    #[pyo3(get)]
    pub score: String,
    #[pyo3(get)]
    pub hard_score: i64,
    #[pyo3(get)]
    pub soft_score: i64,
    #[pyo3(get)]
    pub is_feasible: bool,
    #[pyo3(get)]
    pub duration_ms: u64,
    #[pyo3(get)]
    pub steps: u64,
    #[pyo3(get)]
    pub moves_evaluated: u64,
    pub(crate) solution: DynamicSolution,
}

impl PySolveResult {
    /// Create a new solve result.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        score: String,
        hard_score: i64,
        soft_score: i64,
        is_feasible: bool,
        duration_ms: u64,
        steps: u64,
        moves_evaluated: u64,
        solution: DynamicSolution,
    ) -> Self {
        Self {
            score,
            hard_score,
            soft_score,
            is_feasible,
            duration_ms,
            steps,
            moves_evaluated,
            solution,
        }
    }
}

#[pymethods]
impl PySolveResult {
    /// Get entities of a specific class.
    fn get_entities(&self, py: Python<'_>, class_name: &str) -> PyResult<Py<PyAny>> {
        let class_idx = self
            .solution
            .descriptor
            .entity_class_index(class_name)
            .ok_or_else(|| {
                PyValueError::new_err(format!("Unknown entity class: {}", class_name))
            })?;

        let class_def = &self.solution.descriptor.entity_classes[class_idx];
        let entities: Vec<_> = self
            .solution
            .entities_in_class(class_idx)
            .map(|entity| {
                let dict = PyDict::new(py);
                dict.set_item("id", entity.id).unwrap();
                for (i, field) in class_def.fields.iter().enumerate() {
                    if let Some(value) = entity.fields.get(i) {
                        dict.set_item(field.name.as_ref(), dynamic_to_py(py, value))
                            .unwrap();
                    }
                }
                dict.into_any().unbind()
            })
            .collect();

        Ok(PyList::new(py, entities)?.into_any().unbind())
    }
}
