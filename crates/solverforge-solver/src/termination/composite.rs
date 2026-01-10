//! Composite termination conditions (AND/OR).

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use super::Termination;
use crate::scope::SolverScope;

/// Combines multiple terminations with OR logic.
pub struct OrCompositeTermination<S: PlanningSolution, D: ScoreDirector<S>> {
    terminations: Vec<Box<dyn Termination<S, D>>>,
}

impl<S: PlanningSolution, D: ScoreDirector<S>> Debug for OrCompositeTermination<S, D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrCompositeTermination")
            .field("count", &self.terminations.len())
            .finish()
    }
}

impl<S: PlanningSolution, D: ScoreDirector<S>> OrCompositeTermination<S, D> {
    pub fn new(terminations: Vec<Box<dyn Termination<S, D>>>) -> Self {
        Self { terminations }
    }
}

impl<S: PlanningSolution, D: ScoreDirector<S>> Termination<S, D> for OrCompositeTermination<S, D> {
    fn is_terminated(&self, solver_scope: &SolverScope<S, D>) -> bool {
        self.terminations
            .iter()
            .any(|t| t.is_terminated(solver_scope))
    }
}

/// Combines multiple terminations with AND logic.
pub struct AndCompositeTermination<S: PlanningSolution, D: ScoreDirector<S>> {
    terminations: Vec<Box<dyn Termination<S, D>>>,
}

impl<S: PlanningSolution, D: ScoreDirector<S>> Debug for AndCompositeTermination<S, D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AndCompositeTermination")
            .field("count", &self.terminations.len())
            .finish()
    }
}

impl<S: PlanningSolution, D: ScoreDirector<S>> AndCompositeTermination<S, D> {
    pub fn new(terminations: Vec<Box<dyn Termination<S, D>>>) -> Self {
        Self { terminations }
    }
}

impl<S: PlanningSolution, D: ScoreDirector<S>> Termination<S, D> for AndCompositeTermination<S, D> {
    fn is_terminated(&self, solver_scope: &SolverScope<S, D>) -> bool {
        !self.terminations.is_empty()
            && self
                .terminations
                .iter()
                .all(|t| t.is_terminated(solver_scope))
    }
}
