use std::fmt::{self, Debug};
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::{Director, RecordingDirector};

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::{CandidateId, MoveCursor};
use crate::heuristic::selector::MoveSelector;
use crate::phase::control::{
    settle_search_interrupt, should_interrupt_evaluation, should_interrupt_generation,
    StepInterrupt,
};
use crate::phase::hard_delta::{hard_score_delta, HardScoreDelta};
use crate::phase::Phase;
use crate::scope::{PhaseScope, ProgressCallback, SolverScope, StepScope};

pub struct DynamicVndPhase<S, M, MS> {
    neighborhoods: Vec<MS>,
    _phantom: PhantomData<(fn() -> S, fn() -> M)>,
}

impl<S, M, MS> DynamicVndPhase<S, M, MS> {
    pub fn new(neighborhoods: Vec<MS>) -> Self {
        Self {
            neighborhoods,
            _phantom: PhantomData,
        }
    }
}

impl<S, M, MS: Debug> Debug for DynamicVndPhase<S, M, MS> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DynamicVndPhase")
            .field("neighborhoods", &self.neighborhoods)
            .finish()
    }
}

impl<S, D, ProgressCb, M, MS> Phase<S, D, ProgressCb> for DynamicVndPhase<S, M, MS>
where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
    M: Move<S>,
    MS: MoveSelector<S, M>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<'_, S, D, ProgressCb>) {
        let mut phase_scope = PhaseScope::new(solver_scope, 0);
        let mut current_score = phase_scope.calculate_score();
        let mut k = 0usize;

        while k < self.neighborhoods.len() {
            let mut step_scope = StepScope::new(&mut phase_scope);
            let mut cursor = self.neighborhoods[k].open_cursor(step_scope.score_director());

            match find_best_improving_move(&mut cursor, &mut step_scope, &current_score) {
                MoveSearchResult::Found(selected_move, selected_score) => {
                    step_scope.apply_committed_move(&selected_move);
                    step_scope.phase_scope_mut().record_move_applied();
                    step_scope.set_step_score(selected_score);
                    current_score = selected_score;
                    step_scope.phase_scope_mut().update_best_solution();
                    step_scope.complete();
                    k = 0;
                }
                MoveSearchResult::NotFound => {
                    step_scope.complete();
                    k += 1;
                }
                MoveSearchResult::Interrupted => match settle_search_interrupt(&mut step_scope) {
                    StepInterrupt::Restart => continue,
                    StepInterrupt::TerminatePhase => break,
                },
            }
        }
    }

    fn phase_type_name(&self) -> &'static str {
        "VariableNeighborhoodDescent"
    }
}

enum MoveSearchResult<M, Sc> {
    Found(M, Sc),
    NotFound,
    Interrupted,
}

fn find_best_improving_move<S, D, ProgressCb, M, C>(
    cursor: &mut C,
    step_scope: &mut StepScope<'_, '_, '_, S, D, ProgressCb>,
    current_score: &S::Score,
) -> MoveSearchResult<M, S::Score>
where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    let mut best: Option<(CandidateId, S::Score)> = None;

    let mut generated = 0usize;
    let mut evaluated = 0usize;
    loop {
        if should_interrupt_generation(step_scope, generated) {
            return MoveSearchResult::Interrupted;
        }
        let Some(candidate_index) = cursor.next_candidate() else {
            break;
        };
        generated += 1;
        let mov = cursor
            .candidate(candidate_index)
            .expect("discovered candidate id must remain borrowable");

        if !mov.is_doable(step_scope.score_director()) {
            continue;
        }
        if should_interrupt_evaluation(step_scope, evaluated) {
            return MoveSearchResult::Interrupted;
        }
        evaluated += 1;

        let mut recording = RecordingDirector::new(step_scope.score_director_mut());
        mov.do_move(&mut recording);
        let move_score = recording.calculate_score();
        recording.undo_changes();

        if mov.requires_hard_improvement()
            && hard_score_delta(*current_score, move_score) != Some(HardScoreDelta::Improving)
        {
            continue;
        }

        if move_score > *current_score {
            match &best {
                Some((_, best_score)) if move_score > *best_score => {
                    best = Some((candidate_index, move_score));
                }
                None => best = Some((candidate_index, move_score)),
                _ => {}
            }
        }
    }

    match best {
        Some((index, score)) => MoveSearchResult::Found(cursor.take_candidate(index), score),
        None => MoveSearchResult::NotFound,
    }
}

#[cfg(test)]
#[path = "dynamic_vnd_tests.rs"]
mod tests;
