//! Event listener integration for SolverForge event system.

use std::fmt::Display;
use std::time::Instant;

use solverforge_core::domain::PlanningSolution;
use solverforge_solver::event::{SolverEventListener, PhaseLifecycleListener, StepLifecycleListener};

use crate::console::ConsoleInstance;
use crate::formatter::{format_duration, format_number};

/// Event listener that sends solver events to SERIO Console.
///
/// This listener integrates with SolverForge's event system and routes
/// events to console channels with automatic formatting and thread tagging.
///
/// # Examples
///
/// ```
/// use solverforge_console::{ConsoleManager, ConsoleMode};
/// use solverforge_console::event_listener::ConsoleEventListener;
/// use std::sync::Arc;
///
/// let mut manager = ConsoleManager::new(ConsoleMode::Simple);
/// let mut console = manager.create_console("vrp-job-001".to_string());
///
/// // Create event listener for solver integration
/// let listener = Arc::new(ConsoleEventListener::new(console.clone()));
///
/// // Add to solver event system
/// // solver.add_solver_listener(listener.clone());
/// // solver.add_phase_listener(listener.clone());
/// ```
#[derive(Debug, Clone)]
pub struct ConsoleEventListener {
    console: ConsoleInstance,
    phase_start_time: Option<Instant>,
}

impl ConsoleEventListener {
    /// Creates a new console event listener.
    ///
    /// # Examples
    ///
    /// ```
    /// use solverforge_console::{ConsoleManager, ConsoleMode};
    /// use solverforge_console::event_listener::ConsoleEventListener;
    ///
    /// let mut manager = ConsoleManager::new(ConsoleMode::Simple);
    /// let console = manager.create_console("job-001".to_string());
    /// let listener = ConsoleEventListener::new(console);
    /// ```
    pub fn new(console: ConsoleInstance) -> Self {
        Self {
            console,
            phase_start_time: None,
        }
    }

    /// Returns a reference to the console instance.
    pub fn console(&self) -> &ConsoleInstance {
        &self.console
    }

    /// Returns a mutable reference to the console instance.
    pub fn console_mut(&mut self) -> &mut ConsoleInstance {
        &mut self.console
    }
}

impl<S> SolverEventListener<S> for ConsoleEventListener
where
    S: PlanningSolution,
    S::Score: Display,
{
    fn on_best_solution_changed(&self, _solution: &S, score: &S::Score) {
        let mut console = self.console.clone();
        let core = console.core_channel();
        core.info(format!("New best solution: {}", score));
    }

    fn on_solving_started(&self, _solution: &S) {
        let mut console = self.console.clone();
        let core = console.core_channel();
        core.info("Solving started");
    }

    fn on_solving_ended(&self, _solution: &S, is_terminated_early: bool) {
        let mut console = self.console.clone();
        let core = console.core_channel();
        if is_terminated_early {
            core.info("Solving ended (terminated early)");
        } else {
            core.info("Solving completed");
        }
    }
}

impl<S> PhaseLifecycleListener<S> for ConsoleEventListener
where
    S: PlanningSolution,
    S::Score: Display,
{
    fn on_phase_started(&self, phase_index: usize, phase_type: &str) {
        // Store start time for duration calculation
        let mut self_mut = self.clone();
        self_mut.phase_start_time = Some(Instant::now());

        let mut console = self.console.clone();
        let core = console.core_channel();
        core.info(format!("Phase {} ({}) started", phase_index, phase_type));
    }

    fn on_phase_ended(&self, phase_index: usize, phase_type: &str) {
        let mut console = self.console.clone();
        let core = console.core_channel();

        if let Some(start_time) = self.phase_start_time {
            let duration = start_time.elapsed();
            core.info(format!(
                "Phase {} ({}) ended after {}",
                phase_index,
                phase_type,
                format_duration(duration)
            ));
        } else {
            core.info(format!("Phase {} ({}) ended", phase_index, phase_type));
        }
    }
}

impl<S> StepLifecycleListener<S> for ConsoleEventListener
where
    S: PlanningSolution,
    S::Score: Display,
{
    fn on_step_started(&self, step_index: u64) {
        // Sample: only log every 10,000 steps to avoid overhead
        if step_index % 10_000 == 0 {
            let mut console = self.console.clone();
            let core = console.core_channel();
            core.debug(format!("Step {}", format_number(step_index)));
        }
    }

    fn on_step_ended(&self, step_index: u64, score: &S::Score) {
        // Sample: only log every 10,000 steps
        if step_index % 10_000 == 0 {
            let mut console = self.console.clone();
            let core = console.core_channel();
            core.debug(format!("Step {} completed: {}", format_number(step_index), score));
        }
    }
}
