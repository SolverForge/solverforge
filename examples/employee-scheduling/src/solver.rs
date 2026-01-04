//! Solver service for Employee Scheduling.
//!
//! Uses Late Acceptance local search with change moves.
//! Incremental scoring via TypedScoreDirector for O(1) move evaluation.

use parking_lot::RwLock;
use rand::Rng;
use solverforge::prelude::*;
use solverforge::TypedScoreDirector;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::oneshot;
use tracing::{debug, info};

use crate::console::{self, PhaseTimer};
use crate::constraints::create_fluent_constraints;
use crate::domain::EmployeeSchedule;

/// Default solving time: 30 seconds.
const DEFAULT_TIME_LIMIT_SECS: u64 = 30;

/// Late acceptance history size.
const LATE_ACCEPTANCE_SIZE: usize = 400;

/// Solver configuration with termination criteria.
#[derive(Debug, Clone, Default)]
pub struct SolverConfig {
    /// Stop after this duration.
    pub time_limit: Option<Duration>,
    /// Stop after this duration without improvement.
    pub unimproved_time_limit: Option<Duration>,
    /// Stop after this many steps.
    pub step_limit: Option<u64>,
    /// Stop after this many steps without improvement.
    pub unimproved_step_limit: Option<u64>,
}

impl SolverConfig {
    /// Creates a config with default 30-second time limit.
    pub fn default_config() -> Self {
        Self {
            time_limit: Some(Duration::from_secs(DEFAULT_TIME_LIMIT_SECS)),
            ..Default::default()
        }
    }

    /// Checks if any termination condition is met.
    fn should_terminate(
        &self,
        elapsed: Duration,
        steps: u64,
        time_since_improvement: Duration,
        steps_since_improvement: u64,
    ) -> bool {
        if let Some(limit) = self.time_limit {
            if elapsed >= limit {
                return true;
            }
        }
        if let Some(limit) = self.unimproved_time_limit {
            if time_since_improvement >= limit {
                return true;
            }
        }
        if let Some(limit) = self.step_limit {
            if steps >= limit {
                return true;
            }
        }
        if let Some(limit) = self.unimproved_step_limit {
            if steps_since_improvement >= limit {
                return true;
            }
        }
        false
    }
}

/// Status of a solving job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SolverStatus {
    /// Not currently solving.
    NotSolving,
    /// Actively solving.
    Solving,
}

impl SolverStatus {
    /// Returns the status as a SCREAMING_SNAKE_CASE string for API responses.
    ///
    /// ```
    /// use employee_scheduling::solver::SolverStatus;
    ///
    /// assert_eq!(SolverStatus::NotSolving.as_str(), "NOT_SOLVING");
    /// assert_eq!(SolverStatus::Solving.as_str(), "SOLVING");
    /// ```
    pub fn as_str(self) -> &'static str {
        match self {
            SolverStatus::NotSolving => "NOT_SOLVING",
            SolverStatus::Solving => "SOLVING",
        }
    }
}

/// A solving job with current state.
pub struct SolveJob {
    /// Unique job identifier.
    pub id: String,
    /// Current status.
    pub status: SolverStatus,
    /// Current best schedule.
    pub schedule: EmployeeSchedule,
    /// Solver configuration.
    pub config: SolverConfig,
    /// Stop signal sender.
    stop_signal: Option<oneshot::Sender<()>>,
}

impl SolveJob {
    /// Creates a new solve job with default config.
    pub fn new(id: String, schedule: EmployeeSchedule) -> Self {
        Self {
            id,
            status: SolverStatus::NotSolving,
            schedule,
            config: SolverConfig::default_config(),
            stop_signal: None,
        }
    }

    /// Creates a new solve job with custom config.
    pub fn with_config(id: String, schedule: EmployeeSchedule, config: SolverConfig) -> Self {
        Self {
            id,
            status: SolverStatus::NotSolving,
            schedule,
            config,
            stop_signal: None,
        }
    }
}

/// Manages Employee Scheduling solving jobs.
///
/// # Examples
///
/// ```
/// use employee_scheduling::solver::SolverService;
/// use employee_scheduling::demo_data::{generate, DemoData};
///
/// let service = SolverService::new();
/// let schedule = generate(DemoData::Small);
///
/// // Create a job (doesn't start solving yet)
/// let job = service.create_job("test-1".to_string(), schedule);
/// assert_eq!(job.read().status, employee_scheduling::solver::SolverStatus::NotSolving);
/// ```
pub struct SolverService {
    jobs: RwLock<HashMap<String, Arc<RwLock<SolveJob>>>>,
}

impl SolverService {
    /// Creates a new solver service.
    pub fn new() -> Self {
        Self {
            jobs: RwLock::new(HashMap::new()),
        }
    }

    /// Creates a new job for the given schedule with default config.
    pub fn create_job(&self, id: String, schedule: EmployeeSchedule) -> Arc<RwLock<SolveJob>> {
        let job = Arc::new(RwLock::new(SolveJob::new(id.clone(), schedule)));
        self.jobs.write().insert(id, job.clone());
        job
    }

    /// Creates a new job with custom config.
    pub fn create_job_with_config(
        &self,
        id: String,
        schedule: EmployeeSchedule,
        config: SolverConfig,
    ) -> Arc<RwLock<SolveJob>> {
        let job = Arc::new(RwLock::new(SolveJob::with_config(id.clone(), schedule, config)));
        self.jobs.write().insert(id, job.clone());
        job
    }

    /// Gets a job by ID.
    pub fn get_job(&self, id: &str) -> Option<Arc<RwLock<SolveJob>>> {
        self.jobs.read().get(id).cloned()
    }

    /// Lists all job IDs.
    pub fn list_jobs(&self) -> Vec<String> {
        self.jobs.read().keys().cloned().collect()
    }

    /// Removes a job by ID.
    pub fn remove_job(&self, id: &str) -> Option<Arc<RwLock<SolveJob>>> {
        self.jobs.write().remove(id)
    }

    /// Starts solving a job in the background.
    pub fn start_solving(&self, job: Arc<RwLock<SolveJob>>) {
        let (tx, rx) = oneshot::channel();
        let config = job.read().config.clone();

        {
            let mut job_guard = job.write();
            job_guard.status = SolverStatus::Solving;
            job_guard.stop_signal = Some(tx);
        }

        let job_clone = job.clone();

        tokio::task::spawn_blocking(move || {
            solve_blocking(job_clone, rx, config);
        });
    }

    /// Stops a solving job.
    pub fn stop_solving(&self, id: &str) -> bool {
        if let Some(job) = self.get_job(id) {
            let mut job_guard = job.write();
            if let Some(stop_signal) = job_guard.stop_signal.take() {
                let _ = stop_signal.send(());
                job_guard.status = SolverStatus::NotSolving;
                return true;
            }
        }
        false
    }
}

impl Default for SolverService {
    fn default() -> Self {
        Self::new()
    }
}

/// Runs the solver in a blocking context.
fn solve_blocking(
    job: Arc<RwLock<SolveJob>>,
    mut stop_rx: oneshot::Receiver<()>,
    config: SolverConfig,
) {
    let initial_schedule = job.read().schedule.clone();
    let job_id = job.read().id.clone();
    let solve_start = Instant::now();

    // Print problem configuration
    console::print_config(
        initial_schedule.shifts.len(),
        initial_schedule.employees.len(),
    );

    info!(
        job_id = %job_id,
        shifts = initial_schedule.shifts.len(),
        employees = initial_schedule.employees.len(),
        "Starting Employee Scheduling solver"
    );

    // Create typed constraints and score director
    let constraints = create_fluent_constraints();
    let mut director = TypedScoreDirector::new(initial_schedule.clone(), constraints);

    // Phase 1: Construction heuristic (round-robin)
    let mut ch_timer = PhaseTimer::start("ConstructionHeuristic", 0);
    let mut current_score = construction_heuristic(&mut director, &mut ch_timer);
    ch_timer.finish();

    // Print solving started after construction
    console::print_solving_started(
        solve_start.elapsed().as_millis() as u64,
        &current_score.to_string(),
        initial_schedule.shifts.len(),
        initial_schedule.shifts.len(),
        initial_schedule.employees.len(),
    );

    // Update job with constructed solution
    update_job(&job, &director, current_score);

    // Phase 2: Late Acceptance local search
    let n_employees = director.working_solution().employees.len();
    if n_employees == 0 {
        info!("No employees to optimize");
        console::print_solving_ended(
            solve_start.elapsed(),
            0,
            1,
            &current_score.to_string(),
            current_score.is_feasible(),
        );
        finish_job(&job, &director, current_score);
        return;
    }

    let mut ls_timer = PhaseTimer::start("LateAcceptance", 1);
    let mut late_scores = vec![current_score; LATE_ACCEPTANCE_SIZE];
    let mut step: u64 = 0;
    let mut rng = rand::thread_rng();

    // Track best score and improvement times
    let mut best_score = current_score;
    let mut last_improvement_time = solve_start;
    let mut last_improvement_step: u64 = 0;

    loop {
        // Check termination conditions
        let elapsed = solve_start.elapsed();
        let time_since_improvement = last_improvement_time.elapsed();
        let steps_since_improvement = step - last_improvement_step;

        if config.should_terminate(elapsed, step, time_since_improvement, steps_since_improvement) {
            debug!("Termination condition met");
            break;
        }

        // Check for stop signal
        if stop_rx.try_recv().is_ok() {
            info!("Solving terminated early by user");
            break;
        }

        // Generate random change move
        if let Some((shift_idx, new_employee_idx)) = generate_move(&director, &mut rng) {
            ls_timer.record_move();

            // Try the move
            let old_score = current_score;
            let old_employee_idx = apply_move(&mut director, shift_idx, new_employee_idx);
            let new_score = director.get_score();

            // Late acceptance criterion
            let late_idx = (step as usize) % LATE_ACCEPTANCE_SIZE;
            let late_score = late_scores[late_idx];

            if new_score >= old_score || new_score >= late_score {
                // Accept
                ls_timer.record_accepted(&current_score.to_string());
                current_score = new_score;
                late_scores[late_idx] = new_score;

                // Track improvements
                if new_score > best_score {
                    best_score = new_score;
                    last_improvement_time = Instant::now();
                    last_improvement_step = step;
                }

                // Periodic update
                if ls_timer.steps_accepted().is_multiple_of(1000) {
                    update_job(&job, &director, current_score);
                    debug!(
                        step,
                        moves_accepted = ls_timer.steps_accepted(),
                        score = %current_score,
                        elapsed_secs = solve_start.elapsed().as_secs(),
                        "Progress update"
                    );
                }

                // Periodic console progress (every 10000 moves)
                if ls_timer.moves_evaluated().is_multiple_of(10000) {
                    console::print_step_progress(
                        ls_timer.steps_accepted(),
                        ls_timer.elapsed(),
                        ls_timer.moves_evaluated(),
                        &current_score.to_string(),
                    );
                }
            } else {
                // Reject - undo
                undo_move(&mut director, shift_idx, old_employee_idx);
            }

            step += 1;
        }
    }

    ls_timer.finish();

    let total_duration = solve_start.elapsed();

    info!(
        job_id = %job_id,
        duration_secs = total_duration.as_secs_f64(),
        steps = step,
        score = %current_score,
        feasible = current_score.is_feasible(),
        "Solving complete"
    );

    console::print_solving_ended(
        total_duration,
        step,
        2,
        &current_score.to_string(),
        current_score.is_feasible(),
    );

    finish_job(&job, &director, current_score);
}

/// Construction heuristic: round-robin employee assignment.
fn construction_heuristic(
    director: &mut TypedScoreDirector<EmployeeSchedule, impl ConstraintSet<EmployeeSchedule, HardSoftDecimalScore>>,
    timer: &mut PhaseTimer,
) -> HardSoftDecimalScore {
    // Initialize score
    let _ = director.calculate_score();

    let n_shifts = director.working_solution().shifts.len();
    let n_employees = director.working_solution().employees.len();

    if n_employees == 0 || n_shifts == 0 {
        return director.get_score();
    }

    // Count already-assigned shifts
    let assigned_count = director
        .working_solution()
        .shifts
        .iter()
        .filter(|s| s.employee_idx.is_some())
        .count();

    // If all shifts already assigned, skip construction
    if assigned_count == n_shifts {
        info!("All shifts already assigned, skipping construction heuristic");
        return director.get_score();
    }

    // Round-robin assignment for unassigned shifts only
    let mut employee_idx = 0;
    for shift_idx in 0..n_shifts {
        if director.working_solution().shifts[shift_idx].employee_idx.is_some() {
            continue;
        }

        timer.record_move();
        director.before_variable_changed(shift_idx);
        director.working_solution_mut().shifts[shift_idx].employee_idx = Some(employee_idx);
        director.after_variable_changed(shift_idx);

        let score = director.get_score();
        timer.record_accepted(&score.to_string());

        employee_idx = (employee_idx + 1) % n_employees;
    }

    director.get_score()
}

/// Generates a random change move (assign a different employee to a shift).
fn generate_move<R: Rng>(
    director: &TypedScoreDirector<EmployeeSchedule, impl ConstraintSet<EmployeeSchedule, HardSoftDecimalScore>>,
    rng: &mut R,
) -> Option<(usize, Option<usize>)> {
    let solution = director.working_solution();
    let n_shifts = solution.shifts.len();
    let n_employees = solution.employees.len();

    if n_shifts == 0 || n_employees == 0 {
        return None;
    }

    // Pick random shift
    let shift_idx = rng.gen_range(0..n_shifts);
    let current_employee = solution.shifts[shift_idx].employee_idx;

    // Pick random new employee (different from current)
    let new_employee_idx = rng.gen_range(0..n_employees);

    // Skip no-op moves
    if current_employee == Some(new_employee_idx) {
        return None;
    }

    Some((shift_idx, Some(new_employee_idx)))
}

/// Applies a change move, returns the old employee index.
fn apply_move(
    director: &mut TypedScoreDirector<EmployeeSchedule, impl ConstraintSet<EmployeeSchedule, HardSoftDecimalScore>>,
    shift_idx: usize,
    new_employee_idx: Option<usize>,
) -> Option<usize> {
    let old_employee_idx = director.working_solution().shifts[shift_idx].employee_idx;

    director.before_variable_changed(shift_idx);
    director.working_solution_mut().shifts[shift_idx].employee_idx = new_employee_idx;
    director.after_variable_changed(shift_idx);

    old_employee_idx
}

/// Undoes a change move.
fn undo_move(
    director: &mut TypedScoreDirector<EmployeeSchedule, impl ConstraintSet<EmployeeSchedule, HardSoftDecimalScore>>,
    shift_idx: usize,
    old_employee_idx: Option<usize>,
) {
    director.before_variable_changed(shift_idx);
    director.working_solution_mut().shifts[shift_idx].employee_idx = old_employee_idx;
    director.after_variable_changed(shift_idx);
}

/// Updates job with current solution.
fn update_job(
    job: &Arc<RwLock<SolveJob>>,
    director: &TypedScoreDirector<EmployeeSchedule, impl ConstraintSet<EmployeeSchedule, HardSoftDecimalScore>>,
    score: HardSoftDecimalScore,
) {
    let mut job_guard = job.write();
    job_guard.schedule = director.clone_working_solution();
    job_guard.schedule.score = Some(score);
}

/// Finishes job and sets status.
fn finish_job(
    job: &Arc<RwLock<SolveJob>>,
    director: &TypedScoreDirector<EmployeeSchedule, impl ConstraintSet<EmployeeSchedule, HardSoftDecimalScore>>,
    score: HardSoftDecimalScore,
) {
    let mut job_guard = job.write();
    job_guard.schedule = director.clone_working_solution();
    job_guard.schedule.score = Some(score);
    job_guard.status = SolverStatus::NotSolving;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::demo_data::{generate, DemoData};

    #[test]
    fn test_construction_heuristic() {
        let schedule = generate(DemoData::Small);
        let constraints = create_fluent_constraints();
        let mut director = TypedScoreDirector::new(schedule, constraints);

        let mut timer = PhaseTimer::start("ConstructionHeuristic", 0);
        let score = construction_heuristic(&mut director, &mut timer);

        // All shifts should be assigned
        let assigned_count = director
            .working_solution()
            .shifts
            .iter()
            .filter(|s| s.employee_idx.is_some())
            .count();
        let total_shifts = director.working_solution().shifts.len();
        assert_eq!(assigned_count, total_shifts);
        assert!(score.hard_scaled() <= 0); // May have some violations
    }
}
