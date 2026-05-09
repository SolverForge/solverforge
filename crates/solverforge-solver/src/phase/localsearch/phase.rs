// Local search phase implementation.

use std::fmt::Debug;
use std::marker::PhantomData;
use std::time::Instant;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;
use tracing::{debug, info, trace};

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::{MoveCandidateRef, MoveCursor};
use crate::heuristic::selector::MoveSelector;
use crate::phase::control::{
    settle_search_interrupt, should_interrupt_evaluation, should_interrupt_generation,
    StepInterrupt,
};
use crate::phase::localsearch::evaluation::{
    evaluate_candidate, record_evaluated_move, CandidateEvaluation,
};
use crate::phase::localsearch::{Acceptor, LocalSearchForager};
use crate::phase::Phase;
use crate::scope::ProgressCallback;
use crate::scope::{PhaseScope, SolverScope, StepScope};
use crate::stats::{format_duration, whole_units_per_second};

/// Local search phase that improves an existing solution.
///
/// This phase iteratively:
/// 1. Generates candidate moves into an arena
/// 2. Evaluates each move by index
/// 3. Accepts/rejects based on the acceptor
/// 4. Takes ownership of the best accepted move from arena
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `M` - The move type
/// * `MS` - The move selector type
/// * `A` - The acceptor type
/// * `Fo` - The forager type
///
/// # Zero-Clone Design
///
/// Uses index-based foraging. The forager stores `(usize, Score)` pairs.
/// When a move is selected, ownership transfers via `arena.take(index)`.
/// Moves are NEVER cloned.
pub struct LocalSearchPhase<S, M, MS, A, Fo>
where
    S: PlanningSolution,
    M: Move<S>,
    MS: MoveSelector<S, M>,
    A: Acceptor<S>,
    Fo: LocalSearchForager<S, M>,
{
    move_selector: MS,
    acceptor: A,
    forager: Fo,
    step_limit: Option<u64>,
    _phantom: PhantomData<fn() -> (S, M)>,
}

fn candidate_selector_label<S, M>(mov: &MoveCandidateRef<'_, S, M>) -> String
where
    S: PlanningSolution,
    M: Move<S>,
{
    if mov.variable_name() == "compound_scalar" || mov.variable_name() == "conflict_repair" {
        return mov.variable_name().to_string();
    }
    let mut label = None;
    mov.for_each_affected_entity(&mut |affected| {
        if label.is_none() {
            label = Some(affected.variable_name.to_string());
        }
    });
    label.unwrap_or_else(|| "move".to_string())
}

impl<S, M, MS, A, Fo> LocalSearchPhase<S, M, MS, A, Fo>
where
    S: PlanningSolution,
    M: Move<S> + 'static,
    MS: MoveSelector<S, M>,
    A: Acceptor<S>,
    Fo: LocalSearchForager<S, M>,
{
    pub fn new(move_selector: MS, acceptor: A, forager: Fo, step_limit: Option<u64>) -> Self {
        Self {
            move_selector,
            acceptor,
            forager,
            step_limit,
            _phantom: PhantomData,
        }
    }
}

impl<S, M, MS, A, Fo> Debug for LocalSearchPhase<S, M, MS, A, Fo>
where
    S: PlanningSolution,
    M: Move<S>,
    MS: MoveSelector<S, M> + Debug,
    A: Acceptor<S> + Debug,
    Fo: LocalSearchForager<S, M> + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalSearchPhase")
            .field("move_selector", &self.move_selector)
            .field("acceptor", &self.acceptor)
            .field("forager", &self.forager)
            .field("step_limit", &self.step_limit)
            .finish()
    }
}

impl<S, D, BestCb, M, MS, A, Fo> Phase<S, D, BestCb> for LocalSearchPhase<S, M, MS, A, Fo>
where
    S: PlanningSolution,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
    M: Move<S>,
    MS: MoveSelector<S, M>,
    A: Acceptor<S>,
    Fo: LocalSearchForager<S, M>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D, BestCb>) {
        let mut phase_scope = PhaseScope::new(solver_scope, 0);
        let phase_index = phase_scope.phase_index();

        // Calculate initial score
        let mut last_step_score = phase_scope.calculate_score();

        info!(
            event = "phase_start",
            phase = "Local Search",
            phase_index = phase_index,
            score = %last_step_score,
        );

        // Notify acceptor of phase start
        self.acceptor.phase_started(&last_step_score);

        let start_time = Instant::now();
        let mut local_moves_generated: u64 = 0;
        let mut local_moves_evaluated: u64 = 0;
        let mut last_progress_time = Instant::now();
        let mut last_progress_moves: u64 = 0;
        loop {
            // Check early termination
            if phase_scope.solver_scope_mut().should_terminate() {
                break;
            }

            // Check step limit
            if let Some(limit) = self.step_limit {
                if phase_scope.step_count() >= limit {
                    break;
                }
            }

            let mut step_scope = StepScope::new(&mut phase_scope);

            /* Reset forager and acceptor for this step.
            Pass best and last-step scores so foragers that implement
            pick-early-on-improvement strategies know their reference targets.
            */
            let best_score = step_scope
                .phase_scope()
                .solver_scope()
                .best_score()
                .copied()
                .unwrap_or(last_step_score);
            self.forager.step_started(best_score, last_step_score);
            self.acceptor.step_started();
            let requires_move_signatures = self.acceptor.requires_move_signatures();

            let mut interrupted_step = false;
            let mut generated_moves = 0usize;
            let mut evaluated_moves = 0usize;
            let mut accepted_moves_this_step = 0u64;
            let generation_started = Instant::now();
            let mut cursor = self.move_selector.open_cursor(step_scope.score_director());
            step_scope
                .phase_scope_mut()
                .record_generation_time(generation_started.elapsed());

            while !self.forager.is_quit_early() {
                if should_interrupt_generation(&step_scope, generated_moves) {
                    interrupted_step = true;
                    break;
                }

                let generation_started = Instant::now();
                let Some(candidate_id) = cursor.next_candidate() else {
                    break;
                };
                let selector_index = cursor.selector_index(candidate_id);
                let mov = cursor
                    .candidate(candidate_id)
                    .expect("discovered candidate id must remain borrowable");
                let selector_label = selector_index.map(|_| candidate_selector_label(&mov));
                let generation_elapsed = generation_started.elapsed();
                generated_moves += 1;
                local_moves_generated += 1;
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

                if should_interrupt_evaluation(&step_scope, evaluated_moves) {
                    interrupted_step = true;
                    break;
                }
                evaluated_moves += 1;
                local_moves_evaluated += 1;

                if local_moves_evaluated & 0x1FFF == 0 {
                    let now = Instant::now();
                    if now.duration_since(last_progress_time).as_secs() >= 1 {
                        let current_speed = whole_units_per_second(
                            local_moves_evaluated - last_progress_moves,
                            now.duration_since(last_progress_time),
                        );
                        debug!(
                            event = "progress",
                            steps = step_scope.step_index(),
                            moves_generated = local_moves_generated,
                            moves_evaluated = local_moves_evaluated,
                            moves_accepted = step_scope.phase_scope().solver_scope().stats().moves_accepted,
                            score_calculations = step_scope.phase_scope().solver_scope().stats().score_calculations,
                            speed = current_speed,
                            acceptance_rate = format!(
                                "{:.1}%",
                                step_scope.phase_scope().solver_scope().stats().acceptance_rate() * 100.0
                            ),
                            current_score = %last_step_score,
                            best_score = %best_score,
                        );
                        step_scope.phase_scope().solver_scope().report_progress();
                        last_progress_time = now;
                        last_progress_moves = local_moves_evaluated;
                    }
                }

                let evaluation_started = Instant::now();
                let move_score = match evaluate_candidate(
                    &mov,
                    &mut step_scope,
                    last_step_score,
                    selector_index,
                    evaluation_started,
                ) {
                    CandidateEvaluation::Scored(score) => score,
                    CandidateEvaluation::NotDoable
                    | CandidateEvaluation::RejectedByHardImprovement => continue,
                };

                let move_signature = if requires_move_signatures {
                    Some(mov.tabu_signature(step_scope.score_director()))
                } else {
                    None
                };

                let accepted = self.acceptor.is_accepted(
                    &last_step_score,
                    &move_score,
                    move_signature.as_ref(),
                );

                record_evaluated_move(&mut step_scope, selector_index, evaluation_started);
                if accepted {
                    if let Some(selector_index) = selector_index {
                        step_scope
                            .phase_scope_mut()
                            .record_selector_move_accepted(selector_index);
                    } else {
                        step_scope.phase_scope_mut().record_move_accepted();
                    }
                    accepted_moves_this_step += 1;
                } else if let Some(selector_index) = selector_index {
                    step_scope
                        .phase_scope_mut()
                        .record_selector_move_acceptor_rejected(selector_index);
                } else {
                    step_scope.phase_scope_mut().record_move_acceptor_rejected();
                }

                trace!(
                    event = "step",
                    step = step_scope.step_index(),
                    move_index = candidate_id.index(),
                    score = %move_score,
                    accepted = accepted,
                );

                if accepted {
                    self.forager.add_move_index(candidate_id, move_score);
                }
            }

            if interrupted_step {
                match settle_search_interrupt(&mut step_scope) {
                    StepInterrupt::Restart => continue,
                    StepInterrupt::TerminatePhase => break,
                }
            }

            // Pick the best accepted move index
            let mut accepted_move_signature = None;
            if let Some((selected_index, selected_score)) = self.forager.pick_move_index() {
                let selector_index = cursor.selector_index(selected_index);
                let selected_move = cursor.take_candidate(selected_index);
                if requires_move_signatures {
                    accepted_move_signature =
                        Some(selected_move.tabu_signature(step_scope.score_director()));
                }
                step_scope.apply_committed_move(&selected_move);
                if let Some(selector_index) = selector_index {
                    step_scope
                        .phase_scope_mut()
                        .record_selector_move_applied(selector_index);
                } else {
                    step_scope.phase_scope_mut().record_move_applied();
                }
                step_scope.set_step_score(selected_score);

                // Update last step score
                last_step_score = selected_score;

                // Update best solution if improved
                step_scope.phase_scope_mut().update_best_solution();
                if accepted_moves_this_step > 1 {
                    step_scope
                        .phase_scope_mut()
                        .record_moves_forager_ignored(accepted_moves_this_step - 1);
                }
            } else if accepted_moves_this_step > 0 {
                step_scope
                    .phase_scope_mut()
                    .record_moves_forager_ignored(accepted_moves_this_step);
            }
            /* else: no accepted moves this step — that's fine, the acceptor
            history still needs to advance so Late Acceptance / SA / etc.
            can eventually escape the local optimum.
            */

            /* Always notify acceptor that step ended. For stateful acceptors
            (Late Acceptance, Simulated Annealing, Great Deluge, SCHC),
            the history must advance every step — even steps where no move
            was accepted — otherwise the acceptor state stalls.
            */
            self.acceptor
                .step_ended(&last_step_score, accepted_move_signature.as_ref());

            step_scope.complete();
        }

        // Notify acceptor of phase end
        self.acceptor.phase_ended();

        let duration = start_time.elapsed();
        let steps = phase_scope.step_count();
        let stats = phase_scope.stats();
        let speed = whole_units_per_second(stats.moves_evaluated, duration);
        let acceptance_rate = stats.acceptance_rate() * 100.0;
        let calc_speed = whole_units_per_second(stats.score_calculations, duration);

        let best_score_str = phase_scope
            .solver_scope()
            .best_score()
            .map(|s| format!("{}", s))
            .unwrap_or_else(|| "none".to_string());

        info!(
            event = "phase_end",
            phase = "Local Search",
            phase_index = phase_index,
            duration = %format_duration(duration),
            steps = steps,
            moves_generated = stats.moves_generated,
            moves_evaluated = stats.moves_evaluated,
            moves_accepted = stats.moves_accepted,
            score_calculations = stats.score_calculations,
            generation_time = %format_duration(stats.generation_time()),
            evaluation_time = %format_duration(stats.evaluation_time()),
            moves_speed = speed,
            calc_speed = calc_speed,
            acceptance_rate = format!("{:.1}%", acceptance_rate),
            score = best_score_str,
        );
    }

    fn phase_type_name(&self) -> &'static str {
        "LocalSearch"
    }
}

#[cfg(test)]
mod tests;
