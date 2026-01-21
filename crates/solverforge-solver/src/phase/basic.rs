//! Basic variable phases for simple assignment problems.
//!
//! Provides construction and local search phases for solutions using
//! `#[basic_variable_config]`, where each entity has a single planning
//! variable assignable from a fixed value range.

use std::fmt::{self, Debug};
use std::marker::PhantomData;

use rand::Rng;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::ScoreDirector;
use tokio::sync::mpsc;
use tracing::{debug, info, trace};

use super::Phase;
use crate::scope::{PhaseScope, SolverScope};

/// Late acceptance history size.
const LATE_ACCEPTANCE_SIZE: usize = 400;

/// Construction phase for basic variable problems.
///
/// Randomly assigns values to uninitialized entities.
///
/// # Type Parameters
/// * `S` - Solution type
/// * `G` - Get variable function type
/// * `T` - Set variable function type
/// * `E` - Entity count function type
/// * `V` - Value count function type
pub struct BasicConstructionPhase<S, G, T, E, V>
where
    S: PlanningSolution,
    G: Fn(&S, usize) -> Option<usize> + Send,
    T: Fn(&mut S, usize, Option<usize>) + Send,
    E: Fn(&S) -> usize + Send,
    V: Fn(&S) -> usize + Send,
{
    get_variable: G,
    set_variable: T,
    entity_count: E,
    value_count: V,
    _phantom: PhantomData<fn() -> S>,
}

impl<S, G, T, E, V> Debug for BasicConstructionPhase<S, G, T, E, V>
where
    S: PlanningSolution,
    G: Fn(&S, usize) -> Option<usize> + Send,
    T: Fn(&mut S, usize, Option<usize>) + Send,
    E: Fn(&S) -> usize + Send,
    V: Fn(&S) -> usize + Send,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BasicConstructionPhase").finish()
    }
}

impl<S, G, T, E, V> BasicConstructionPhase<S, G, T, E, V>
where
    S: PlanningSolution,
    G: Fn(&S, usize) -> Option<usize> + Send,
    T: Fn(&mut S, usize, Option<usize>) + Send,
    E: Fn(&S) -> usize + Send,
    V: Fn(&S) -> usize + Send,
{
    /// Creates a new construction phase.
    pub fn new(get_variable: G, set_variable: T, entity_count: E, value_count: V) -> Self {
        Self {
            get_variable,
            set_variable,
            entity_count,
            value_count,
            _phantom: PhantomData,
        }
    }
}

impl<S, D, G, T, E, V> Phase<S, D> for BasicConstructionPhase<S, G, T, E, V>
where
    S: PlanningSolution,
    S::Score: Score,
    D: ScoreDirector<S>,
    G: Fn(&S, usize) -> Option<usize> + Send,
    T: Fn(&mut S, usize, Option<usize>) + Send,
    E: Fn(&S) -> usize + Send,
    V: Fn(&S) -> usize + Send,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D>) {
        let mut phase_scope = PhaseScope::new(solver_scope, 0);
        let mut rng = rand::rng();

        let n_entities = (self.entity_count)(phase_scope.solver_scope().working_solution());
        let n_values = (self.value_count)(phase_scope.solver_scope().working_solution());

        info!(
            event = "phase_start",
            phase = "Construction Heuristic",
            phase_index = 0,
        );

        if n_entities == 0 || n_values == 0 {
            phase_scope.update_best_solution();
            info!(
                event = "phase_end",
                phase = "Construction Heuristic",
                phase_index = 0,
                duration_ms = phase_scope.elapsed().as_millis() as u64,
                steps = 0u64,
                speed = 0u64,
                score = "N/A",
            );
            return;
        }

        for entity_idx in 0..n_entities {
            if phase_scope.solver_scope().should_terminate() {
                break;
            }

            if (self.get_variable)(phase_scope.solver_scope().working_solution(), entity_idx)
                .is_none()
            {
                let value = rng.random_range(0..n_values);
                (self.set_variable)(
                    phase_scope.solver_scope_mut().working_solution_mut(),
                    entity_idx,
                    Some(value),
                );
            }
            phase_scope.increment_step_count();
        }

        phase_scope.update_best_solution();

        let best_score = phase_scope
            .solver_scope()
            .best_score()
            .map(|s| format!("{s}"))
            .unwrap_or_else(|| "none".to_string());

        let duration = phase_scope.elapsed();
        let steps = phase_scope.step_count();
        let speed = if duration.as_secs_f64() > 0.0 {
            (steps as f64 / duration.as_secs_f64()) as u64
        } else {
            0
        };

        info!(
            event = "phase_end",
            phase = "Construction Heuristic",
            phase_index = 0,
            duration_ms = duration.as_millis() as u64,
            steps = steps,
            speed = speed,
            score = best_score,
        );
    }

    fn phase_type_name(&self) -> &'static str {
        "BasicConstruction"
    }
}

/// Late acceptance local search phase for basic variable problems.
///
/// # Type Parameters
/// * `S` - Solution type
/// * `G` - Get variable function type
/// * `T` - Set variable function type
/// * `E` - Entity count function type
/// * `V` - Value count function type
pub struct BasicLocalSearchPhase<S, G, T, E, V>
where
    S: PlanningSolution,
    G: Fn(&S, usize) -> Option<usize> + Send,
    T: Fn(&mut S, usize, Option<usize>) + Send,
    E: Fn(&S) -> usize + Send,
    V: Fn(&S) -> usize + Send,
{
    get_variable: G,
    set_variable: T,
    entity_count: E,
    value_count: V,
    sender: mpsc::UnboundedSender<(S, S::Score)>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S, G, T, E, V> Debug for BasicLocalSearchPhase<S, G, T, E, V>
where
    S: PlanningSolution,
    G: Fn(&S, usize) -> Option<usize> + Send,
    T: Fn(&mut S, usize, Option<usize>) + Send,
    E: Fn(&S) -> usize + Send,
    V: Fn(&S) -> usize + Send,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BasicLocalSearchPhase").finish()
    }
}

impl<S, G, T, E, V> BasicLocalSearchPhase<S, G, T, E, V>
where
    S: PlanningSolution,
    G: Fn(&S, usize) -> Option<usize> + Send,
    T: Fn(&mut S, usize, Option<usize>) + Send,
    E: Fn(&S) -> usize + Send,
    V: Fn(&S) -> usize + Send,
{
    /// Creates a new local search phase with a channel sender.
    pub fn new(
        get_variable: G,
        set_variable: T,
        entity_count: E,
        value_count: V,
        sender: mpsc::UnboundedSender<(S, S::Score)>,
    ) -> Self {
        Self {
            get_variable,
            set_variable,
            entity_count,
            value_count,
            sender,
            _phantom: PhantomData,
        }
    }
}

impl<S, D, G, T, E, V> Phase<S, D> for BasicLocalSearchPhase<S, G, T, E, V>
where
    S: PlanningSolution,
    S::Score: Score,
    D: ScoreDirector<S>,
    G: Fn(&S, usize) -> Option<usize> + Send,
    T: Fn(&mut S, usize, Option<usize>) + Send,
    E: Fn(&S) -> usize + Send,
    V: Fn(&S) -> usize + Send,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D>) {
        let mut phase_scope = PhaseScope::new(solver_scope, 1);
        let mut rng = rand::rng();

        let n_entities = (self.entity_count)(phase_scope.solver_scope().working_solution());
        let n_values = (self.value_count)(phase_scope.solver_scope().working_solution());

        info!(
            event = "phase_start",
            phase = "Late Acceptance",
            phase_index = 1,
        );

        if n_entities == 0 || n_values == 0 {
            info!(
                event = "phase_end",
                phase = "Late Acceptance",
                phase_index = 1,
                duration_ms = phase_scope.elapsed().as_millis() as u64,
                steps = 0u64,
                speed = 0u64,
                score = "N/A",
            );
            return;
        }

        // Initialize current score
        let initial_score = phase_scope.calculate_score();
        let mut current_score = initial_score;
        let mut best_score = initial_score;

        // Late acceptance history
        let mut late_scores = [initial_score; LATE_ACCEPTANCE_SIZE];
        let mut moves_evaluated: u64 = 0;
        let mut last_progress_time = std::time::Instant::now();
        let mut last_progress_moves: u64 = 0;

        // Send initial best through channel
        {
            let solution = phase_scope.solver_scope().working_solution().clone();
            let _ = self.sender.send((solution, best_score));
        }

        loop {
            if phase_scope.solver_scope().should_terminate() {
                break;
            }

            let entity_idx = rng.random_range(0..n_entities);
            let old_value =
                (self.get_variable)(phase_scope.solver_scope().working_solution(), entity_idx);
            let new_value = Some(rng.random_range(0..n_values));

            if old_value == new_value {
                continue;
            }

            moves_evaluated += 1;

            // Log progress every second
            let now = std::time::Instant::now();
            if now.duration_since(last_progress_time).as_secs() >= 1 {
                let moves_delta = moves_evaluated - last_progress_moves;
                let elapsed_secs = now.duration_since(last_progress_time).as_secs_f64();
                let current_speed = (moves_delta as f64 / elapsed_secs) as u64;
                debug!(
                    event = "progress",
                    steps = phase_scope.step_count(),
                    speed = current_speed,
                    score = %best_score,
                );
                last_progress_time = now;
                last_progress_moves = moves_evaluated;
            }

            // Apply move
            let director = phase_scope.score_director_mut();
            director.before_entity_changed(entity_idx);
            (self.set_variable)(director.working_solution_mut(), entity_idx, new_value);
            director.after_entity_changed(entity_idx);
            let new_score = director.calculate_score();

            let step = phase_scope.step_count();
            let late_idx = (step as usize) % LATE_ACCEPTANCE_SIZE;
            let late_score = late_scores[late_idx];

            if new_score >= current_score || new_score >= late_score {
                late_scores[late_idx] = new_score;
                current_score = new_score;
                let new_step = phase_scope.increment_step_count();

                trace!(
                    event = "step",
                    step = new_step,
                    entity = entity_idx,
                    score = %new_score,
                    accepted = true,
                );

                if new_score > best_score {
                    best_score = new_score;
                    phase_scope.update_best_solution();

                    let solution = phase_scope.solver_scope().working_solution().clone();
                    let _ = self.sender.send((solution, best_score));
                }
            } else {
                trace!(
                    event = "step",
                    step = moves_evaluated,
                    entity = entity_idx,
                    score = %new_score,
                    accepted = false,
                );
                // Undo move
                let director = phase_scope.score_director_mut();
                director.before_entity_changed(entity_idx);
                (self.set_variable)(director.working_solution_mut(), entity_idx, old_value);
                director.after_entity_changed(entity_idx);
                director.calculate_score();
            }
        }

        let duration = phase_scope.elapsed();
        let speed = if duration.as_secs_f64() > 0.0 {
            (moves_evaluated as f64 / duration.as_secs_f64()) as u64
        } else {
            0
        };

        let best_score_str = format!("{best_score}");
        info!(
            event = "phase_end",
            phase = "Late Acceptance",
            phase_index = 1,
            duration_ms = duration.as_millis() as u64,
            steps = phase_scope.step_count(),
            speed = speed,
            score = best_score_str,
        );
    }

    fn phase_type_name(&self) -> &'static str {
        "BasicLocalSearch"
    }
}
