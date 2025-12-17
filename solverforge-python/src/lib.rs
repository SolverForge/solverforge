//! Python bindings for SolverForge constraint solver
//!
//! This crate provides Python bindings using PyO3, offering a 1:1 compatible API
//! with Timefold's Python bindings.

use pyo3::prelude::*;

mod annotations;
mod bridge;
mod collectors;
mod decorators;
mod joiners;
mod lambda_analyzer;
mod score;
mod solver;
mod stream;

pub use annotations::{
    PyDeepPlanningClone, PyInverseRelationShadowVariable, PyPlanningEntityCollectionProperty,
    PyPlanningEntityProperty, PyPlanningId, PyPlanningListVariable, PyPlanningPin, PyPlanningScore,
    PyPlanningVariable, PyProblemFactCollectionProperty, PyProblemFactProperty,
    PyValueRangeProvider,
};
pub use bridge::{PyBridge, PythonBridge};
pub use collectors::{PyCollector, PyConstraintCollectors};
pub use decorators::{PyConstraintProvider, PyDomainClass, PyDomainModel};
pub use joiners::{PyJoiner, PyJoiners};
pub use lambda_analyzer::{analyze_lambda, generate_lambda_name, LambdaInfo};
pub use score::{PyHardMediumSoftScore, PyHardSoftScore, PySimpleScore};
pub use solver::{
    PyDiminishedReturnsConfig, PyEnvironmentMode, PyMoveThreadCount, PySolveHandle,
    PySolveResponse, PySolveStatus, PySolver, PySolverConfig, PySolverFactory, PyTerminationConfig,
};
pub use stream::{
    PyBiConstraintBuilder, PyBiConstraintStream, PyConstraint, PyConstraintFactory,
    PyTriConstraintBuilder, PyTriConstraintStream, PyUniConstraintBuilder, PyUniConstraintStream,
};

/// SolverForge Python module
///
/// Provides constraint solving capabilities with an API compatible with Timefold.
#[pymodule]
fn _solverforge(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Version info
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    // Bridge for Python-Rust object interop
    m.add_class::<PyBridge>()?;

    // Annotation marker classes
    annotations::register_annotations(m)?;

    // Score types
    score::register_scores(m)?;

    // Decorators
    decorators::register_decorators(m)?;

    // Constraint streams
    stream::register_streams(m)?;

    // Joiners
    joiners::register_joiners(m)?;

    // Collectors
    collectors::register_collectors(m)?;

    // Solver
    solver::register_solver(m)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that all solver runtime types are accessible from lib.rs exports
    #[test]
    fn test_solver_runtime_types_exported() {
        // These type assertions verify the types are publicly re-exported
        fn assert_type<T>() {}

        assert_type::<PySolverConfig>();
        assert_type::<PyTerminationConfig>();
        assert_type::<PySolverFactory>();
        assert_type::<PySolver>();
        assert_type::<PySolveHandle>();
        assert_type::<PySolveResponse>();
        assert_type::<PySolveStatus>();
        assert_type::<PyEnvironmentMode>();
        assert_type::<PyMoveThreadCount>();
        assert_type::<PyDiminishedReturnsConfig>();
    }

    /// Verify that all constraint stream types are accessible
    #[test]
    fn test_constraint_stream_types_exported() {
        fn assert_type<T>() {}

        assert_type::<PyConstraintFactory>();
        assert_type::<PyUniConstraintStream>();
        assert_type::<PyBiConstraintStream>();
        assert_type::<PyTriConstraintStream>();
        assert_type::<PyUniConstraintBuilder>();
        assert_type::<PyBiConstraintBuilder>();
        assert_type::<PyTriConstraintBuilder>();
        assert_type::<PyConstraint>();
    }

    /// Verify that joiner types are accessible
    #[test]
    fn test_joiner_types_exported() {
        fn assert_type<T>() {}

        assert_type::<PyJoiner>();
        assert_type::<PyJoiners>();
    }

    /// Verify that collector types are accessible
    #[test]
    fn test_collector_types_exported() {
        fn assert_type<T>() {}

        assert_type::<PyCollector>();
        assert_type::<PyConstraintCollectors>();
    }

    /// Verify that decorator types are accessible
    #[test]
    fn test_decorator_types_exported() {
        fn assert_type<T>() {}

        assert_type::<PyConstraintProvider>();
        assert_type::<PyDomainClass>();
        assert_type::<PyDomainModel>();
    }

    /// Verify that score types are accessible
    #[test]
    fn test_score_types_exported() {
        fn assert_type<T>() {}

        assert_type::<PySimpleScore>();
        assert_type::<PyHardSoftScore>();
        assert_type::<PyHardMediumSoftScore>();
    }
}
