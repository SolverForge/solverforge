use std::fmt::{self, Debug};
use std::marker::PhantomData;
use std::time::Instant;

use rand::RngExt;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;
use tracing::info;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::{CandidateId, MoveCursor, MoveStreamContext};
use crate::heuristic::selector::MoveSelector;
use crate::phase::control::{
    settle_search_interrupt, should_interrupt_after_step, should_interrupt_before_candidate,
    should_interrupt_before_evaluation, StepInterrupt,
};
use crate::phase::localsearch::evaluation::{
    evaluate_candidate, record_evaluated_move, CandidateEvaluation,
};
use crate::phase::localsearch::vnd::telemetry::{candidate_selector_label, VndProgress};
use crate::phase::Phase;
use crate::scope::{PhaseScope, ProgressCallback, SolverScope, StepScope};
use crate::stats::{format_duration, whole_units_per_second};

pub(crate) struct VndPhase<S, M, MS> {
    neighborhoods: Vec<MS>,
    step_limit: Option<u64>,
    _phantom: PhantomData<(fn() -> S, fn() -> M)>,
}

impl<S, M, MS> VndPhase<S, M, MS> {
    pub(crate) fn new(neighborhoods: Vec<MS>, step_limit: Option<u64>) -> Self {
        Self {
            neighborhoods,
            step_limit,
            _phantom: PhantomData,
        }
    }
}

impl<S, M, MS: Debug> Debug for VndPhase<S, M, MS> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VndPhase")
            .field("neighborhoods", &self.neighborhoods)
            .field("step_limit", &self.step_limit)
            .finish()
    }
}

impl<S, D, ProgressCb, M, MS> Phase<S, D, ProgressCb> for VndPhase<S, M, MS>
where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
    M: Move<S>,
    MS: MoveSelector<S, M>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<'_, S, D, ProgressCb>) {
        let phase_name = "Variable Neighborhood Descent";
        let mut phase_scope = PhaseScope::with_phase_type(solver_scope, 0, phase_name);
        let phase_index = phase_scope.phase_index();
        let mut current_score = phase_scope.calculate_score();
        let mut progress = VndProgress::new();
        let mut k = 0usize;

        info!(
            event = "phase_start",
            phase = phase_name,
            phase_index = phase_index,
            score = %current_score,
        );
        phase_scope.solver_scope().report_progress();

        while k < self.neighborhoods.len() {
            if phase_scope.solver_scope_mut().should_terminate() {
                break;
            }

            if let Some(limit) = self.step_limit {
                if phase_scope.step_count() >= limit {
                    break;
                }
            }

            let mut step_scope = StepScope::new(&mut phase_scope);
            let stream_context = MoveStreamContext::new(
                step_scope.step_index(),
                step_scope
                    .phase_scope_mut()
                    .solver_scope_mut()
                    .rng()
                    .random::<u64>(),
                None,
            );
            let mut cursor = self.neighborhoods[k]
                .open_cursor_with_context(step_scope.score_director(), stream_context);

            match find_best_improving_move(
                &mut cursor,
                &mut step_scope,
                &current_score,
                &mut progress,
            ) {
                MoveSearchResult::Found(selected_index, selected_score, selector_index) => {
                    let selected_move = cursor
                        .candidate(selected_index)
                        .expect("selected VND candidate id must remain borrowable until commit");
                    step_scope.apply_committed_move(&selected_move);
                    if let Some(selector_index) = selector_index {
                        step_scope
                            .phase_scope_mut()
                            .record_selector_move_accepted(selector_index);
                        step_scope
                            .phase_scope_mut()
                            .record_selector_move_applied(selector_index);
                    } else {
                        step_scope.phase_scope_mut().record_move_accepted();
                        step_scope.phase_scope_mut().record_move_applied();
                    }
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

        phase_scope.solver_scope().report_progress();
        let duration = phase_scope.elapsed();
        let steps = phase_scope.step_count();
        let speed = whole_units_per_second(progress.moves_evaluated(), duration);
        let stats = phase_scope.stats();
        info!(
            event = "phase_end",
            phase = phase_name,
            phase_index = phase_index,
            duration = %format_duration(duration),
            steps = steps,
            moves_generated = stats.moves_generated,
            moves_evaluated = stats.moves_evaluated,
            moves_accepted = stats.moves_accepted,
            score_calculations = stats.score_calculations,
            generation_time = %format_duration(stats.generation_time()),
            evaluation_time = %format_duration(stats.evaluation_time()),
            speed = speed,
            score = %current_score,
        );
    }

    fn phase_type_name(&self) -> &'static str {
        "VariableNeighborhoodDescent"
    }
}

enum MoveSearchResult<Sc> {
    Found(CandidateId, Sc, Option<usize>),
    NotFound,
    Interrupted,
}

fn find_best_improving_move<S, D, ProgressCb, M, C>(
    cursor: &mut C,
    step_scope: &mut StepScope<'_, '_, '_, S, D, ProgressCb>,
    current_score: &S::Score,
    progress: &mut VndProgress,
) -> MoveSearchResult<S::Score>
where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    let mut best: Option<(CandidateId, S::Score)> = None;

    loop {
        if should_interrupt_before_candidate(step_scope) {
            return MoveSearchResult::Interrupted;
        }
        let generation_started = Instant::now();
        let Some(candidate_index) = cursor.next_candidate() else {
            break;
        };
        let generation_elapsed = generation_started.elapsed();
        let mov = cursor
            .candidate(candidate_index)
            .expect("discovered candidate id must remain borrowable");
        let selector_index = cursor.selector_index(candidate_index);
        let selector_label = selector_index.map(|_| candidate_selector_label(&mov));
        if let Some(selector_index) = selector_index {
            step_scope
                .phase_scope_mut()
                .record_selector_generated_move_with_label(
                    selector_index,
                    selector_label.as_deref().unwrap_or("selector"),
                    generation_elapsed,
                );
        } else {
            step_scope
                .phase_scope_mut()
                .record_generated_move(generation_elapsed);
        }
        progress.record_generated();

        if should_interrupt_before_evaluation(step_scope) {
            return MoveSearchResult::Interrupted;
        }
        let evaluation_started = Instant::now();
        let move_score = match evaluate_candidate(
            &mov,
            step_scope,
            *current_score,
            selector_index,
            evaluation_started,
        ) {
            CandidateEvaluation::Scored(score) => score,
            CandidateEvaluation::NotDoable | CandidateEvaluation::RejectedByHardImprovement => {
                progress.record_evaluated();
                progress.maybe_report(step_scope, current_score);
                continue;
            }
        };

        record_evaluated_move(step_scope, selector_index, evaluation_started);
        progress.record_evaluated();
        progress.maybe_report(step_scope, current_score);

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

    if should_interrupt_after_step(step_scope) {
        return MoveSearchResult::Interrupted;
    }

    match best {
        Some((index, score)) => {
            let selector_index = cursor.selector_index(index);
            MoveSearchResult::Found(index, score, selector_index)
        }
        None => MoveSearchResult::NotFound,
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
