//! Python bindings for SolverForge constraint solver
//!
//! This crate provides Python bindings using PyO3, offering a 1:1 compatible API
//! with Timefold's Python bindings.

use pyo3::prelude::*;

mod annotations;
mod bridge;
mod score;

pub use annotations::{
    PyInverseRelationShadowVariable, PyPlanningEntityCollectionProperty, PyPlanningEntityProperty,
    PyPlanningId, PyPlanningListVariable, PyPlanningPin, PyPlanningScore, PyPlanningVariable,
    PyProblemFactCollectionProperty, PyProblemFactProperty, PyValueRangeProvider,
};
pub use bridge::{PyBridge, PythonBridge};
pub use score::{PyHardMediumSoftScore, PyHardSoftScore, PySimpleScore};

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

    Ok(())
}
