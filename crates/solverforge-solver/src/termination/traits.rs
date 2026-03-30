// Termination trait definition.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::scope::ProgressCallback;
use crate::scope::SolverScope;

/// Trait for determining when to stop solving.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `D` - The score director type
/// * `BestCb` - The best-solution callback type (default `()`)
pub trait Termination<S: PlanningSolution, D: Director<S>, BestCb: ProgressCallback<S> = ()>:
    Send + Debug
{
    // Returns true if solving should terminate.
    fn is_terminated(&self, solver_scope: &SolverScope<S, D, BestCb>) -> bool;

    /* Installs this termination's limit as an in-phase limit on the solver scope.

    This allows the termination to fire inside the phase step loop (T1 fix).
    The default implementation is a no-op.
    */
    fn install_inphase_limits(&self, _solver_scope: &mut SolverScope<S, D, BestCb>) {}
}
