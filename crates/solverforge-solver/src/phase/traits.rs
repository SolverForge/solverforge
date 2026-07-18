// Phase trait definition.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::scope::ProgressCallback;
use crate::scope::SolverScope;
use crate::stats::CandidateTracePhasePlan;

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
    use typed move undo for speculative work, use `mutate(...)` for committed
    arbitrary changes, and update the best solution when improvements are found.
    */
    fn solve(&mut self, solver_scope: &mut SolverScope<'_, S, D, BestCb>);

    fn phase_type_name(&self) -> &'static str;

    /// Whether this phase owns a structural completion gate that must pass
    /// before the initial working solution can be published as a best solution.
    fn defers_initial_best_solution_publication(&self) -> bool {
        false
    }

    /// Runs once when the enclosing solver reaches its terminal boundary.
    ///
    /// The solver invokes this after it has attempted every configured
    /// top-level phase and before it snapshots final statistics. It runs for
    /// ordinary completion as well as cancellation or configured termination,
    /// including phases whose [`Self::solve`] method was skipped. Implementors
    /// can use it to publish terminal-only diagnostics; the default preserves
    /// the existing phase lifecycle unchanged.
    fn on_solver_terminal(&mut self, _solver_scope: &mut SolverScope<'_, S, D, BestCb>) {}

    /// Returns the exact resolved-plan provenance this phase can prove.
    ///
    /// A phase that does not override this method is deliberately marked
    /// opaque.  Candidate traces must never synthesize a lookalike plan for
    /// custom or foreign phases; a consumer can then reject that comparison
    /// rather than accepting a misleading fallback.
    fn candidate_trace_plan(&self) -> CandidateTracePhasePlan {
        CandidateTracePhasePlan::opaque(self.phase_type_name())
    }
}

// Unit type implements Phase as a no-op (empty phase list).
impl<S: PlanningSolution, D: Director<S>, BestCb: ProgressCallback<S>> Phase<S, D, BestCb> for () {
    fn solve(&mut self, _solver_scope: &mut SolverScope<'_, S, D, BestCb>) {
        // No-op: empty phase list does nothing
    }

    fn phase_type_name(&self) -> &'static str {
        "NoOp"
    }

    fn candidate_trace_plan(&self) -> CandidateTracePhasePlan {
        CandidateTracePhasePlan::known(
            "solverforge.phase.no_op",
            std::iter::empty::<(String, String)>(),
            Vec::new(),
        )
    }
}
