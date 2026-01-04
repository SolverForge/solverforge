//! Composite termination conditions (AND/OR).

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;

use super::Termination;
use crate::scope::SolverScope;

/// Combines multiple terminations with OR logic.
///
/// Terminates when ANY of the child terminations triggers.
pub struct OrCompositeTermination<S: PlanningSolution> {
    terminations: Vec<Box<dyn Termination<S>>>,
}

impl<S: PlanningSolution> Debug for OrCompositeTermination<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrCompositeTermination")
            .field("count", &self.terminations.len())
            .finish()
    }
}

impl<S: PlanningSolution> OrCompositeTermination<S> {
    pub fn new(terminations: Vec<Box<dyn Termination<S>>>) -> Self {
        Self { terminations }
    }
}

impl<S: PlanningSolution> Termination<S> for OrCompositeTermination<S> {
    fn is_terminated(&self, solver_scope: &SolverScope<S>) -> bool {
        self.terminations.iter().any(|t| t.is_terminated(solver_scope))
    }
}

/// Combines multiple terminations with AND logic.
///
/// All terminations must agree before solving terminates.
pub struct AndCompositeTermination<S: PlanningSolution> {
    terminations: Vec<Box<dyn Termination<S>>>,
}

impl<S: PlanningSolution> Debug for AndCompositeTermination<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AndCompositeTermination")
            .field("count", &self.terminations.len())
            .finish()
    }
}

impl<S: PlanningSolution> AndCompositeTermination<S> {
    pub fn new(terminations: Vec<Box<dyn Termination<S>>>) -> Self {
        Self { terminations }
    }
}

impl<S: PlanningSolution> Termination<S> for AndCompositeTermination<S> {
    fn is_terminated(&self, solver_scope: &SolverScope<S>) -> bool {
        !self.terminations.is_empty()
            && self.terminations.iter().all(|t| t.is_terminated(solver_scope))
    }
}
