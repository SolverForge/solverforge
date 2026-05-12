use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;

pub(crate) fn evaluate_trial_move<S, D, M>(score_director: &mut D, m: &M) -> S::Score
where
    S: PlanningSolution,
    D: Director<S>,
    M: Move<S>,
{
    let score_state = score_director.snapshot_score_state();
    let undo = m.do_move(score_director);
    let score = score_director.calculate_score();
    m.undo_move(score_director, undo);
    score_director.restore_score_state(score_state);
    score
}
