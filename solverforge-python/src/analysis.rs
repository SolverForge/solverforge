//! Python bindings for solution analysis types.
//!
//! Exposes the Rust SolutionManager, ScoreExplanation, ConstraintMatch, and Indictment
//! types to Python via PyO3.

use pyo3::prelude::*;
use solverforge_core::analysis::{ConstraintMatch, Indictment, ScoreExplanation, SolutionManager};
use solverforge_core::solver::ScoreDto;
use std::sync::Arc;

use crate::bridge::PythonBridge;
use crate::solver::PySolverFactory;

/// Python wrapper for ScoreExplanation.
#[pyclass(name = "ScoreExplanation")]
#[derive(Clone)]
pub struct PyScoreExplanation {
    inner: ScoreExplanation,
}

#[pymethods]
impl PyScoreExplanation {
    /// Get the total score.
    #[getter]
    fn score(&self) -> PyScoreDto {
        PyScoreDto::from_rust(self.inner.score.clone())
    }

    /// Get the hard score component.
    #[getter]
    fn hard_score(&self) -> i64 {
        self.inner.hard_score()
    }

    /// Get the soft score component.
    #[getter]
    fn soft_score(&self) -> i64 {
        self.inner.soft_score()
    }

    /// Get the medium score component (if present).
    #[getter]
    fn medium_score(&self) -> Option<i64> {
        self.inner.medium_score()
    }

    /// Check if the solution is feasible (hard score >= 0).
    fn is_feasible(&self) -> bool {
        self.inner.is_feasible()
    }

    /// Get all constraint matches.
    #[getter]
    fn constraint_matches(&self) -> Vec<PyConstraintMatch> {
        self.inner
            .constraint_matches
            .iter()
            .map(|cm| PyConstraintMatch::from_rust(cm.clone()))
            .collect()
    }

    /// Get all indictments.
    #[getter]
    fn indictments(&self) -> Vec<PyIndictment> {
        self.inner
            .indictments
            .iter()
            .map(|i| PyIndictment::from_rust(i.clone()))
            .collect()
    }

    /// Get the number of constraint matches.
    fn constraint_count(&self) -> usize {
        self.inner.constraint_count()
    }

    /// Get constraint matches by name.
    fn get_constraint_matches_by_name(&self, name: &str) -> Vec<PyConstraintMatch> {
        self.inner
            .get_constraint_matches_by_name(name)
            .into_iter()
            .map(|cm| PyConstraintMatch::from_rust(cm.clone()))
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "ScoreExplanation(score={}, feasible={}, constraints={})",
            self.inner.score.score_string,
            self.inner.is_feasible(),
            self.inner.constraint_count()
        )
    }
}

impl PyScoreExplanation {
    pub fn from_rust(explanation: ScoreExplanation) -> Self {
        Self { inner: explanation }
    }
}

/// Python wrapper for ConstraintMatch.
#[pyclass(name = "ConstraintMatch")]
#[derive(Clone)]
pub struct PyConstraintMatch {
    inner: ConstraintMatch,
}

#[pymethods]
impl PyConstraintMatch {
    /// Get the constraint name.
    #[getter]
    fn constraint_name(&self) -> &str {
        &self.inner.constraint_name
    }

    /// Get the constraint package (if set).
    #[getter]
    fn constraint_package(&self) -> Option<&str> {
        self.inner.constraint_package.as_deref()
    }

    /// Get the full constraint name (package.name).
    fn full_constraint_name(&self) -> String {
        self.inner.full_constraint_name()
    }

    /// Get the score impact of this constraint match.
    #[getter]
    fn score(&self) -> PyScoreDto {
        PyScoreDto::from_rust(self.inner.score.clone())
    }

    /// Get the hard score impact.
    #[getter]
    fn hard_score(&self) -> i64 {
        self.inner.hard_score()
    }

    /// Get the soft score impact.
    #[getter]
    fn soft_score(&self) -> i64 {
        self.inner.soft_score()
    }

    /// Check if this constraint match is feasible.
    fn is_feasible(&self) -> bool {
        self.inner.is_feasible()
    }

    /// Get the indicted object handles.
    #[getter]
    fn indicted_objects(&self) -> Vec<u64> {
        self.inner.indicted_objects.iter().map(|h| h.id()).collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "ConstraintMatch(name='{}', score={})",
            self.inner.constraint_name, self.inner.score.score_string
        )
    }
}

impl PyConstraintMatch {
    pub fn from_rust(cm: ConstraintMatch) -> Self {
        Self { inner: cm }
    }
}

/// Python wrapper for Indictment.
#[pyclass(name = "Indictment")]
#[derive(Clone)]
pub struct PyIndictment {
    inner: Indictment,
}

#[pymethods]
impl PyIndictment {
    /// Get the indicted object handle.
    #[getter]
    fn indicted_object(&self) -> u64 {
        self.inner.indicted_object.id()
    }

    /// Get the total score impact on this object.
    #[getter]
    fn score(&self) -> PyScoreDto {
        PyScoreDto::from_rust(self.inner.score.clone())
    }

    /// Get the hard score impact.
    #[getter]
    fn hard_score(&self) -> i64 {
        self.inner.hard_score()
    }

    /// Get the soft score impact.
    #[getter]
    fn soft_score(&self) -> i64 {
        self.inner.soft_score()
    }

    /// Check if this indictment is feasible.
    fn is_feasible(&self) -> bool {
        self.inner.is_feasible()
    }

    /// Get constraint matches for this indicted object.
    #[getter]
    fn constraint_matches(&self) -> Vec<PyConstraintMatch> {
        self.inner
            .constraint_matches
            .iter()
            .map(|cm| PyConstraintMatch::from_rust(cm.clone()))
            .collect()
    }

    /// Get the number of constraints affecting this object.
    fn constraint_count(&self) -> usize {
        self.inner.constraint_count()
    }

    /// Get constraint matches by name.
    fn get_constraint_matches_by_name(&self, name: &str) -> Vec<PyConstraintMatch> {
        self.inner
            .get_constraint_matches_by_name(name)
            .into_iter()
            .map(|cm| PyConstraintMatch::from_rust(cm.clone()))
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "Indictment(object={}, score={}, constraints={})",
            self.inner.indicted_object.id(),
            self.inner.score.score_string,
            self.inner.constraint_count()
        )
    }
}

impl PyIndictment {
    pub fn from_rust(indictment: Indictment) -> Self {
        Self { inner: indictment }
    }
}

/// Python wrapper for ScoreDto (used in analysis results).
#[pyclass(name = "ScoreDto")]
#[derive(Clone)]
pub struct PyScoreDto {
    inner: ScoreDto,
}

#[pymethods]
impl PyScoreDto {
    /// Get the score as a string.
    #[getter]
    fn score_string(&self) -> &str {
        &self.inner.score_string
    }

    /// Get the hard score component.
    #[getter]
    fn hard_score(&self) -> i64 {
        self.inner.hard_score
    }

    /// Get the soft score component.
    #[getter]
    fn soft_score(&self) -> i64 {
        self.inner.soft_score
    }

    /// Get the medium score component (if present).
    #[getter]
    fn medium_score(&self) -> Option<i64> {
        self.inner.medium_score
    }

    /// Check if the score is feasible.
    #[getter]
    fn is_feasible(&self) -> bool {
        self.inner.is_feasible
    }

    fn __repr__(&self) -> String {
        format!("ScoreDto('{}')", self.inner.score_string)
    }
}

impl PyScoreDto {
    pub fn from_rust(score: ScoreDto) -> Self {
        Self { inner: score }
    }
}

/// Python wrapper for SolutionManager.
#[pyclass(name = "SolutionManager")]
pub struct PySolutionManager {
    inner: SolutionManager<PythonBridge>,
    bridge: Arc<PythonBridge>,
}

#[pymethods]
impl PySolutionManager {
    /// Create a SolutionManager from a SolverFactory.
    #[staticmethod]
    fn create(factory: &PySolverFactory) -> PyResult<Self> {
        let bridge = Arc::new(PythonBridge::new());
        let inner = factory.create_solution_manager();
        Ok(Self { inner, bridge })
    }

    /// Calculate the score for a solution.
    ///
    /// Args:
    ///     solution: The planning solution to score
    ///
    /// Returns:
    ///     ScoreDto with the calculated score
    fn update(&self, py: Python<'_>, solution: Py<PyAny>) -> PyResult<PyScoreDto> {
        let handle = self.bridge.register_object(solution.clone_ref(py));
        let score = self
            .inner
            .update(&*self.bridge, handle)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(PyScoreDto::from_rust(score))
    }

    /// Get a detailed score explanation for a solution.
    ///
    /// Args:
    ///     solution: The planning solution to explain
    ///
    /// Returns:
    ///     ScoreExplanation with constraint matches and indictments
    fn explain(&self, py: Python<'_>, solution: Py<PyAny>) -> PyResult<PyScoreExplanation> {
        let handle = self.bridge.register_object(solution.clone_ref(py));
        let explanation = self
            .inner
            .explain(&*self.bridge, handle)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(PyScoreExplanation::from_rust(explanation))
    }

    /// Analyze constraint matches for a solution (alias for explain).
    fn analyze(&self, py: Python<'_>, solution: Py<PyAny>) -> PyResult<PyScoreExplanation> {
        self.explain(py, solution)
    }

    fn __repr__(&self) -> String {
        format!(
            "SolutionManager(solution_class={:?})",
            self.inner.config().solution_class
        )
    }
}

/// Register analysis classes with the Python module.
pub fn register_analysis(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyScoreExplanation>()?;
    m.add_class::<PyConstraintMatch>()?;
    m.add_class::<PyIndictment>()?;
    m.add_class::<PyScoreDto>()?;
    m.add_class::<PySolutionManager>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use solverforge_core::ObjectHandle;

    #[test]
    fn test_score_dto_from_rust() {
        let score = ScoreDto::hard_soft(-2, -15);
        let py_score = PyScoreDto::from_rust(score);

        assert_eq!(py_score.inner.hard_score, -2);
        assert_eq!(py_score.inner.soft_score, -15);
        assert!(!py_score.inner.is_feasible);
    }

    #[test]
    fn test_constraint_match_from_rust() {
        let cm = ConstraintMatch::new("roomConflict", ScoreDto::hard_soft(-1, 0));
        let py_cm = PyConstraintMatch::from_rust(cm);

        assert_eq!(py_cm.inner.constraint_name, "roomConflict");
        assert_eq!(py_cm.inner.hard_score(), -1);
    }

    #[test]
    fn test_indictment_from_rust() {
        let indictment = Indictment::new(ObjectHandle::new(42), ScoreDto::hard_soft(-1, -5));
        let py_indictment = PyIndictment::from_rust(indictment);

        assert_eq!(py_indictment.inner.indicted_object.id(), 42);
        assert_eq!(py_indictment.inner.hard_score(), -1);
    }

    #[test]
    fn test_score_explanation_from_rust() {
        let explanation = ScoreExplanation::new(ScoreDto::hard_soft(0, -10))
            .with_constraint_match(ConstraintMatch::new("test", ScoreDto::hard_soft(0, -10)));

        let py_explanation = PyScoreExplanation::from_rust(explanation);

        assert!(py_explanation.inner.is_feasible());
        assert_eq!(py_explanation.inner.constraint_count(), 1);
    }
}
