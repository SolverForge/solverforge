use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::phase::Phase;
use crate::scope::SolverScope;

#[derive(Debug, Clone)]
pub(super) struct NoOpPhase;

impl<S, D, ProgressCb> Phase<S, D, ProgressCb> for NoOpPhase
where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: crate::scope::ProgressCallback<S>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D, ProgressCb>) {
        solver_scope.update_best_solution();
    }

    fn phase_type_name(&self) -> &'static str {
        "NoOpPhase"
    }
}
