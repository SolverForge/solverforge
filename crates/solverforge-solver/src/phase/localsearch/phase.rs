//! Local search phase implementation.

use std::fmt::Debug;
use std::marker::PhantomData;
use std::time::Instant;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::{RecordingScoreDirector, ScoreDirector};
use tracing::{debug, info, trace};

use crate::heuristic::r#move::{Move, MoveArena};
use crate::heuristic::selector::MoveSelector;
use crate::phase::localsearch::{Acceptor, LocalSearchForager};
use crate::phase::Phase;
use crate::scope::{PhaseScope, SolverScope, StepScope};

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
    Fo: LocalSearchForager<S>,
{
    move_selector: MS,
    acceptor: A,
    forager: Fo,
    arena: MoveArena<M>,
    step_limit: Option<u64>,
    _phantom: PhantomData<fn() -> (S, M)>,
}

impl<S, M, MS, A, Fo> LocalSearchPhase<S, M, MS, A, Fo>
where
    S: PlanningSolution,
    M: Move<S> + 'static,
    MS: MoveSelector<S, M>,
    A: Acceptor<S>,
    Fo: LocalSearchForager<S>,
{
    /// Creates a new local search phase.
    pub fn new(move_selector: MS, acceptor: A, forager: Fo, step_limit: Option<u64>) -> Self {
        Self {
            move_selector,
            acceptor,
            forager,
            arena: MoveArena::new(),
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
    Fo: LocalSearchForager<S> + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalSearchPhase")
            .field("move_selector", &self.move_selector)
            .field("acceptor", &self.acceptor)
            .field("forager", &self.forager)
            .field("arena", &self.arena)
            .field("step_limit", &self.step_limit)
            .finish()
    }
}

impl<S, D, M, MS, A, Fo> Phase<S, D> for LocalSearchPhase<S, M, MS, A, Fo>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    M: Move<S>,
    MS: MoveSelector<S, M>,
    A: Acceptor<S>,
    Fo: LocalSearchForager<S>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D>) {
        let phase_start = Instant::now();
        let mut phase_scope = PhaseScope::new(solver_scope, 0);

        // Calculate initial score
        let mut last_step_score = phase_scope.calculate_score();

        info!(
            event = "local_search_phase_start",
            initial_score = %last_step_score,
            step_limit = ?self.step_limit,
        );

        // Notify acceptor of phase start
        self.acceptor.phase_started(&last_step_score);

        loop {
            // Check early termination
            if phase_scope.solver_scope().is_terminate_early() {
                debug!(event = "local_search_terminated_early", reason = "termination_flag");
                break;
            }

            // Check step limit
            if let Some(limit) = self.step_limit {
                if phase_scope.step_count() >= limit {
                    debug!(
                        event = "local_search_step_limit_reached",
                        step_count = phase_scope.step_count(),
                        limit = limit,
                    );
                    break;
                }
            }

            let step_start = Instant::now();
            let mut step_scope = StepScope::new(&mut phase_scope);
            let step_number = step_scope.phase_scope().step_count();

            // Reset forager for this step
            self.forager.step_started();

            // Reset arena and populate with moves - O(1) reset
            self.arena.reset();
            self.arena
                .extend(self.move_selector.iter_moves(step_scope.score_director()));

            let move_count = self.arena.len();
            trace!(
                event = "local_search_step_start",
                step = step_number,
                candidate_moves = move_count,
            );

            let mut evaluated_count = 0;
            let mut accepted_count = 0;

            // Evaluate moves by index
            for i in 0..self.arena.len() {
                let m = self.arena.get(i).unwrap();

                if !m.is_doable(step_scope.score_director()) {
                    continue;
                }

                evaluated_count += 1;

                // Use RecordingScoreDirector for automatic undo
                let move_score = {
                    let mut recording =
                        RecordingScoreDirector::new(step_scope.score_director_mut());

                    // Execute move
                    m.do_move(&mut recording);

                    // Calculate resulting score
                    let score = recording.calculate_score();

                    // Undo the move
                    recording.undo_changes();

                    score
                };

                // Check if accepted
                let accepted = self.acceptor.is_accepted(&last_step_score, &move_score);

                // Add index to forager if accepted (not the move itself)
                if accepted {
                    accepted_count += 1;
                    self.forager.add_move_index(i, move_score);
                }

                // Check if forager wants to quit early
                if self.forager.is_quit_early() {
                    break;
                }
            }

            // Pick the best accepted move index
            if let Some((selected_index, selected_score)) = self.forager.pick_move_index() {
                // Take ownership of the move from arena
                let selected_move = self.arena.take(selected_index);

                // Execute the selected move (for real this time)
                selected_move.do_move(step_scope.score_director_mut());
                step_scope.set_step_score(selected_score);

                let step_duration = step_start.elapsed();

                debug!(
                    event = "local_search_step_complete",
                    step = step_number,
                    moves_evaluated = evaluated_count,
                    moves_accepted = accepted_count,
                    old_score = %last_step_score,
                    new_score = %selected_score,
                    step_duration_ms = step_duration.as_millis() as u64,
                );

                // Update last step score
                last_step_score = selected_score;

                // Update best solution if improved
                step_scope.phase_scope_mut().update_best_solution();
            } else {
                // No accepted moves - we're stuck
                debug!(
                    event = "local_search_no_accepted_moves",
                    step = step_number,
                    moves_evaluated = evaluated_count,
                );
                break;
            }

            step_scope.complete();
        }

        // Notify acceptor of phase end
        self.acceptor.phase_ended();

        let phase_duration = phase_start.elapsed();
        info!(
            event = "local_search_phase_end",
            final_score = %last_step_score,
            total_steps = phase_scope.step_count(),
            phase_duration_ms = phase_duration.as_millis() as u64,
        );
    }

    fn phase_type_name(&self) -> &'static str {
        "LocalSearch"
    }
}
