use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::{Director, RecordingDirector};

use crate::heuristic::r#move::Move;

pub(crate) fn evaluate_trial_move<S, D, M>(score_director: &mut D, m: &M) -> S::Score
where
    S: PlanningSolution,
    D: Director<S>,
    M: Move<S>,
{
    let mut recording = RecordingDirector::new(score_director);
    m.do_move(&mut recording);
    let score = recording.calculate_score();
    recording.undo_changes();
    score
}
