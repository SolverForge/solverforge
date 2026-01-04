//! Solver service for Vehicle Routing Problem.
//!
//! Uses Late Acceptance local search with list-change moves (visit relocation).
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
use crate::constraints::create_constraints;
use crate::domain::VehicleRoutePlan;

/// Default solving time: 30 seconds.
const DEFAULT_TIME_LIMIT_SECS: u64 = 30;

/// Late acceptance history size.
const LATE_ACCEPTANCE_SIZE: usize = 400;

/// Solver configuration with termination criteria.
///
/// Multiple termination conditions combine with OR logic (any triggers termination).
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
    /// use vehicle_routing::solver::SolverStatus;
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
    /// Current best solution.
    pub plan: VehicleRoutePlan,
    /// Solver configuration.
    pub config: SolverConfig,
    /// Stop signal sender.
    stop_signal: Option<oneshot::Sender<()>>,
}

impl SolveJob {
    /// Creates a new solve job with default config.
    pub fn new(id: String, plan: VehicleRoutePlan) -> Self {
        Self {
            id,
            status: SolverStatus::NotSolving,
            plan,
            config: SolverConfig::default_config(),
            stop_signal: None,
        }
    }

    /// Creates a new solve job with custom config.
    pub fn with_config(id: String, plan: VehicleRoutePlan, config: SolverConfig) -> Self {
        Self {
            id,
            status: SolverStatus::NotSolving,
            plan,
            config,
            stop_signal: None,
        }
    }
}

/// Manages VRP solving jobs.
///
/// # Examples
///
/// ```
/// use vehicle_routing::solver::SolverService;
/// use vehicle_routing::demo_data::generate_philadelphia;
///
/// let service = SolverService::new();
/// let plan = generate_philadelphia();
///
/// // Create a job (doesn't start solving yet)
/// let job = service.create_job("test-1".to_string(), plan);
/// assert_eq!(job.read().status, vehicle_routing::solver::SolverStatus::NotSolving);
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

    /// Creates a new job for the given plan with default config.
    pub fn create_job(&self, id: String, plan: VehicleRoutePlan) -> Arc<RwLock<SolveJob>> {
        let job = Arc::new(RwLock::new(SolveJob::new(id.clone(), plan)));
        self.jobs.write().insert(id, job.clone());
        job
    }

    /// Creates a new job with custom config.
    pub fn create_job_with_config(
        &self,
        id: String,
        plan: VehicleRoutePlan,
        config: SolverConfig,
    ) -> Arc<RwLock<SolveJob>> {
        let job = Arc::new(RwLock::new(SolveJob::with_config(id.clone(), plan, config)));
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
    let initial_plan = job.read().plan.clone();
    let job_id = job.read().id.clone();
    let solve_start = Instant::now();

    // Print problem configuration
    console::print_config(
        initial_plan.vehicles.len(),
        initial_plan.visits.len(),
        initial_plan.locations.len(),
    );

    info!(
        job_id = %job_id,
        visits = initial_plan.visits.len(),
        vehicles = initial_plan.vehicles.len(),
        "Starting VRP solver"
    );

    // Create typed constraints and score director
    let constraints = create_constraints();
    let mut director = TypedScoreDirector::new(initial_plan.clone(), constraints);

    // Phase 1: Construction heuristic (round-robin)
    let mut ch_timer = PhaseTimer::start("ConstructionHeuristic", 0);
    let mut current_score = construction_heuristic(&mut director, &mut ch_timer);
    ch_timer.finish();

    // Print solving started after construction
    console::print_solving_started(
        solve_start.elapsed().as_millis() as u64,
        &current_score.to_string(),
        initial_plan.visits.len(),
        initial_plan.visits.len(),
        initial_plan.vehicles.len(),
    );

    // Update job with constructed solution
    update_job(&job, &director, current_score);

    // Phase 2: Late Acceptance local search
    let n_vehicles = director.working_solution().vehicles.len();
    if n_vehicles == 0 {
        info!("No vehicles to optimize");
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

        // Generate random list change move
        if let Some((src_vehicle, src_pos, dst_vehicle, dst_pos)) =
            generate_move(&director, &mut rng)
        {
            ls_timer.record_move();

            // Try the move
            let old_score = current_score;
            let visit_idx = apply_move(&mut director, src_vehicle, src_pos, dst_vehicle, dst_pos);
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
                undo_move(&mut director, src_vehicle, src_pos, dst_vehicle, dst_pos, visit_idx);
            }

            step += 1;
        }
    }

    ls_timer.finish();

    let total_duration = solve_start.elapsed();
    let total_moves = step;

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
        total_moves,
        2,
        &current_score.to_string(),
        current_score.is_feasible(),
    );

    finish_job(&job, &director, current_score);
}

/// Construction heuristic: round-robin visit assignment.
///
/// Skips construction if all visits are already assigned (continue mode).
fn construction_heuristic(
    director: &mut TypedScoreDirector<VehicleRoutePlan, impl ConstraintSet<VehicleRoutePlan, HardSoftScore>>,
    timer: &mut PhaseTimer,
) -> HardSoftScore {
    // Initialize score
    let _ = director.calculate_score();

    let n_visits = director.working_solution().visits.len();
    let n_vehicles = director.working_solution().vehicles.len();

    if n_vehicles == 0 || n_visits == 0 {
        return director.get_score();
    }

    // Count already-assigned visits
    let assigned_count: usize = director
        .working_solution()
        .vehicles
        .iter()
        .map(|v| v.visits.len())
        .sum();

    // If all visits already assigned, skip construction (continue mode)
    if assigned_count == n_visits {
        info!("All visits already assigned, skipping construction heuristic");
        return director.get_score();
    }

    // Build set of already-assigned visits
    let assigned: std::collections::HashSet<usize> = director
        .working_solution()
        .vehicles
        .iter()
        .flat_map(|v| v.visits.iter().copied())
        .collect();

    // Round-robin assignment for unassigned visits only
    let mut vehicle_idx = 0;
    for visit_idx in 0..n_visits {
        if assigned.contains(&visit_idx) {
            continue;
        }

        timer.record_move();
        director.before_variable_changed(vehicle_idx);
        director.working_solution_mut().vehicles[vehicle_idx]
            .visits
            .push(visit_idx);
        director.after_variable_changed(vehicle_idx);

        let score = director.get_score();
        timer.record_accepted(&score.to_string());

        vehicle_idx = (vehicle_idx + 1) % n_vehicles;
    }

    director.get_score()
}

/// Generates a random list change move.
/// Returns (source_vehicle, source_pos, dest_vehicle, dest_pos) or None.
fn generate_move<R: Rng>(
    director: &TypedScoreDirector<VehicleRoutePlan, impl ConstraintSet<VehicleRoutePlan, HardSoftScore>>,
    rng: &mut R,
) -> Option<(usize, usize, usize, usize)> {
    let solution = director.working_solution();
    let n_vehicles = solution.vehicles.len();

    // Find a non-empty source vehicle
    let non_empty: Vec<usize> = solution
        .vehicles
        .iter()
        .enumerate()
        .filter(|(_, v)| !v.visits.is_empty())
        .map(|(i, _)| i)
        .collect();

    if non_empty.is_empty() {
        return None;
    }

    let src_vehicle = non_empty[rng.gen_range(0..non_empty.len())];
    let src_len = solution.vehicles[src_vehicle].visits.len();
    let src_pos = rng.gen_range(0..src_len);

    // Pick destination vehicle and position
    let dst_vehicle = rng.gen_range(0..n_vehicles);
    let dst_len = solution.vehicles[dst_vehicle].visits.len();

    // Valid insertion position
    let max_pos = if src_vehicle == dst_vehicle {
        src_len // After removal, can insert at 0..src_len-1+1 = 0..src_len
    } else {
        dst_len + 1 // Can insert at end
    };

    if max_pos == 0 {
        return None;
    }

    let dst_pos = rng.gen_range(0..max_pos);

    // Skip no-op moves
    if src_vehicle == dst_vehicle {
        let effective_dst = if dst_pos > src_pos {
            dst_pos - 1
        } else {
            dst_pos
        };
        if src_pos == effective_dst {
            return None;
        }
    }

    Some((src_vehicle, src_pos, dst_vehicle, dst_pos))
}

/// Applies a list change move, returns the moved visit index.
fn apply_move(
    director: &mut TypedScoreDirector<VehicleRoutePlan, impl ConstraintSet<VehicleRoutePlan, HardSoftScore>>,
    src_vehicle: usize,
    src_pos: usize,
    dst_vehicle: usize,
    dst_pos: usize,
) -> usize {
    // Notify and remove
    director.before_variable_changed(src_vehicle);
    let visit_idx = director.working_solution_mut().vehicles[src_vehicle]
        .visits
        .remove(src_pos);
    director.after_variable_changed(src_vehicle);

    // Adjust position for intra-list moves
    let adjusted_dst = if src_vehicle == dst_vehicle && dst_pos > src_pos {
        dst_pos - 1
    } else {
        dst_pos
    };

    // Notify and insert
    director.before_variable_changed(dst_vehicle);
    director.working_solution_mut().vehicles[dst_vehicle]
        .visits
        .insert(adjusted_dst, visit_idx);
    director.after_variable_changed(dst_vehicle);

    visit_idx
}

/// Undoes a list change move.
fn undo_move(
    director: &mut TypedScoreDirector<VehicleRoutePlan, impl ConstraintSet<VehicleRoutePlan, HardSoftScore>>,
    src_vehicle: usize,
    src_pos: usize,
    dst_vehicle: usize,
    dst_pos: usize,
    visit_idx: usize,
) {
    // Compute where we inserted
    let adjusted_dst = if src_vehicle == dst_vehicle && dst_pos > src_pos {
        dst_pos - 1
    } else {
        dst_pos
    };

    // Remove from destination
    director.before_variable_changed(dst_vehicle);
    director.working_solution_mut().vehicles[dst_vehicle]
        .visits
        .remove(adjusted_dst);
    director.after_variable_changed(dst_vehicle);

    // Insert back at source
    director.before_variable_changed(src_vehicle);
    director.working_solution_mut().vehicles[src_vehicle]
        .visits
        .insert(src_pos, visit_idx);
    director.after_variable_changed(src_vehicle);
}

/// Updates job with current solution.
fn update_job(
    job: &Arc<RwLock<SolveJob>>,
    director: &TypedScoreDirector<VehicleRoutePlan, impl ConstraintSet<VehicleRoutePlan, HardSoftScore>>,
    score: HardSoftScore,
) {
    let mut job_guard = job.write();
    job_guard.plan = director.clone_working_solution();
    job_guard.plan.score = Some(score);
}

/// Finishes job and sets status.
fn finish_job(
    job: &Arc<RwLock<SolveJob>>,
    director: &TypedScoreDirector<VehicleRoutePlan, impl ConstraintSet<VehicleRoutePlan, HardSoftScore>>,
    score: HardSoftScore,
) {
    let mut job_guard = job.write();
    job_guard.plan = director.clone_working_solution();
    job_guard.plan.score = Some(score);
    job_guard.status = SolverStatus::NotSolving;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::demo_data::generate_philadelphia;

    #[test]
    fn test_construction_heuristic() {
        let plan = generate_philadelphia();
        let constraints = create_constraints();
        let mut director = TypedScoreDirector::new(plan, constraints);

        // Create a timer but don't print (we're in a test)
        let mut timer = PhaseTimer::start("ConstructionHeuristic", 0);
        let score = construction_heuristic(&mut director, &mut timer);

        // All visits should be assigned
        let total_visits: usize = director
            .working_solution()
            .vehicles
            .iter()
            .map(|v| v.visits.len())
            .sum();
        assert_eq!(total_visits, 49); // Philadelphia has 49 visits
        assert!(score.hard() <= 0); // May have some violations
    }
}
