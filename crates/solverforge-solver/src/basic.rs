//! Basic variable solver for simple assignment problems.
//!
//! This module provides `run_solver` for problems using `#[basic_variable_config]`,
//! where each entity has a single planning variable that can be assigned from a
//! fixed value range.

use rand::Rng;
use solverforge_config::{SolverConfig, TerminationConfig};
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::Score;
use solverforge_scoring::{ConstraintSet, TypedScoreDirector};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Late acceptance history size.
const LATE_ACCEPTANCE_SIZE: usize = 400;

/// Default time limit in seconds.
const DEFAULT_TIME_LIMIT_SECS: u64 = 30;

/// Tracks termination conditions during solving.
///
/// Supports all termination types from `TerminationConfig`:
/// - Time limit (seconds/minutes)
/// - Step count limit
/// - Unimproved step count limit
/// - Unimproved time limit
/// - Best score limit (requires score parsing)
/// - External termination flag
struct TerminationChecker<'a, Sc> {
    start: Instant,
    time_limit: Option<Duration>,
    step_limit: Option<u64>,
    unimproved_step_limit: Option<u64>,
    unimproved_time_limit: Option<Duration>,
    best_score_limit: Option<Sc>,
    external_flag: Option<&'a AtomicBool>,
    last_improvement_step: u64,
    last_improvement_time: Instant,
}

impl<'a, Sc: Score> TerminationChecker<'a, Sc> {
    fn new(
        config: &TerminationConfig,
        external_flag: Option<&'a AtomicBool>,
        best_score_limit: Option<Sc>,
    ) -> Self {
        let now = Instant::now();
        Self {
            start: now,
            time_limit: config.time_limit().or(Some(Duration::from_secs(DEFAULT_TIME_LIMIT_SECS))),
            step_limit: config.step_count_limit,
            unimproved_step_limit: config.unimproved_step_count_limit,
            unimproved_time_limit: config.unimproved_time_limit(),
            best_score_limit,
            external_flag,
            last_improvement_step: 0,
            last_improvement_time: now,
        }
    }

    fn from_defaults(external_flag: Option<&'a AtomicBool>) -> Self {
        let now = Instant::now();
        Self {
            start: now,
            time_limit: Some(Duration::from_secs(DEFAULT_TIME_LIMIT_SECS)),
            step_limit: None,
            unimproved_step_limit: None,
            unimproved_time_limit: None,
            best_score_limit: None,
            external_flag,
            last_improvement_step: 0,
            last_improvement_time: now,
        }
    }

    /// Records that the best score improved at this step.
    fn record_improvement(&mut self, step: u64) {
        self.last_improvement_step = step;
        self.last_improvement_time = Instant::now();
    }

    /// Checks if solving should terminate.
    fn should_terminate(&self, step: u64, best_score: Sc) -> bool {
        // External termination flag
        if self.external_flag.map_or(false, |f| f.load(Ordering::Relaxed)) {
            return true;
        }

        // Time limit
        if let Some(limit) = self.time_limit {
            if self.start.elapsed() >= limit {
                return true;
            }
        }

        // Step count limit
        if let Some(limit) = self.step_limit {
            if step >= limit {
                return true;
            }
        }

        // Unimproved step count limit
        if let Some(limit) = self.unimproved_step_limit {
            if step - self.last_improvement_step >= limit {
                return true;
            }
        }

        // Unimproved time limit
        if let Some(limit) = self.unimproved_time_limit {
            if self.last_improvement_time.elapsed() >= limit {
                return true;
            }
        }

        // Best score limit (terminate when score >= limit)
        if let Some(ref limit) = self.best_score_limit {
            if best_score >= *limit {
                return true;
            }
        }

        false
    }
}

/// Events emitted during solving for console/UI feedback.
#[derive(Debug, Clone)]
pub enum SolverEvent<Sc> {
    /// Solving has started.
    Started {
        entity_count: usize,
        variable_count: usize,
        value_count: usize,
    },
    /// A phase has started.
    PhaseStarted {
        phase_index: usize,
        phase_name: &'static str,
    },
    /// A phase has ended.
    PhaseEnded {
        phase_index: usize,
        phase_name: &'static str,
        duration: Duration,
        steps: u64,
        moves_evaluated: u64,
        best_score: Sc,
    },
    /// A new best solution was found.
    BestSolutionChanged {
        step: u64,
        elapsed: Duration,
        moves_evaluated: u64,
        score: Sc,
    },
    /// Solving has ended.
    Ended {
        duration: Duration,
        total_moves: u64,
        phase_count: usize,
        final_score: Sc,
    },
}

/// Solves a basic variable problem using construction heuristic + late acceptance local search.
///
/// This function is called by macro-generated `solve()` methods for solutions
/// using `#[basic_variable_config]`.
///
/// # Type Parameters
///
/// * `S` - The solution type (must implement `PlanningSolution`)
/// * `C` - The constraint set type
///
/// # Arguments
///
/// * `solution` - The initial solution to solve
/// * `finalize_fn` - Function to prepare derived fields before solving
/// * `constraints_fn` - Function that creates the constraint set
/// * `get_variable` - Gets the planning variable value for an entity
/// * `set_variable` - Sets the planning variable value for an entity
/// * `value_count` - Returns the number of valid values
/// * `entity_count_fn` - Returns the number of entities
/// * `_descriptor` - Solution descriptor (unused, for future extensions)
/// * `_entity_count` - Entity count function (unused, for future extensions)
/// * `_variable_field` - Variable field name (unused, for future extensions)
/// * `_descriptor_index` - Descriptor index (unused, for future extensions)
pub fn run_solver<S, C>(
    solution: S,
    finalize_fn: fn(&mut S),
    constraints_fn: fn() -> C,
    get_variable: fn(&S, usize) -> Option<usize>,
    set_variable: fn(&mut S, usize, Option<usize>),
    value_count: fn(&S) -> usize,
    entity_count_fn: fn(&S) -> usize,
    _descriptor: fn() -> SolutionDescriptor,
    _entity_count: fn(&S, usize) -> usize,
    _variable_field: &'static str,
    _descriptor_index: usize,
) -> S
where
    S: PlanningSolution + Clone,
    S::Score: Score + Default + Copy,
    C: ConstraintSet<S, S::Score>,
{
    run_solver_with_events(
        solution,
        finalize_fn,
        constraints_fn,
        get_variable,
        set_variable,
        value_count,
        entity_count_fn,
        None,
        |_| {},
        |_, _| {},
    )
}

/// Solves a basic variable problem with full event callbacks.
///
/// Provides events for phases, steps, and best solutions for console/UI feedback.
/// Optionally accepts a termination flag to stop solving early.
pub fn run_solver_with_events<S, C, E, F>(
    mut solution: S,
    finalize_fn: fn(&mut S),
    constraints_fn: fn() -> C,
    get_variable: fn(&S, usize) -> Option<usize>,
    set_variable: fn(&mut S, usize, Option<usize>),
    value_count: fn(&S) -> usize,
    entity_count_fn: fn(&S) -> usize,
    terminate: Option<&AtomicBool>,
    mut on_event: E,
    mut on_best_solution: F,
) -> S
where
    S: PlanningSolution + Clone,
    S::Score: Score + Default + Copy,
    C: ConstraintSet<S, S::Score>,
    E: FnMut(SolverEvent<S::Score>),
    F: FnMut(&S, S::Score),
{
    // Finalize derived fields
    finalize_fn(&mut solution);

    // Load config and create termination checker
    let config = SolverConfig::load("solver.toml").unwrap_or_default();
    let mut termination = match &config.termination {
        Some(term_config) => TerminationChecker::new(term_config, terminate, None),
        None => TerminationChecker::from_defaults(terminate),
    };

    // Create constraints and score director
    let constraints = constraints_fn();

    let n_entities = entity_count_fn(&solution);
    let n_values = value_count(&solution);

    // Emit started event
    on_event(SolverEvent::Started {
        entity_count: n_entities,
        variable_count: n_entities,
        value_count: n_values,
    });

    let solve_start = Instant::now();

    if n_entities == 0 || n_values == 0 {
        // Still calculate and set score for empty problems
        let mut director = TypedScoreDirector::new(solution, constraints);
        let score = director.calculate_score();
        on_event(SolverEvent::Ended {
            duration: solve_start.elapsed(),
            total_moves: 0,
            phase_count: 0,
            final_score: score,
        });
        return director.into_working_solution();
    }

    // Phase 1: Construction heuristic (greedy first-fit)
    on_event(SolverEvent::PhaseStarted {
        phase_index: 0,
        phase_name: "Construction Heuristic",
    });
    let phase1_start = Instant::now();

    let mut rng = rand::rng();
    for entity_idx in 0..n_entities {
        if get_variable(&solution, entity_idx).is_none() {
            let value = rng.random_range(0..n_values);
            set_variable(&mut solution, entity_idx, Some(value));
        }
    }

    // Create score director with working solution
    let mut director = TypedScoreDirector::new(solution, constraints);
    let mut current_score = director.calculate_score();
    let mut best_score = current_score;

    on_event(SolverEvent::PhaseEnded {
        phase_index: 0,
        phase_name: "Construction Heuristic",
        duration: phase1_start.elapsed(),
        steps: n_entities as u64,
        moves_evaluated: n_entities as u64,
        best_score,
    });

    // Notify initial best solution after construction
    {
        let mut best = director.working_solution().clone();
        best.set_score(Some(best_score));
        on_event(SolverEvent::BestSolutionChanged {
            step: 0,
            elapsed: solve_start.elapsed(),
            moves_evaluated: n_entities as u64,
            score: best_score,
        });
        on_best_solution(&best, best_score);
    }

    // Phase 2: Late Acceptance local search
    on_event(SolverEvent::PhaseStarted {
        phase_index: 1,
        phase_name: "Late Acceptance",
    });
    let phase2_start = Instant::now();

    let mut late_scores = vec![current_score; LATE_ACCEPTANCE_SIZE];
    let mut step: u64 = 0;
    let mut moves_evaluated: u64 = 0;
    let mut steps_accepted: u64 = 0;

    // Record initial improvement point
    termination.record_improvement(0);

    while !termination.should_terminate(step, best_score) {
        // Pick random entity and new value
        let entity_idx = rng.random_range(0..n_entities);
        let old_value = get_variable(director.working_solution(), entity_idx);
        let new_value = Some(rng.random_range(0..n_values));

        if old_value == new_value {
            continue;
        }

        moves_evaluated += 1;

        // Apply move using incremental protocol
        director.before_variable_changed(entity_idx);
        set_variable(director.working_solution_mut(), entity_idx, new_value);
        director.after_variable_changed(entity_idx);
        let new_score = director.calculate_score();

        // Late acceptance criterion
        let late_idx = (step as usize) % LATE_ACCEPTANCE_SIZE;
        let late_score = late_scores[late_idx];

        if new_score >= current_score || new_score >= late_score {
            // Accept
            late_scores[late_idx] = new_score;
            current_score = new_score;
            steps_accepted += 1;

            // Check if this is a new best
            if new_score > best_score {
                best_score = new_score;
                termination.record_improvement(step);
                let mut best = director.working_solution().clone();
                best.set_score(Some(best_score));
                on_event(SolverEvent::BestSolutionChanged {
                    step,
                    elapsed: solve_start.elapsed(),
                    moves_evaluated: n_entities as u64 + moves_evaluated,
                    score: best_score,
                });
                on_best_solution(&best, best_score);
            }
        } else {
            // Reject - undo using incremental protocol
            director.before_variable_changed(entity_idx);
            set_variable(director.working_solution_mut(), entity_idx, old_value);
            director.after_variable_changed(entity_idx);
            director.calculate_score(); // Update cached_score after undo
        }

        step += 1;
    }

    on_event(SolverEvent::PhaseEnded {
        phase_index: 1,
        phase_name: "Late Acceptance",
        duration: phase2_start.elapsed(),
        steps: steps_accepted,
        moves_evaluated,
        best_score,
    });

    let total_moves = n_entities as u64 + moves_evaluated;
    on_event(SolverEvent::Ended {
        duration: solve_start.elapsed(),
        total_moves,
        phase_count: 2,
        final_score: best_score,
    });

    director.into_working_solution()
}
