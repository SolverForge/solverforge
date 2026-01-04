//! Generic event listener for solver monitoring.
//!
//! The [`ConsoleEventListener`] works with any [`PlanningSolution`] type and provides
//! full explainability through console channels without requiring custom code.

use parking_lot::RwLock;
use solverforge_core::PlanningSolution;
use solverforge_solver::event::{
    PhaseLifecycleListener, SolverEventListener, StepLifecycleListener,
};
use std::marker::PhantomData;
use std::time::Instant;

use crate::{ConsoleInstance, ConsoleManager};
use crate::formatter::{format_duration, format_number};

/// Generic event listener for solver monitoring.
///
/// This listener works with any problem type (VRP, scheduling, N-queens, etc.) without
/// requiring custom implementations. It automatically tracks solver progress, phase metrics,
/// and solution quality through console channels.
///
/// # Examples
///
/// ```no_run
/// use solverforge_console::{ConsoleManager, ConsoleMode, ConsoleEventListener};
///
/// // Initialize console (once at startup)
/// ConsoleManager::init(ConsoleMode::Tui);
/// std::thread::spawn(|| ConsoleManager::run());
///
/// // Create generic listener for any problem type
/// let listener = ConsoleEventListener::new("my-job-123");
///
/// // Attach to solver and use...
/// // listener.on_solving_started(&solution);
/// // listener.record_move();
/// // listener.record_accepted(&score.to_string());
/// ```
#[derive(Debug)]
pub struct ConsoleEventListener<S: PlanningSolution> {
    console: RwLock<ConsoleInstance>,
    solve_start: Instant,
    phase_start: RwLock<Option<Instant>>,
    phase_metrics: RwLock<PhaseMetrics>,
    _phantom: PhantomData<S>,
}

/// Internal metrics tracked during a phase.
#[derive(Debug, Default, Clone)]
struct PhaseMetrics {
    steps_accepted: u64,
    moves_evaluated: u64,
    last_score: String,
}

impl<S: PlanningSolution> ConsoleEventListener<S> {
    /// Creates a new event listener for the given job ID.
    ///
    /// The listener creates its own console instance from the global [`ConsoleManager`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use solverforge_console::{ConsoleManager, ConsoleMode, ConsoleEventListener};
    ///
    /// # ConsoleManager::init(ConsoleMode::Tui);
    /// let listener = ConsoleEventListener::new("vrp-job-001");
    /// ```
    pub fn new(job_id: &str) -> Self {
        let console = ConsoleManager::global().create_console(job_id.to_string());
        Self {
            console: RwLock::new(console),
            solve_start: Instant::now(),
            phase_start: RwLock::new(None),
            phase_metrics: RwLock::new(PhaseMetrics::default()),
            _phantom: PhantomData,
        }
    }

    /// Records a move evaluation.
    ///
    /// Call this after each move is evaluated in the solver loop to track move throughput.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use solverforge_console::{ConsoleManager, ConsoleMode, ConsoleEventListener};
    /// # ConsoleManager::init(ConsoleMode::Tui);
    /// # let listener = ConsoleEventListener::new("job");
    /// // In solver loop
    /// listener.record_move();
    /// ```
    pub fn record_move(&self) {
        self.phase_metrics.write().moves_evaluated += 1;
    }

    /// Records a move acceptance with the current score.
    ///
    /// Call this when a move is accepted to track acceptance rate and score progression.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use solverforge_console::{ConsoleManager, ConsoleMode, ConsoleEventListener};
    /// # ConsoleManager::init(ConsoleMode::Tui);
    /// # let listener = ConsoleEventListener::new("job");
    /// // After accepting a move
    /// listener.record_accepted("0hard/-1234soft");
    /// ```
    pub fn record_accepted(&self, score: &str) {
        let mut metrics = self.phase_metrics.write();
        metrics.steps_accepted += 1;
        metrics.last_score = score.to_string();
    }

    /// Reports periodic progress to the console.
    ///
    /// Call this every N steps (e.g., every 10,000 moves) to emit progress updates.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use solverforge_console::{ConsoleManager, ConsoleMode, ConsoleEventListener};
    /// # ConsoleManager::init(ConsoleMode::Tui);
    /// # let listener = ConsoleEventListener::new("job");
    /// // Every 10,000 moves
    /// if step % 10000 == 0 {
    ///     listener.report_step_progress(step);
    /// }
    /// ```
    pub fn report_step_progress(&self, step: u64) {
        let metrics = self.phase_metrics.read();
        let phase_elapsed = self
            .phase_start
            .read()
            .map(|start| start.elapsed())
            .unwrap_or_default();

        let moves_per_sec = if phase_elapsed.as_secs_f64() > 0.0 {
            (metrics.moves_evaluated as f64 / phase_elapsed.as_secs_f64()) as u64
        } else {
            0
        };

        let mut console = self.console.write();
        let solver_channel = console.channel("solver");

        solver_channel.info(&format!(
            "Step {} | {} | {}/sec | {}",
            format_number(step),
            format_duration(phase_elapsed),
            format_number(moves_per_sec),
            metrics.last_score
        ));

        solver_channel.metric("moves_per_sec", &moves_per_sec.to_string());
        solver_channel.metric("steps_accepted", &metrics.steps_accepted.to_string());
    }
}

/// Generic implementation for any PlanningSolution type.
impl<S: PlanningSolution + std::fmt::Debug> SolverEventListener<S> for ConsoleEventListener<S>
where
    S::Score: std::fmt::Display,
{
    fn on_best_solution_changed(&self, _solution: &S, score: &S::Score) {
        let mut console = self.console.write();
        let core = console.core_channel();

        core.info(&format!("New best solution: {}", score));
        core.metric("best_score", &score.to_string());
    }

    fn on_solving_started(&self, _solution: &S) {
        let mut console = self.console.write();
        let core = console.core_channel();

        core.status(crate::backend::SolverState::Solving);
        core.info("Solving started");
    }

    fn on_solving_ended(&self, _solution: &S, is_terminated_early: bool) {
        let total_duration = self.solve_start.elapsed();
        let metrics = self.phase_metrics.read();

        let moves_per_sec = if total_duration.as_secs_f64() > 0.0 {
            (metrics.moves_evaluated as f64 / total_duration.as_secs_f64()) as u64
        } else {
            0
        };

        let mut console = self.console.write();
        let core = console.core_channel();

        if is_terminated_early {
            core.status(crate::backend::SolverState::TerminatedEarly);
            core.warn("Solving terminated early");
        } else {
            core.status(crate::backend::SolverState::Completed);
            core.info("Solving completed");
        }

        core.info(&format!(
            "Total time: {}, best score: {}, move speed: {}/sec",
            format_duration(total_duration),
            metrics.last_score,
            format_number(moves_per_sec)
        ));

        core.metric("total_duration_ms", &total_duration.as_millis().to_string());
        core.metric("total_moves", &metrics.moves_evaluated.to_string());
        core.metric("total_steps", &metrics.steps_accepted.to_string());
        core.metric("final_score", &metrics.last_score);
    }
}

/// Generic implementation for any PlanningSolution type.
impl<S: PlanningSolution + std::fmt::Debug> PhaseLifecycleListener<S> for ConsoleEventListener<S> {
    fn on_phase_started(&self, phase_index: usize, phase_type: &str) {
        *self.phase_start.write() = Some(Instant::now());
        *self.phase_metrics.write() = PhaseMetrics::default();

        let mut console = self.console.write();
        let solver_channel = console.channel("solver");

        solver_channel.info(&format!("Phase {} ({}) started", phase_index, phase_type));
    }

    fn on_phase_ended(&self, phase_index: usize, phase_type: &str) {
        let phase_duration = self
            .phase_start
            .read()
            .map(|start| start.elapsed())
            .unwrap_or_default();

        let metrics = self.phase_metrics.read();

        let moves_per_sec = if phase_duration.as_secs_f64() > 0.0 {
            (metrics.moves_evaluated as f64 / phase_duration.as_secs_f64()) as u64
        } else {
            0
        };

        let acceptance_rate = if metrics.moves_evaluated > 0 {
            (metrics.steps_accepted as f64 / metrics.moves_evaluated as f64) * 100.0
        } else {
            0.0
        };

        let mut console = self.console.write();
        let solver_channel = console.channel("solver");

        solver_channel.info(&format!(
            "Phase {} ({}) ended: time ({}), best score ({}), speed ({}/sec), steps ({}, {:.1}% accepted)",
            phase_index,
            phase_type,
            format_duration(phase_duration),
            metrics.last_score,
            format_number(moves_per_sec),
            format_number(metrics.steps_accepted),
            acceptance_rate
        ));

        solver_channel.metric(
            &format!("phase_{}_duration_ms", phase_index),
            &phase_duration.as_millis().to_string(),
        );
        solver_channel.metric(
            &format!("phase_{}_moves_per_sec", phase_index),
            &moves_per_sec.to_string(),
        );
        solver_channel.metric(
            &format!("phase_{}_acceptance_rate", phase_index),
            &format!("{:.1}", acceptance_rate),
        );
    }
}

/// Generic implementation for any PlanningSolution type.
impl<S: PlanningSolution + std::fmt::Debug> StepLifecycleListener<S> for ConsoleEventListener<S> {
    fn on_step_started(&self, _step_index: u64) {
        // Step-level events are too granular for console output
        // Metrics are tracked via record_move/record_accepted instead
    }

    fn on_step_ended(&self, _step_index: u64, _score: &S::Score) {
        // Step-level events are too granular for console output
        // Use report_step_progress for periodic updates
    }
}
