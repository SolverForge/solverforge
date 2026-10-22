use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::scope::{ProgressCallback, SolverScope};

/* Remaining in-phase evaluation budget: the smaller of the move-count and
score-calculation limits minus the work already counted, or `None` when the
phase sets neither limit. A window never exceeds it, so fixed-work limits stop
at the same candidate as sequential evaluation.
*/
pub(crate) fn remaining_evaluation_budget<S, D, BestCb>(
    solver_scope: &SolverScope<'_, S, D, BestCb>,
) -> Option<u64>
where
    S: PlanningSolution,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
{
    let stats = solver_scope.stats();
    let moves = solver_scope
        .inphase_move_count_limit
        .map(|limit| limit.saturating_sub(stats.moves_evaluated));
    let score_calculations = solver_scope
        .inphase_score_calc_count_limit
        .map(|limit| limit.saturating_sub(stats.score_calculations));
    match (moves, score_calculations) {
        (Some(moves), Some(score_calculations)) => Some(moves.min(score_calculations)),
        (moves, score_calculations) => moves.or(score_calculations),
    }
}
