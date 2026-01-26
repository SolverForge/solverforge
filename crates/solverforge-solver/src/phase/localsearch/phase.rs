//! Local search phase implementation.

use std::fmt::Debug;
use std::marker::PhantomData;
use std::time::Instant;

use rand::prelude::IndexedRandom;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::api::constraint_set::ConstraintSet;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, info, warn};

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
///
/// # Stagnation Detection
///
/// Tracks consecutive steps without accepted moves. When the counter exceeds
/// `stagnation_threshold`, the phase force-accepts a random doable move to
/// escape local optima. This works with any acceptor (Late Acceptance, Hill
/// Climbing, Simulated Annealing, etc.).
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
    arena: MoveArena<M>,
    step_limit: Option<u64>,
    sender: Option<UnboundedSender<(S, S::Score)>>,
    /// Number of consecutive steps without accepted moves before forcing a random move.
    stagnation_threshold: u64,
    /// Counter for consecutive steps without accepted moves.
    steps_without_accepted_move: u64,
    /// Time limit in seconds for this phase (used for SimulatedAnnealing time gradient).
    time_limit_seconds: Option<f64>,
    _phantom: PhantomData<fn() -> (S, M)>,
}

impl<S, M, MS, A, Fo> LocalSearchPhase<S, M, MS, A, Fo>
where
    S: PlanningSolution,
    M: Move<S> + 'static,
    MS: MoveSelector<S, M>,
    A: Acceptor<S>,
    Fo: LocalSearchForager<S, M>,
{
    /// Creates a new local search phase.
    ///
    /// # Arguments
    /// * `move_selector` - Selector that generates candidate moves
    /// * `acceptor` - Acceptor that decides whether to accept moves
    /// * `forager` - Forager that picks the best accepted move
    /// * `step_limit` - Optional maximum number of steps
    /// * `stagnation_threshold` - Number of consecutive steps without accepted moves
    ///   before forcing a random move (0 disables stagnation detection)
    pub fn new(
        move_selector: MS,
        acceptor: A,
        forager: Fo,
        step_limit: Option<u64>,
        stagnation_threshold: u64,
    ) -> Self {
        Self {
            move_selector,
            acceptor,
            forager,
            arena: MoveArena::new(),
            step_limit,
            sender: None,
            stagnation_threshold,
            steps_without_accepted_move: 0,
            time_limit_seconds: None,
            _phantom: PhantomData,
        }
    }

    /// Sets the time limit for this phase (used for SimulatedAnnealing time gradient).
    pub fn with_time_limit_seconds(mut self, seconds: f64) -> Self {
        self.time_limit_seconds = Some(seconds);
        self
    }

    /// Sets the sender for streaming improved solutions.
    pub fn with_sender(mut self, sender: UnboundedSender<(S, S::Score)>) -> Self {
        self.sender = Some(sender);
        self
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
            .field("arena", &self.arena)
            .field("step_limit", &self.step_limit)
            .finish()
    }
}

impl<S, C, M, MS, A, Fo> Phase<S, C> for LocalSearchPhase<S, M, MS, A, Fo>
where
    S: PlanningSolution + solverforge_scoring::ShadowVariableSupport,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
    M: Move<S> + 'static,
    MS: MoveSelector<S, M>,
    A: Acceptor<S>,
    Fo: LocalSearchForager<S, M>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S, C>) {
        let mut phase_scope = PhaseScope::new(solver_scope, 0);

        // Phase start event
        info!(event = "phase_start", phase = "LocalSearch");

        // Calculate initial score
        let mut last_step_score = phase_scope.calculate_score();

        // Notify acceptor of phase start
        self.acceptor.phase_started(&last_step_score);

        // Track phase start time for time gradient calculation
        let phase_start_time = Instant::now();

        // Send initial solution
        if let Some(ref sender) = self.sender {
            let solution = phase_scope.solver_scope().working_solution().clone();
            let _ = sender.send((solution, last_step_score));
        }

        // Progress tracking
        let mut last_progress_time = Instant::now();
        let mut moves_at_last_progress = 0u64;
        let mut total_moves = 0u64;

        loop {
            // Check early termination
            if phase_scope.solver_scope().is_terminate_early() {
                break;
            }

            // Check if receiver dropped
            if self.sender.as_ref().is_some_and(|s| s.is_closed()) {
                break;
            }

            // Check step limit
            if let Some(limit) = self.step_limit {
                if phase_scope.step_count() >= limit {
                    break;
                }
            }

            let mut step_scope = StepScope::new(&mut phase_scope);

            // Update time gradient for time-based acceptors (SimulatedAnnealing)
            if let Some(time_limit) = self.time_limit_seconds {
                let elapsed = phase_start_time.elapsed().as_secs_f64();
                let time_gradient = (elapsed / time_limit).min(1.0);
                self.acceptor.set_time_gradient(time_gradient);
            }

            // Reset forager for this step
            self.forager.step_started();

            // Reset arena and populate with moves - O(1) reset
            self.arena.reset();
            self.arena
                .extend(self.move_selector.iter_moves(step_scope.score_director()));

            let moves_generated = self.arena.len();
            let mut moves_evaluated = 0u64;
            let mut moves_accepted = 0u64;
            let mut moves_not_doable = 0u64;
            // Only collect doable indices when stagnation escape is imminent.
            // After the loop, if no move is accepted, steps_without_accepted_move will be
            // incremented by 1, so we check +1 here to predict whether we'll need the cache.
            let collect_doable = self.stagnation_threshold > 0
                && self.steps_without_accepted_move + 1 >= self.stagnation_threshold;
            let mut doable_indices_with_scores: Option<Vec<(usize, S::Score)>> =
                if collect_doable { Some(Vec::new()) } else { None };

            // Progress event every 1 second
            let now = Instant::now();
            if now.duration_since(last_progress_time).as_secs() >= 1 {
                let delta = total_moves - moves_at_last_progress;
                let elapsed = now.duration_since(last_progress_time).as_secs_f64();
                let speed = (delta as f64 / elapsed) as u64;
                // Show best score (monotonically improving) rather than current step score
                // (which can oscillate with Late Acceptance, Simulated Annealing, etc.)
                let best_score = step_scope
                    .phase_scope()
                    .solver_scope()
                    .best_score()
                    .copied()
                    .unwrap_or(last_step_score);
                debug!(
                    event = "progress",
                    steps = step_scope.phase_scope().step_count(),
                    speed = speed,
                    score = %best_score,
                );
                last_progress_time = now;
                moves_at_last_progress = total_moves;
            }

            // Evaluate moves by index
            for i in 0..self.arena.len() {
                let m = self.arena.get(i).unwrap();

                if !m.is_doable(step_scope.score_director()) {
                    moves_not_doable += 1;
                    continue;
                }

                // Count moves actually evaluated (after doable check)
                total_moves += 1;
                moves_evaluated += 1;

                // Evaluate move: save score, execute, calculate, undo
                let move_score = {
                    let sd = step_scope.score_director_mut();
                    sd.save_score_snapshot(); // Save score before move
                    m.do_move(sd); // Execute move
                    let score = sd.calculate_score(); // Calculate resulting score
                    sd.undo_changes(); // Restore solution AND score
                    score
                };

                // Record move context for tabu acceptors before checking acceptance
                // Use arena index as move hash (unique within this step's arena)
                self.acceptor
                    .record_move_context(m.entity_indices(), i as u64);

                // Check if accepted
                let accepted = self.acceptor.is_accepted(&last_step_score, &move_score);

                debug!(
                    event = "move_evaluated",
                    move_idx = i,
                    move_score = %move_score,
                    last_step_score = %last_step_score,
                    accepted = accepted,
                );

                // Cache doable move with its score for potential stagnation escape
                if let Some(ref mut cache) = doable_indices_with_scores {
                    cache.push((i, move_score));
                }

                // Add index to forager if accepted (not the move itself)
                if accepted {
                    moves_accepted += 1;
                    self.forager.add_move_index(i, move_score);
                }

                // Check if forager wants to quit early
                if self.forager.is_quit_early() {
                    break;
                }
            }

            // Log step evaluation summary
            debug!(
                event = "step_evaluation",
                step = step_scope.phase_scope().step_count(),
                moves_generated = moves_generated,
                moves_not_doable = moves_not_doable,
                moves_evaluated = moves_evaluated,
                moves_accepted = moves_accepted,
                last_step_score = %last_step_score,
            );

            // Pick the best accepted move index, or force-accept a random move if stagnating
            let maybe_move = if let Some((idx, score)) = self.forager.pick_move_index() {
                // Normal case: forager picked a move
                self.steps_without_accepted_move = 0;
                Some((idx, score, false))
            } else {
                // No accepted move this step
                self.steps_without_accepted_move += 1;

                debug!(
                    event = "no_move_selected",
                    step = step_scope.phase_scope().step_count(),
                    moves_generated = moves_generated,
                    moves_accepted = moves_accepted,
                    steps_without_accepted_move = self.steps_without_accepted_move,
                    stagnation_threshold = self.stagnation_threshold,
                    last_step_score = %last_step_score,
                );

                // Check for stagnation and force-accept a random doable move
                if self.stagnation_threshold > 0
                    && self.steps_without_accepted_move >= self.stagnation_threshold
                {
                    // Reuse cached doable indices from main evaluation loop
                    let cache = doable_indices_with_scores.as_deref().unwrap_or(&[]);
                    if !cache.is_empty() {
                        // Pick a random doable move (score already evaluated)
                        let rng = step_scope.phase_scope_mut().solver_scope_mut().rng();
                        let &(random_idx, move_score) = cache.choose(rng).unwrap();

                        warn!(
                            event = "stagnation_escape",
                            step = step_scope.phase_scope().step_count(),
                            steps_without_accepted_move = self.steps_without_accepted_move,
                            force_accepted_index = random_idx,
                            force_accepted_score = %move_score,
                            last_step_score = %last_step_score,
                        );

                        // Reset stagnation counter
                        self.steps_without_accepted_move = 0;

                        Some((random_idx, move_score, true))
                    } else {
                        warn!(
                            event = "stagnation_no_doable_moves",
                            step = step_scope.phase_scope().step_count(),
                            steps_without_accepted_move = self.steps_without_accepted_move,
                        );
                        None
                    }
                } else {
                    None
                }
            };

            if let Some((selected_index, selected_score, _was_forced)) = maybe_move {
                // Take ownership of the move from arena
                let selected_move = self.arena.take(selected_index);

                debug!(
                    event = "move_selected",
                    step = step_scope.phase_scope().step_count(),
                    selected_index = selected_index,
                    selected_score = %selected_score,
                    last_step_score = %last_step_score,
                    move_type = ?selected_move,
                );

                // Execute the selected move (for real this time)
                selected_move.do_move(step_scope.score_director_mut());

                // Clear the undo stack - this move is now permanently applied
                step_scope.score_director_mut().clear_undo_stack();

                step_scope.set_step_score(selected_score);

                // Update last step score
                last_step_score = selected_score;

                // Track previous best score to detect improvement
                let prev_best = step_scope
                    .phase_scope()
                    .solver_scope()
                    .best_score()
                    .cloned();

                // Update best solution if improved
                step_scope.phase_scope_mut().update_best_solution();

                // Stream solution if best improved and sender is configured
                if let Some(ref sender) = self.sender {
                    let new_best = step_scope.phase_scope().solver_scope().best_score();
                    let improved = match (&prev_best, new_best) {
                        (None, Some(_)) => true,
                        (Some(prev), Some(curr)) => curr > prev,
                        _ => false,
                    };
                    if improved {
                        if let (Some(sol), Some(score)) = (
                            step_scope.phase_scope().solver_scope().best_solution(),
                            new_best,
                        ) {
                            let _ = sender.send((sol.clone(), *score));
                        }
                    }
                }
            }

            step_scope.complete();

            // Notify acceptor that step is complete - CRITICAL for stateful acceptors
            // (Late Acceptance, Simulated Annealing, Great Deluge, etc.)
            self.acceptor.step_ended(&last_step_score);
        }

        // Notify acceptor of phase end
        self.acceptor.phase_ended();

        // Phase end event
        let duration = phase_scope.elapsed();
        let steps = phase_scope.step_count();
        let speed = if duration.as_secs_f64() > 0.0 {
            (total_moves as f64 / duration.as_secs_f64()) as u64
        } else {
            0
        };
        info!(
            event = "phase_end",
            phase = "LocalSearch",
            duration_ms = duration.as_millis() as u64,
            steps = steps,
            speed = speed,
            score = %last_step_score,
        );
    }

    fn phase_type_name(&self) -> &'static str {
        "LocalSearch"
    }
}
