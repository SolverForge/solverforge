// Variable Neighborhood Descent phase implementation.

use std::fmt::Debug;
use std::marker::PhantomData;
use std::time::Instant;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::{Director, RecordingDirector};
use tracing::info;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::{CandidateId, MoveCursor};
use crate::heuristic::selector::MoveSelector;
use crate::phase::control::{
    settle_search_interrupt, should_interrupt_evaluation, should_interrupt_generation,
    StepInterrupt,
};
use crate::phase::hard_delta::{hard_score_delta, HardScoreDelta};
use crate::phase::vnd::telemetry::{candidate_selector_label, VndProgress};
use crate::phase::Phase;
use crate::scope::ProgressCallback;
use crate::scope::{PhaseScope, SolverScope, StepScope};
use crate::stats::{format_duration, whole_units_per_second};

/// Variable Neighborhood Descent phase.
///
/// Wraps a tuple of move selectors (neighborhoods) and explores them in sequence,
/// restarting from the first whenever an improvement is found.
///
/// Uses macro-generated tuple implementations for zero type erasure.
///
/// # Type Parameters
/// * `T` - Tuple of move selectors
/// * `M` - The move type produced by all selectors
///
/// # Example
///
/// ```
/// use solverforge_solver::phase::vnd::VndPhase;
/// use solverforge_solver::heuristic::r#move::ChangeMove;
/// use solverforge_solver::heuristic::selector::ChangeMoveSelector;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SoftScore;
///
/// #[derive(Clone, Debug)]
/// struct MySolution {
///     values: Vec<Option<i32>>,
///     score: Option<SoftScore>,
/// }
///
/// impl PlanningSolution for MySolution {
///     type Score = SoftScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// fn get_value(s: &MySolution, idx: usize, _variable_index: usize) -> Option<i32> {
///     s.values.get(idx).copied().flatten()
/// }
/// fn set_value(s: &mut MySolution, idx: usize, _variable_index: usize, v: Option<i32>) {
///     if let Some(slot) = s.values.get_mut(idx) { *slot = v; }
/// }
///
/// type MyMove = ChangeMove<MySolution, i32>;
///
/// let selector = ChangeMoveSelector::simple(
///     get_value, set_value, 0,  0, "value", vec![1, 2, 3]
/// );
///
/// // Single neighborhood VND
/// let vnd: VndPhase<_, MyMove> = VndPhase::new((selector,));
/// ```
pub struct VndPhase<T, M>(pub T, PhantomData<fn() -> M>);

impl<T: Debug, M> Debug for VndPhase<T, M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("VndPhase").field(&self.0).finish()
    }
}

impl<T, M> VndPhase<T, M> {
    pub fn new(neighborhoods: T) -> Self {
        Self(neighborhoods, PhantomData)
    }
}

// Generates `Phase` implementations for VndPhase with tuple neighborhoods.
macro_rules! impl_vnd_phase {
    // Single neighborhood
    ($idx:tt: $MS:ident) => {
        impl<S, D, BestCb, M, $MS> Phase<S, D, BestCb> for VndPhase<($MS,), M>
        where
            S: PlanningSolution,
            D: Director<S>,
            BestCb: ProgressCallback<S>,
            M: Move<S>,
            $MS: MoveSelector<S, M>,
        {
            fn solve(&mut self, solver_scope: &mut SolverScope<S, D, BestCb>) {
                let phase_name = "Variable Neighborhood Descent";
                let mut phase_scope = PhaseScope::with_phase_type(solver_scope, 0, phase_name);
                let phase_index = phase_scope.phase_index();
                let mut current_score = phase_scope.calculate_score();
                let mut progress = VndProgress::new();

                info!(
                    event = "phase_start",
                    phase = phase_name,
                    phase_index = phase_index,
                    score = %current_score,
                );
                phase_scope.solver_scope().report_progress();

                loop {
                    let mut step_scope = StepScope::new(&mut phase_scope);
                    let mut cursor = (self.0).$idx.open_cursor(step_scope.score_director());

                    match find_best_improving_move(&mut cursor, &mut step_scope, &current_score, &mut progress) {
                        MoveSearchResult::Found(selected_move, selected_score, selector_index) => {
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
                        }
                        MoveSearchResult::NotFound => {
                            step_scope.complete();
                            break;
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
    };

    // Multiple neighborhoods
    ($($idx:tt: $MS:ident),+) => {
        impl<S, D, BestCb, M, $($MS),+> Phase<S, D, BestCb> for VndPhase<($($MS,)+), M>
        where
            S: PlanningSolution,
            D: Director<S>,
            BestCb: ProgressCallback<S>,
            M: Move<S>,
            $($MS: MoveSelector<S, M>,)+
        {
            fn solve(&mut self, solver_scope: &mut SolverScope<S, D, BestCb>) {
                const COUNT: usize = impl_vnd_phase!(@count $($idx),+);
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

                while k < COUNT {
                    let mut step_scope = StepScope::new(&mut phase_scope);
                    let search_result = match k {
                        $($idx => {
                            let mut cursor = (self.0).$idx.open_cursor(step_scope.score_director());
                            find_best_improving_move(&mut cursor, &mut step_scope, &current_score, &mut progress)
                        },)+
                        _ => MoveSearchResult::NotFound,
                    };

                    match search_result {
                        MoveSearchResult::Found(selected_move, selected_score, selector_index) => {
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
                            k = 0; // Restart from first neighborhood
                        }
                        MoveSearchResult::NotFound => {
                            step_scope.complete();
                            k += 1; // Try next neighborhood
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
    };

    // Helper: count tuple elements
    (@count $($idx:tt),+) => {
        0 $(+ { let _ = $idx; 1 })+
    };
}

/* Finds the index of the best improving move in the arena.

Returns `Some((index, score))` if an improving move is found, `None` otherwise.
*/
enum MoveSearchResult<M, Sc> {
    Found(M, Sc, Option<usize>),
    NotFound,
    Interrupted,
}

fn find_best_improving_move<S, D, BestCb, M, C>(
    cursor: &mut C,
    step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
    current_score: &S::Score,
    progress: &mut VndProgress,
) -> MoveSearchResult<M, S::Score>
where
    S: PlanningSolution,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
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
        let generation_started = Instant::now();
        let Some(candidate_index) = cursor.next_candidate() else {
            break;
        };
        let generation_elapsed = generation_started.elapsed();
        generated += 1;
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

        if should_interrupt_evaluation(step_scope, evaluated) {
            return MoveSearchResult::Interrupted;
        }
        evaluated += 1;
        let evaluation_started = Instant::now();
        if !mov.is_doable(step_scope.score_director()) {
            if let Some(selector_index) = selector_index {
                step_scope
                    .phase_scope_mut()
                    .record_selector_evaluated_move(selector_index, evaluation_started.elapsed());
                step_scope
                    .phase_scope_mut()
                    .record_selector_move_not_doable(selector_index);
            } else {
                step_scope
                    .phase_scope_mut()
                    .record_evaluated_move(evaluation_started.elapsed());
                step_scope.phase_scope_mut().record_move_not_doable();
            }
            progress.record_evaluated();
            progress.maybe_report(step_scope, current_score);
            continue;
        }

        let mut recording = RecordingDirector::new(step_scope.score_director_mut());
        mov.do_move(&mut recording);
        let move_score = recording.calculate_score();
        recording.undo_changes();
        step_scope.phase_scope_mut().record_score_calculation();

        let hard_delta = hard_score_delta(*current_score, move_score);
        match hard_delta {
            Some(HardScoreDelta::Improving) => {
                step_scope.phase_scope_mut().record_move_hard_improving();
            }
            Some(HardScoreDelta::Neutral) => {
                step_scope.phase_scope_mut().record_move_hard_neutral();
            }
            Some(HardScoreDelta::Worse) => {
                step_scope.phase_scope_mut().record_move_hard_worse();
            }
            None => {}
        }

        if mov.requires_hard_improvement() && hard_delta != Some(HardScoreDelta::Improving) {
            if let Some(selector_index) = selector_index {
                step_scope
                    .phase_scope_mut()
                    .record_selector_evaluated_move(selector_index, evaluation_started.elapsed());
                step_scope
                    .phase_scope_mut()
                    .record_selector_move_acceptor_rejected(selector_index);
            } else {
                step_scope
                    .phase_scope_mut()
                    .record_evaluated_move(evaluation_started.elapsed());
                step_scope.phase_scope_mut().record_move_acceptor_rejected();
            }
            progress.record_evaluated();
            progress.maybe_report(step_scope, current_score);
            continue;
        }

        if let Some(selector_index) = selector_index {
            step_scope
                .phase_scope_mut()
                .record_selector_evaluated_move(selector_index, evaluation_started.elapsed());
        } else {
            step_scope
                .phase_scope_mut()
                .record_evaluated_move(evaluation_started.elapsed());
        }
        progress.record_evaluated();
        progress.maybe_report(step_scope, current_score);

        if move_score > *current_score {
            match &best {
                Some((_, best_score)) if move_score > *best_score => {
                    best = Some((candidate_index, move_score));
                }
                None => {
                    best = Some((candidate_index, move_score));
                }
                _ => {}
            }
        }
    }

    match best {
        Some((index, score)) => {
            let selector_index = cursor.selector_index(index);
            MoveSearchResult::Found(cursor.take_candidate(index), score, selector_index)
        }
        None => MoveSearchResult::NotFound,
    }
}

impl_vnd_phase!(0: MS0);
impl_vnd_phase!(0: MS0, 1: MS1);
impl_vnd_phase!(0: MS0, 1: MS1, 2: MS2);
impl_vnd_phase!(0: MS0, 1: MS1, 2: MS2, 3: MS3);
impl_vnd_phase!(0: MS0, 1: MS1, 2: MS2, 3: MS3, 4: MS4);
impl_vnd_phase!(0: MS0, 1: MS1, 2: MS2, 3: MS3, 4: MS4, 5: MS5);
impl_vnd_phase!(0: MS0, 1: MS1, 2: MS2, 3: MS3, 4: MS4, 5: MS5, 6: MS6);
impl_vnd_phase!(0: MS0, 1: MS1, 2: MS2, 3: MS3, 4: MS4, 5: MS5, 6: MS6, 7: MS7);

#[cfg(test)]
#[path = "phase_tests.rs"]
mod tests;
