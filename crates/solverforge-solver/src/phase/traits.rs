// Phase trait definition.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::scope::ProgressCallback;
use crate::scope::SolverScope;

/// A phase of the solving process.
///
/// Phases are executed in sequence by the solver. Each phase has its own
/// strategy for exploring or constructing solutions.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `D` - The score director type
/// * `BestCb` - The best-solution callback type (default `()`)
pub trait Phase<S: PlanningSolution, D: Director<S>, BestCb: ProgressCallback<S> = ()>:
    Send + Debug
{
    /* Executes this phase.

    The phase should inspect the current state through immutable accessors,
    use `trial(...)` for speculative work, use `mutate(...)` for committed
    arbitrary changes, and update the best solution when improvements are
    found.
    */
    fn solve(&mut self, solver_scope: &mut SolverScope<'_, S, D, BestCb>);

    fn phase_type_name(&self) -> &'static str;
}

// Unit type implements Phase as a no-op (empty phase list).
impl<S: PlanningSolution, D: Director<S>, BestCb: ProgressCallback<S>> Phase<S, D, BestCb> for () {
    fn solve(&mut self, _solver_scope: &mut SolverScope<'_, S, D, BestCb>) {
        // No-op: empty phase list does nothing
    }

    fn phase_type_name(&self) -> &'static str {
        "NoOp"
    }
}
