//! SolutionManager for job management and score analysis.
//!
//! Provides the high-level API for:
//! - Starting/stopping solve jobs with callbacks
//! - Tracking solver status per job
//! - Analyzing solutions for constraint violations

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;

use crate::basic::SolverEvent;

/// Analysis of a single constraint's contribution to the score.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConstraintAnalysis<Sc> {
    /// Name of the constraint.
    pub name: String,
    /// Weight of the constraint.
    pub weight: Sc,
    /// Score contribution from this constraint.
    pub score: Sc,
    /// Number of matches (violations or rewards).
    pub match_count: usize,
}

/// Result of analyzing a solution's constraints.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreAnalysis<Sc> {
    /// The total score.
    pub score: Sc,
    /// Analysis of each constraint.
    pub constraints: Vec<ConstraintAnalysis<Sc>>,
}

/// Trait for solutions that can be analyzed for constraint violations.
///
/// This trait is implemented by the `#[planning_solution]` macro when
/// `constraints` is specified. It provides constraint analysis without
/// knowing the concrete solution type.
pub trait Analyzable: PlanningSolution + Clone + Send + 'static {
    /// Analyzes the solution and returns constraint breakdowns.
    fn analyze(&self) -> ScoreAnalysis<Self::Score>;
}

/// Trait for solutions that can be solved with events.
///
/// This trait is implemented by the `#[planning_solution]` macro when
/// `constraints` is specified. It allows `SolutionManager` to call
/// `solve_with_events` without knowing the concrete solution type.
pub trait Solvable: PlanningSolution + Clone + Send + 'static {
    /// Solves the solution with event callbacks.
    fn solve_with_events<E, F>(self, on_event: E, on_best_solution: F) -> Self
    where
        E: FnMut(SolverEvent<Self::Score>),
        F: FnMut(&Self, Self::Score);
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
    /// Returns the status as a string.
    pub fn as_str(self) -> &'static str {
        match self {
            SolverStatus::NotSolving => "NOT_SOLVING",
            SolverStatus::Solving => "SOLVING",
        }
    }
}

/// Internal job state.
struct SolveJob<S> {
    status: SolverStatus,
    solution: S,
    terminate_flag: Arc<AtomicBool>,
    #[allow(dead_code)]
    handle: Option<JoinHandle<()>>,
}

/// Manages solve jobs with callbacks for best solution updates.
///
/// This is the Rust equivalent of Python's `SolverManager` + `SolutionManager`.
/// It provides job management for concurrent solving and constraint analysis.
pub struct SolutionManager<S: Solvable> {
    jobs: Mutex<HashMap<String, SolveJob<S>>>,
}

impl<S: Solvable> Default for SolutionManager<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Solvable> SolutionManager<S>
where
    S::Score: Score,
{
    /// Creates a new SolutionManager.
    pub fn new() -> Self {
        Self {
            jobs: Mutex::new(HashMap::new()),
        }
    }

    /// Starts solving a problem and calls the listener with each best solution.
    ///
    /// This is the main API matching Python/Timefold's `solve_and_listen`.
    /// It calls `solve_with_events` internally - no manual wiring needed.
    ///
    /// # Arguments
    ///
    /// * `problem_id` - Unique identifier for this job
    /// * `initial_solution` - The starting solution
    /// * `listener` - Callback invoked with each new best solution
    pub fn solve_and_listen<L>(
        &self,
        problem_id: impl Into<String>,
        initial_solution: S,
        listener: L,
    ) where
        L: Fn(&S) + Send + 'static,
    {
        let problem_id = problem_id.into();
        let terminate_flag = Arc::new(AtomicBool::new(false));

        // Create job entry
        {
            let mut jobs = self.jobs.lock().unwrap();
            jobs.insert(
                problem_id.clone(),
                SolveJob {
                    status: SolverStatus::Solving,
                    solution: initial_solution.clone(),
                    terminate_flag: terminate_flag.clone(),
                    handle: None,
                },
            );
        }

        // Spawn solving thread
        let handle = std::thread::spawn(move || {
            // Call solve_with_events - the macro-generated method
            let result = initial_solution.solve_with_events(
                |_event| {
                    // Events can be logged/handled here if needed
                },
                |best_solution, _score| {
                    // Call listener with each new best solution
                    listener(best_solution);
                },
            );

            // Final callback with result
            listener(&result);
        });

        // Update job with handle
        {
            let mut jobs = self.jobs.lock().unwrap();
            if let Some(job) = jobs.get_mut(&problem_id) {
                job.handle = Some(handle);
            }
        }
    }

    /// Gets the solver status for a job.
    pub fn get_solver_status(&self, problem_id: &str) -> SolverStatus {
        let jobs = self.jobs.lock().unwrap();
        jobs.get(problem_id)
            .map(|job| job.status)
            .unwrap_or(SolverStatus::NotSolving)
    }

    /// Gets the current best solution for a job.
    pub fn get_solution(&self, problem_id: &str) -> Option<S> {
        let jobs = self.jobs.lock().unwrap();
        jobs.get(problem_id).map(|job| job.solution.clone())
    }

    /// Updates the solution for a job (called from listener callback).
    pub fn update_solution(&self, problem_id: &str, solution: S) {
        let mut jobs = self.jobs.lock().unwrap();
        if let Some(job) = jobs.get_mut(problem_id) {
            job.solution = solution;
        }
    }

    /// Marks a job as finished.
    pub fn mark_finished(&self, problem_id: &str) {
        let mut jobs = self.jobs.lock().unwrap();
        if let Some(job) = jobs.get_mut(problem_id) {
            job.status = SolverStatus::NotSolving;
        }
    }

    /// Requests early termination of a job.
    ///
    /// Returns `true` if the job was found and termination was requested.
    pub fn terminate_early(&self, problem_id: &str) -> bool {
        let jobs = self.jobs.lock().unwrap();
        if let Some(job) = jobs.get(problem_id) {
            job.terminate_flag.store(true, Ordering::SeqCst);
            true
        } else {
            false
        }
    }

    /// Removes a job from the manager.
    pub fn remove_job(&self, problem_id: &str) -> Option<S> {
        let mut jobs = self.jobs.lock().unwrap();
        jobs.remove(problem_id).map(|job| job.solution)
    }

    /// Lists all job IDs.
    pub fn list_jobs(&self) -> Vec<String> {
        let jobs = self.jobs.lock().unwrap();
        jobs.keys().cloned().collect()
    }
}

impl<S> SolutionManager<S>
where
    S: Solvable + Analyzable,
    S::Score: Score,
{
    /// Analyzes a solution for constraint violations.
    ///
    /// Returns a breakdown of each constraint's contribution to the score.
    pub fn analyze(&self, solution: &S) -> ScoreAnalysis<S::Score> {
        solution.analyze()
    }
}
