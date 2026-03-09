//! Blanket Phase implementation for nested tuples.

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::scope::BestSolutionCallback;
use crate::scope::SolverScope;

use super::Phase;

/// Blanket impl: any `(Prev, P)` where both implement `Phase` is itself a `Phase`.
///
/// Combined with the `()` no-op impl, this covers all nested tuple arities:
/// - `((), P1)` — single phase
/// - `(((), P1), P2)` — two phases
/// - `((((), P1), P2), P3)` — three phases, etc.
///
/// Built by `SolverFactoryBuilder::with_phase()` which wraps `(self.phases, phase)`.
impl<S, D, BestCb, Prev, P> Phase<S, D, BestCb> for (Prev, P)
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    BestCb: BestSolutionCallback<S>,
    Prev: Phase<S, D, BestCb>,
    P: Phase<S, D, BestCb>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<'_, S, D, BestCb>) {
        self.0.solve(solver_scope);
        self.1.solve(solver_scope);
    }

    fn phase_type_name(&self) -> &'static str {
        "PhaseTuple"
    }
}
