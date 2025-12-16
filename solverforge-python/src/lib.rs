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
