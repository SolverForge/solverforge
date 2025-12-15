//! Python bindings for SolverForge constraint solver
//!
//! This crate provides Python bindings using PyO3, offering a 1:1 compatible API
//! with Timefold's Python bindings.

use pyo3::prelude::*;

mod bridge;

pub use bridge::{PyBridge, PythonBridge};

/// SolverForge Python module
///
/// Provides constraint solving capabilities with an API compatible with Timefold.
#[pymodule]
fn _solverforge(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Version info
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    // Bridge for Python-Rust object interop
    m.add_class::<PyBridge>()?;

    Ok(())
}
