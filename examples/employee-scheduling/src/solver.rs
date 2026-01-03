//! Background solver service for Employee Scheduling.
//!
//! Uses TypedScoreDirector for incremental scoring (O(1) per move instead of O(n²)).

use parking_lot::RwLock;
use rand::Rng;
use solverforge::{
    prelude::*, AcceptorConfig, LateAcceptanceConfig, LocalSearchConfig, PhaseConfig, SolverConfig,
    TypedScoreDirector,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::oneshot;

use crate::console::{self, PhaseTimer};
use crate::constraints::create_fluent_constraints;
use crate::domain::EmployeeSchedule;

/// Default termination: 30 seconds
const DEFAULT_TERMINATION_SECONDS: u64 = 30;

/// Default late acceptance size.
const DEFAULT_LATE_ACCEPTANCE_SIZE: usize = 400;

/// Status of a solving job.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SolverStatus {
    NotSolving,
    Solving,
}

/// A solving job that can be queried for current state.
pub struct SolveJob {
    pub id: String,
    pub status: SolverStatus,
    pub schedule: EmployeeSchedule,
    stop_signal: Option<oneshot::Sender<()>>,
}

impl SolveJob {
    pub fn new(id: String, schedule: EmployeeSchedule) -> Self {
        Self {
            id,
            status: SolverStatus::NotSolving,
            schedule,
            stop_signal: None,
        }
    }
}

/// Manages solving jobs with configurable solver settings.
pub struct SolverService {
    jobs: RwLock<HashMap<String, Arc<RwLock<SolveJob>>>>,
    config: SolverConfig,
}

impl SolverService {
    /// Creates a new solver service with default configuration.
    ///
    /// Default: 30 seconds termination, Late Acceptance with size 400.
    pub fn new() -> Self {
        Self::with_config(Self::default_config())
    }

    /// Creates a new solver service with custom configuration.
    pub fn with_config(config: SolverConfig) -> Self {
        Self {
            jobs: RwLock::new(HashMap::new()),
            config,
        }
    }

    /// Returns the default solver configuration
    pub fn default_config() -> SolverConfig {
        SolverConfig::new()
            .with_termination_seconds(DEFAULT_TERMINATION_SECONDS)
            .with_phase(PhaseConfig::ConstructionHeuristic(Default::default()))
            .with_phase(PhaseConfig::LocalSearch(LocalSearchConfig {
                acceptor: Some(AcceptorConfig::LateAcceptance(LateAcceptanceConfig {
                    late_acceptance_size: Some(DEFAULT_LATE_ACCEPTANCE_SIZE),
                })),
                termination: None,
                forager: None,
                move_selector: None,
            }))
    }

    /// Returns a reference to the current configuration.
    pub fn config(&self) -> &SolverConfig {
        &self.config
    }

    /// Creates a new solving job with the given schedule.
    pub fn create_job(&self, id: String, schedule: EmployeeSchedule) -> Arc<RwLock<SolveJob>> {
        let job = Arc::new(RwLock::new(SolveJob::new(id.clone(), schedule)));
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

        {
            let mut job_guard = job.write();
            job_guard.status = SolverStatus::Solving;
            job_guard.stop_signal = Some(tx);
        }

        let job_clone = job.clone();
        let config = self.config.clone();

        // Spawn a blocking task for CPU-bound solving
        tokio::task::spawn_blocking(move || {
            solve_blocking(job_clone, rx, config);
        });
    }

    /// Stops a solving job
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

/// Extracts termination time limit from config.
fn get_time_limit(config: &SolverConfig) -> Duration {
    config
        .termination
        .as_ref()
        .and_then(|t| t.time_limit())
        .unwrap_or(Duration::from_secs(DEFAULT_TERMINATION_SECONDS))
}

/// Extracts late acceptance size from config.
fn get_late_acceptance_size(config: &SolverConfig) -> usize {
    for phase in &config.phases {
        if let PhaseConfig::LocalSearch(ls) = phase {
            if let Some(AcceptorConfig::LateAcceptance(la)) = &ls.acceptor {
                if let Some(size) = la.late_acceptance_size {
                    return size;
                }
            }
        }
    }
    DEFAULT_LATE_ACCEPTANCE_SIZE
}

/// Runs the solver in a blocking context using TypedScoreDirector for incremental scoring.
fn solve_blocking(
    job: Arc<RwLock<SolveJob>>,
    mut stop_rx: oneshot::Receiver<()>,
    config: SolverConfig,
) {
    use num_format::{Locale, ToFormattedString};
    use owo_colors::OwoColorize;

    // Extract configuration
    let time_limit = get_time_limit(&config);
    let late_acceptance_size = get_late_acceptance_size(&config);

    // Get initial schedule
    let initial_schedule = job.read().schedule.clone();
    let solve_start = Instant::now();

    // Print initial state
    let entity_count = initial_schedule.shifts.len();
    let variable_count = entity_count;
    let value_count = initial_schedule.employees.len();

    console::print_solving_started(0, "0hard/0soft", entity_count, variable_count, value_count);

    println!(
        "  {} Termination: {} seconds",
        "⚙".bright_black(),
        time_limit.as_secs().to_string().white()
    );

    // Employee count for value range (0..n indices)
    let n_employees = initial_schedule.employees.len();

    // Create typed constraints for incremental scoring (fluent API)
    let constraints = create_fluent_constraints();
    let mut director = TypedScoreDirector::new(initial_schedule, constraints);

    // ═══════════════════════════════════════════════════════════════════════
    // PHASE 1: Construction Heuristic
    // ═══════════════════════════════════════════════════════════════════════
    let mut phase1_timer = PhaseTimer::start("Construction Heuristic", 0);

    // Initialize the score director (populates constraint state)
    let mut current_score = director.calculate_score();

    for i in 0..director.working_solution().shifts.len() {
        let required_skill = director.working_solution().shifts[i].required_skill.clone();

        // Find best employee: prefer skill match, but always assign someone
        let mut best_emp_idx = 0; // Fallback to first employee
        for (emp_idx, emp) in director.working_solution().employees.iter().enumerate() {
            if emp.skills.contains(&required_skill) {
                best_emp_idx = emp_idx;
                break;
            }
        }

        // Always assign (construction heuristic must initialize all entities)
        director.before_variable_changed(i);
        director.working_solution_mut().shifts[i].employee_idx = Some(best_emp_idx);
        director.after_variable_changed(i);

        current_score = director.get_score();
        phase1_timer.record_move();
        phase1_timer.record_accepted(&format!("{}", current_score));
    }

    // Update job with constructed solution
    {
        let mut job_guard = job.write();
        job_guard.schedule = director.clone_working_solution();
        job_guard.schedule.score = Some(current_score);
    }

    phase1_timer.finish();

    // ═══════════════════════════════════════════════════════════════════════
    // PHASE 2: Late Acceptance Local Search (with incremental scoring)
    // ═══════════════════════════════════════════════════════════════════════
    let mut phase2_timer = PhaseTimer::start("Late Acceptance Local Search", 1);

    let mut late_scores: Vec<HardSoftDecimalScore> = vec![current_score; late_acceptance_size];
    let mut step: u64 = 0;
    let mut moves_evaluated: u64 = 0;
    let mut moves_accepted: u64 = 0;
    let mut rng = rand::thread_rng();
    let mut last_print = Instant::now();
    let n_shifts = director.working_solution().shifts.len();

    // Time-based termination
    while solve_start.elapsed() < time_limit {
        // Check for stop signal (terminateEarly)
        if stop_rx.try_recv().is_ok() {
            println!(
                "  {} {}",
                "⚠".yellow(),
                "Solving terminated early by user".yellow()
            );
            break;
        }

        // Generate a random move: change one shift's employee assignment
        let shift_idx = rng.gen_range(0..n_shifts);
        let old_employee_idx = director.working_solution().shifts[shift_idx].employee_idx;

        // Try a random employee index
        let new_employee_idx = if n_employees == 0 {
            None
        } else {
            Some(rng.gen_range(0..n_employees))
        };

        // Skip if no change
        if new_employee_idx == old_employee_idx {
            continue;
        }

        // INCREMENTAL: Apply move with delta scoring
        director.before_variable_changed(shift_idx);
        director.working_solution_mut().shifts[shift_idx].employee_idx = new_employee_idx;
        director.after_variable_changed(shift_idx);

        let new_score = director.get_score(); // O(1) - cached
        moves_evaluated += 1;

        // Late Acceptance criterion
        let late_idx = (step as usize) % late_acceptance_size;
        let late_score = late_scores[late_idx];

        if new_score >= current_score || new_score >= late_score {
            // Accept the move
            moves_accepted += 1;
            current_score = new_score;
            late_scores[late_idx] = new_score;
            phase2_timer.record_accepted(&format!("{}", current_score));

            // Update job periodically (every 1000 accepted steps)
            if moves_accepted.is_multiple_of(1000) {
                let mut job_guard = job.write();
                job_guard.schedule = director.clone_working_solution();
                job_guard.schedule.score = Some(current_score);
            }
        } else {
            // Reject - undo move (incremental)
            director.before_variable_changed(shift_idx);
            director.working_solution_mut().shifts[shift_idx].employee_idx = old_employee_idx;
            director.after_variable_changed(shift_idx);
        }

        step += 1;
        phase2_timer.record_move();

        // Print progress every second
        if last_print.elapsed().as_secs() >= 1 {
            let elapsed = phase2_timer.elapsed();
            let remaining = time_limit.saturating_sub(solve_start.elapsed());
            let moves_per_sec = if elapsed.as_secs_f64() > 0.0 {
                (moves_evaluated as f64 / elapsed.as_secs_f64()) as u64
            } else {
                0
            };

            println!(
                "  {} Step {:>6} │ {:.1}s elapsed │ {:.0}s remaining │ {}/sec │ Score: {}",
                "·".bright_black(),
                step.to_formatted_string(&Locale::en).white(),
                elapsed.as_secs_f64(),
                remaining.as_secs_f64(),
                moves_per_sec
                    .to_formatted_string(&Locale::en)
                    .bright_magenta()
                    .bold(),
                format!("{}", current_score).bright_cyan()
            );
            last_print = Instant::now();
        }
    }

    phase2_timer.finish();

    // ═══════════════════════════════════════════════════════════════════════
    // Summary
    // ═══════════════════════════════════════════════════════════════════════
    let total_duration = solve_start.elapsed();
    let is_feasible = current_score.is_feasible();

    console::print_solving_ended(
        total_duration,
        moves_evaluated,
        2, // phase_count: Construction Heuristic + Local Search
        &format!("{}", current_score),
        is_feasible,
    );

    // Final update
    {
        let mut job_guard = job.write();
        job_guard.schedule = director.clone_working_solution();
        job_guard.schedule.score = Some(current_score);
        job_guard.status = SolverStatus::NotSolving;
    }
}
