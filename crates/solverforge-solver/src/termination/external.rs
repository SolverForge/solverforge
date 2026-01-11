//! External termination via AtomicBool flag.

use std::sync::atomic::{AtomicBool, Ordering};

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use super::Termination;
use crate::scope::SolverScope;

/// Terminates when an external flag is set.
///
/// Allows external code to request termination by setting an `AtomicBool`.
///
/// # Example
///
/// ```
/// use std::sync::atomic::AtomicBool;
/// use solverforge_solver::termination::ExternalTermination;
///
/// let flag = AtomicBool::new(false);
/// let term = ExternalTermination::new(&flag);
///
/// // Later: flag.store(true, Ordering::SeqCst);
/// ```
#[derive(Debug)]
pub struct ExternalTermination<'a> {
    flag: &'a AtomicBool,
}

impl<'a> ExternalTermination<'a> {
    /// Creates a termination that checks the given flag.
    pub fn new(flag: &'a AtomicBool) -> Self {
        Self { flag }
    }
}

// SAFETY: The AtomicBool reference is thread-safe by definition.
unsafe impl Send for ExternalTermination<'_> {}

impl<S: PlanningSolution, D: ScoreDirector<S>> Termination<S, D> for ExternalTermination<'_> {
    fn is_terminated(&self, _solver_scope: &SolverScope<S, D>) -> bool {
        self.flag.load(Ordering::Relaxed)
    }
}
