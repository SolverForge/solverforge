//! Python bindings for SolverForge.
//!
//! This module provides a Python API for defining and solving constraint
//! optimization problems using SolverForge's dynamic solution types.

// Allow deprecated pyo3 APIs until we upgrade to a newer version
#![allow(deprecated)]

mod constraint_builder;
mod convert;
mod solve_result;
mod solver;
mod solver_manager;

use pyo3::prelude::*;

pub use constraint_builder::ConstraintBuilder;
pub use solve_result::PySolveResult;
pub use solver::Solver;
pub use solver_manager::SolverManager;

/// SolverForge Python module.
#[pymodule]
fn solverforge(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Solver>()?;
    m.add_class::<SolverManager>()?;
    m.add_class::<ConstraintBuilder>()?;
    m.add_class::<PySolveResult>()?;
    Ok(())
}
