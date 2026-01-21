//! List variable solver for VRP-style problems.
//!
//! This module provides `run_list_solver` for problems using `#[shadow_variable_updates]`
//! with list variables, where each entity has a list of elements that can be rearranged.
//!
//! Logging levels:
//! - **INFO**: Solver start/end, phase summaries, problem scale
//! - **DEBUG**: Individual steps with timing and scores
//! - **TRACE**: Move evaluation details

use std::fmt::{self, Debug};
use std::hash::Hash;
use std::marker::PhantomData;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use rand::Rng;
use solverforge_config::{PhaseConfig, SolverConfig};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::{
    ConstraintSet, ScoreDirector, ShadowAwareScoreDirector, ShadowVariableSupport,
    TypedScoreDirector,
};
use tokio::sync::mpsc;
use tracing::{debug, info, trace};

use crate::phase::localsearch::{Acceptor, AcceptorImpl};
use crate::phase::Phase;
use crate::scope::{PhaseScope, SolverScope};
use crate::solver::Solver;
use crate::termination::{
    OrTermination, StepCountTermination, TimeTermination, UnimprovedStepCountTermination,
    UnimprovedTimeTermination,
};

/// Default time limit in seconds.
const DEFAULT_TIME_LIMIT_SECS: u64 = 30;

/// Solves a list variable problem using construction heuristic + late acceptance local search.
///
/// This function is called by macro-generated `solve()` methods for solutions
/// using `#[shadow_variable_updates]` with list variables.
///
/// # Type Parameters
///
/// * `S` - The solution type (must implement `PlanningSolution`)
/// * `C` - The constraint set type
/// * `E` - The element type (e.g., visit index)
#[allow(clippy::too_many_arguments)]
pub fn run_list_solver<S, C, E>(
    solution: S,
    finalize_fn: fn(&mut S),
    constraints_fn: fn() -> C,
    element_count: fn(&S) -> usize,
    get_assigned: fn(&S) -> Vec<E>,
    entity_count: fn(&S) -> usize,
    list_len: fn(&S, usize) -> usize,
    list_insert: fn(&mut S, usize, usize, E),
    list_remove: fn(&mut S, usize, usize) -> E,
    index_to_element: fn(usize) -> E,
    descriptor_index: usize,
    variable_name: &'static str,
) -> S
where
    S: PlanningSolution + ShadowVariableSupport,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
    E: Copy + Eq + Hash + Send + Sync + 'static,
{
    let (sender, _receiver) = mpsc::unbounded_channel();
    run_list_solver_with_channel(
        solution,
        finalize_fn,
        constraints_fn,
        element_count,
        get_assigned,
        entity_count,
        list_len,
        list_insert,
        list_remove,
        index_to_element,
        descriptor_index,
        variable_name,
        None,
        sender,
    )
}

/// Solves a list variable problem with channel-based solution streaming.
///
/// Logs solver progress via `tracing`. Optionally accepts a termination flag.
/// Solutions are sent through the channel as they improve.
#[allow(clippy::too_many_arguments)]
pub fn run_list_solver_with_channel<S, C, E>(
    mut solution: S,
    finalize_fn: fn(&mut S),
    constraints_fn: fn() -> C,
    element_count: fn(&S) -> usize,
    get_assigned: fn(&S) -> Vec<E>,
    entity_count: fn(&S) -> usize,
    list_len: fn(&S, usize) -> usize,
    list_insert: fn(&mut S, usize, usize, E),
    list_remove: fn(&mut S, usize, usize) -> E,
    index_to_element: fn(usize) -> E,
    descriptor_index: usize,
    variable_name: &'static str,
    terminate: Option<&AtomicBool>,
    sender: mpsc::UnboundedSender<(S, S::Score)>,
) -> S
where
    S: PlanningSolution + ShadowVariableSupport,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
    E: Copy + Eq + Hash + Send + Sync + 'static,
{
    finalize_fn(&mut solution);

    let config = SolverConfig::load("solver.toml").unwrap_or_default();
    let n_entities = entity_count(&solution);
    let n_elements = element_count(&solution);

    info!(
        event = "solve_start",
        entity_count = n_entities,
        element_count = n_elements,
    );

    let constraints = constraints_fn();
    let typed_director = TypedScoreDirector::new(solution, constraints);
    let director = ShadowAwareScoreDirector::new(typed_director);

    if n_entities == 0 || n_elements == 0 {
        let mut solver_scope = SolverScope::new(director);
        solver_scope.start_solving();
        let score = solver_scope.calculate_score();
        info!(event = "solve_end", score = %score);
        return solver_scope.take_best_or_working_solution();
    }

    let construction = ListConstructionPhase::new(
        element_count,
        get_assigned,
        entity_count,
        list_insert,
        index_to_element,
        descriptor_index,
        variable_name,
    );

    let acceptor = extract_acceptor_from_config::<S>(&config);

    let local_search = ListLocalSearchPhase::new(
        entity_count,
        list_len,
        list_insert,
        list_remove,
        descriptor_index,
        variable_name,
        acceptor,
        sender,
    );

    let result = solve_with_termination(
        director,
        construction,
        local_search,
        terminate,
        config.termination.as_ref(),
    );

    let score = result.score().unwrap_or_default();
    info!(event = "solve_end", score = %score);
    result
}

fn extract_acceptor_from_config<S: PlanningSolution>(config: &SolverConfig) -> AcceptorImpl<S> {
    for phase in &config.phases {
        if let PhaseConfig::LocalSearch(ls) = phase {
            if let Some(acceptor_config) = &ls.acceptor {
                return AcceptorImpl::from_config(acceptor_config);
            }
        }
    }
    AcceptorImpl::late_acceptance()
}

fn solve_with_termination<S, D, E, A>(
    director: D,
    construction: ListConstructionPhase<S, E>,
    local_search: ListLocalSearchPhase<S, E, A>,
    terminate: Option<&AtomicBool>,
    term_config: Option<&solverforge_config::TerminationConfig>,
) -> S
where
    S: PlanningSolution,
    S::Score: Score,
    D: ScoreDirector<S>,
    E: Copy + Eq + Hash + Send + Sync + 'static,
    A: Acceptor<S> + Send,
{
    let time_limit = term_config
        .and_then(|c| c.time_limit())
        .unwrap_or(Duration::from_secs(DEFAULT_TIME_LIMIT_SECS));
    let time = TimeTermination::new(time_limit);

    if let Some(step_limit) = term_config.and_then(|c| c.step_count_limit) {
        let step = StepCountTermination::new(step_limit);
        let termination: OrTermination<_, S, D> = OrTermination::new((time, step));
        build_and_solve(
            construction,
            local_search,
            termination,
            terminate,
            director,
            time_limit,
        )
    } else if let Some(unimproved_step_limit) =
        term_config.and_then(|c| c.unimproved_step_count_limit)
    {
        let unimproved = UnimprovedStepCountTermination::<S>::new(unimproved_step_limit);
        let termination: OrTermination<_, S, D> = OrTermination::new((time, unimproved));
        build_and_solve(
            construction,
            local_search,
            termination,
            terminate,
            director,
            time_limit,
        )
    } else if let Some(unimproved_time) = term_config.and_then(|c| c.unimproved_time_limit()) {
        let unimproved = UnimprovedTimeTermination::<S>::new(unimproved_time);
        let termination: OrTermination<_, S, D> = OrTermination::new((time, unimproved));
        build_and_solve(
            construction,
            local_search,
            termination,
            terminate,
            director,
            time_limit,
        )
    } else {
        let termination: OrTermination<_, S, D> = OrTermination::new((time,));
        build_and_solve(
            construction,
            local_search,
            termination,
            terminate,
            director,
            time_limit,
        )
    }
}

fn build_and_solve<S, D, E, A, Term>(
    construction: ListConstructionPhase<S, E>,
    local_search: ListLocalSearchPhase<S, E, A>,
    termination: Term,
    terminate: Option<&AtomicBool>,
    director: D,
    time_limit: Duration,
) -> S
where
    S: PlanningSolution,
    S::Score: Score,
    D: ScoreDirector<S>,
    E: Copy + Eq + Hash + Send + Sync + 'static,
    A: Acceptor<S> + Send,
    Term: crate::termination::Termination<S, D>,
{
    match terminate {
        Some(flag) => Solver::new((construction, local_search))
            .with_termination(termination)
            .with_time_limit(time_limit)
            .with_terminate(flag)
            .solve(director),
        None => Solver::new((construction, local_search))
            .with_termination(termination)
            .with_time_limit(time_limit)
            .solve(director),
    }
}

// ============================================================================
// List Construction Phase
// ============================================================================

/// Construction phase for list variable problems.
///
/// Assigns unassigned elements to entities round-robin.
pub struct ListConstructionPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + Eq + Hash + Send + Sync + 'static,
{
    element_count: fn(&S) -> usize,
    get_assigned: fn(&S) -> Vec<E>,
    entity_count: fn(&S) -> usize,
    list_insert: fn(&mut S, usize, usize, E),
    index_to_element: fn(usize) -> E,
    descriptor_index: usize,
    variable_name: &'static str,
    _phantom: PhantomData<fn() -> (S, E)>,
}

impl<S, E> Debug for ListConstructionPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + Eq + Hash + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ListConstructionPhase").finish()
    }
}

impl<S, E> ListConstructionPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + Eq + Hash + Send + Sync + 'static,
{
    /// Creates a new list construction phase.
    pub fn new(
        element_count: fn(&S) -> usize,
        get_assigned: fn(&S) -> Vec<E>,
        entity_count: fn(&S) -> usize,
        list_insert: fn(&mut S, usize, usize, E),
        index_to_element: fn(usize) -> E,
        descriptor_index: usize,
        variable_name: &'static str,
    ) -> Self {
        Self {
            element_count,
            get_assigned,
            entity_count,
            list_insert,
            index_to_element,
            descriptor_index,
            variable_name,
            _phantom: PhantomData,
        }
    }
}

impl<S, D, E> Phase<S, D> for ListConstructionPhase<S, E>
where
    S: PlanningSolution,
    S::Score: Score,
    D: ScoreDirector<S>,
    E: Copy + Eq + Hash + Send + Sync + 'static,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D>) {
        let mut phase_scope = PhaseScope::new(solver_scope, 0);

        let n_entities = (self.entity_count)(phase_scope.solver_scope().working_solution());
        let n_elements = (self.element_count)(phase_scope.solver_scope().working_solution());

        info!(
            event = "phase_start",
            phase = "List Construction",
            phase_index = 0,
        );

        if n_entities == 0 || n_elements == 0 {
            phase_scope.update_best_solution();
            info!(
                event = "phase_end",
                phase = "List Construction",
                phase_index = 0,
                duration_ms = phase_scope.elapsed().as_millis() as u64,
                steps = 0u64,
                speed = 0u64,
                score = "N/A",
            );
            return;
        }

        let assigned: std::collections::HashSet<E> =
            (self.get_assigned)(phase_scope.solver_scope().working_solution())
                .into_iter()
                .collect();

        if assigned.len() >= n_elements {
            tracing::info!("All elements already assigned, skipping construction");
            let _score = phase_scope.calculate_score();
            phase_scope.update_best_solution();
            return;
        }

        let mut entity_idx = 0;
        for elem_idx in 0..n_elements {
            if phase_scope.solver_scope().should_terminate() {
                break;
            }

            let element = (self.index_to_element)(elem_idx);
            if assigned.contains(&element) {
                continue;
            }

            {
                let sd = phase_scope.score_director_mut();
                sd.before_variable_changed(self.descriptor_index, entity_idx, self.variable_name);
                (self.list_insert)(sd.working_solution_mut(), entity_idx, 0, element);
                sd.after_variable_changed(self.descriptor_index, entity_idx, self.variable_name);
            }

            phase_scope.increment_step_count();
            entity_idx = (entity_idx + 1) % n_entities;
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
            phase = "List Construction",
            phase_index = 0,
            duration_ms = duration.as_millis() as u64,
            steps = steps,
            speed = speed,
            score = best_score,
        );
    }

    fn phase_type_name(&self) -> &'static str {
        "ListConstruction"
    }
}

// ============================================================================
// List Local Search Phase
// ============================================================================

/// Local search phase for list variable problems.
///
/// Uses random relocate moves (remove from one position, insert at another).
pub struct ListLocalSearchPhase<S, E, A>
where
    S: PlanningSolution,
    E: Copy + Eq + Hash + Send + Sync + 'static,
    A: Acceptor<S> + Send,
{
    entity_count: fn(&S) -> usize,
    list_len: fn(&S, usize) -> usize,
    list_insert: fn(&mut S, usize, usize, E),
    list_remove: fn(&mut S, usize, usize) -> E,
    descriptor_index: usize,
    variable_name: &'static str,
    acceptor: A,
    sender: mpsc::UnboundedSender<(S, S::Score)>,
    _phantom: PhantomData<fn() -> (S, E)>,
}

impl<S, E, A> Debug for ListLocalSearchPhase<S, E, A>
where
    S: PlanningSolution,
    E: Copy + Eq + Hash + Send + Sync + 'static,
    A: Acceptor<S> + Send + Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ListLocalSearchPhase")
            .field("acceptor", &self.acceptor)
            .finish()
    }
}

impl<S, E, A> ListLocalSearchPhase<S, E, A>
where
    S: PlanningSolution,
    E: Copy + Eq + Hash + Send + Sync + 'static,
    A: Acceptor<S> + Send,
{
    /// Creates a new list local search phase.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_count: fn(&S) -> usize,
        list_len: fn(&S, usize) -> usize,
        list_insert: fn(&mut S, usize, usize, E),
        list_remove: fn(&mut S, usize, usize) -> E,
        descriptor_index: usize,
        variable_name: &'static str,
        acceptor: A,
        sender: mpsc::UnboundedSender<(S, S::Score)>,
    ) -> Self {
        Self {
            entity_count,
            list_len,
            list_insert,
            list_remove,
            descriptor_index,
            variable_name,
            acceptor,
            sender,
            _phantom: PhantomData,
        }
    }
}

impl<S, D, E, A> Phase<S, D> for ListLocalSearchPhase<S, E, A>
where
    S: PlanningSolution,
    S::Score: Score,
    D: ScoreDirector<S>,
    E: Copy + Eq + Hash + Send + Sync + 'static,
    A: Acceptor<S> + Send,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D>) {
        let mut phase_scope = PhaseScope::new(solver_scope, 1);
        let mut rng = rand::rng();

        let n_entities = (self.entity_count)(phase_scope.solver_scope().working_solution());

        info!(
            event = "phase_start",
            phase = "List Local Search",
            phase_index = 1,
        );

        if n_entities == 0 {
            info!(
                event = "phase_end",
                phase = "List Local Search",
                phase_index = 1,
                duration_ms = phase_scope.elapsed().as_millis() as u64,
                steps = 0u64,
                speed = 0u64,
                score = "N/A",
            );
            return;
        }

        let initial_score = phase_scope.calculate_score();
        let mut current_score = initial_score;
        let mut best_score = initial_score;

        self.acceptor.phase_started(&initial_score);

        let mut moves_evaluated: u64 = 0;
        let mut last_progress_time = std::time::Instant::now();
        let mut last_progress_moves: u64 = 0;

        {
            let solution = phase_scope.solver_scope().working_solution().clone();
            let _ = self.sender.send((solution, best_score));
        }

        loop {
            if phase_scope.solver_scope().should_terminate() || self.sender.is_closed() {
                break;
            }

            // Count total elements across all entities
            let mut total_elements = 0usize;
            for e in 0..n_entities {
                total_elements += (self.list_len)(phase_scope.solver_scope().working_solution(), e);
            }

            if total_elements == 0 {
                break;
            }

            // Pick a random element to relocate
            let from_entity = rng.random_range(0..n_entities);
            let from_len =
                (self.list_len)(phase_scope.solver_scope().working_solution(), from_entity);
            if from_len == 0 {
                continue;
            }
            let from_pos = rng.random_range(0..from_len);

            // Pick a random destination
            let to_entity = rng.random_range(0..n_entities);
            let to_len = (self.list_len)(phase_scope.solver_scope().working_solution(), to_entity);
            let to_pos = rng.random_range(0..=to_len);

            // Skip no-op moves (same position or adjacent positions in same entity)
            if from_entity == to_entity && (to_pos == from_pos || to_pos == from_pos + 1) {
                continue;
            }

            moves_evaluated += 1;

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

            // Adjust to_pos if removing from same entity before to_pos
            let adjusted_to_pos = if from_entity == to_entity && from_pos < to_pos {
                to_pos - 1
            } else {
                to_pos
            };

            // Apply move with proper atomic before/after ordering:
            // 1. Retract ALL affected entities BEFORE any modifications
            // 2. Apply modifications
            // 3. Insert ALL affected entities AFTER all modifications
            {
                let sd = phase_scope.score_director_mut();

                // Step 1: Retract BOTH entities before any changes
                sd.before_variable_changed(self.descriptor_index, from_entity, self.variable_name);
                if from_entity != to_entity {
                    sd.before_variable_changed(
                        self.descriptor_index,
                        to_entity,
                        self.variable_name,
                    );
                }

                // Step 2: Apply the move
                let removed = (self.list_remove)(sd.working_solution_mut(), from_entity, from_pos);
                (self.list_insert)(
                    sd.working_solution_mut(),
                    to_entity,
                    adjusted_to_pos,
                    removed,
                );

                // Step 3: Insert BOTH entities after all changes
                sd.after_variable_changed(self.descriptor_index, from_entity, self.variable_name);
                if from_entity != to_entity {
                    sd.after_variable_changed(self.descriptor_index, to_entity, self.variable_name);
                }
            }

            let new_score = phase_scope.calculate_score();

            self.acceptor.step_started();
            let accepted = self.acceptor.is_accepted(&current_score, &new_score);

            if accepted {
                // Move already applied, just update state
                self.acceptor.step_ended(&new_score);
                current_score = new_score;
                let new_step = phase_scope.increment_step_count();

                trace!(
                    event = "step",
                    step = new_step,
                    from_entity = from_entity,
                    from_pos = from_pos,
                    to_entity = to_entity,
                    to_pos = to_pos,
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
                    from_entity = from_entity,
                    to_entity = to_entity,
                    score = %new_score,
                    accepted = false,
                );

                // Undo the move with proper atomic before/after ordering
                let sd = phase_scope.score_director_mut();

                // Step 1: Retract BOTH entities before any changes
                sd.before_variable_changed(self.descriptor_index, to_entity, self.variable_name);
                if from_entity != to_entity {
                    sd.before_variable_changed(
                        self.descriptor_index,
                        from_entity,
                        self.variable_name,
                    );
                }

                // Step 2: Undo the move
                let removed =
                    (self.list_remove)(sd.working_solution_mut(), to_entity, adjusted_to_pos);
                (self.list_insert)(sd.working_solution_mut(), from_entity, from_pos, removed);

                // Step 3: Insert BOTH entities after all changes
                sd.after_variable_changed(self.descriptor_index, to_entity, self.variable_name);
                if from_entity != to_entity {
                    sd.after_variable_changed(
                        self.descriptor_index,
                        from_entity,
                        self.variable_name,
                    );
                }
            }
        }

        self.acceptor.phase_ended();

        let duration = phase_scope.elapsed();
        let speed = if duration.as_secs_f64() > 0.0 {
            (moves_evaluated as f64 / duration.as_secs_f64()) as u64
        } else {
            0
        };

        let best_score_str = format!("{best_score}");
        info!(
            event = "phase_end",
            phase = "List Local Search",
            phase_index = 1,
            duration_ms = duration.as_millis() as u64,
            steps = phase_scope.step_count(),
            speed = speed,
            score = best_score_str,
        );
    }

    fn phase_type_name(&self) -> &'static str {
        "ListLocalSearch"
    }
}
