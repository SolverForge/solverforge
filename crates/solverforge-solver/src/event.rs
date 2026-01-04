//! Event system for solver monitoring and extensibility.
//!
//! The event system provides hooks for monitoring solver progress and
//! extending solver behavior. Event listeners can be registered to receive
//! notifications about solver lifecycle events.
//!
//! # Event Types
//!
//! - **Solver Events**: Best solution changed, solving started/ended
//! - **Phase Events**: Phase started, phase ended
//! - **Step Events**: Step started, step ended
//!
//! # Usage
//!
//! ```
//! use std::sync::Arc;
//! use solverforge_solver::event::{SolverEventSupport, SolverEventListener};
//! use solverforge_core::domain::PlanningSolution;
//! use solverforge_core::score::SimpleScore;
//!
//! #[derive(Clone, Debug)]
//! struct MySolution { score: Option<SimpleScore> }
//! impl PlanningSolution for MySolution {
//!     type Score = SimpleScore;
//!     fn score(&self) -> Option<Self::Score> { self.score.clone() }
//!     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
//! }
//!
//! #[derive(Debug)]
//! struct MyListener;
//! impl SolverEventListener<MySolution> for MyListener {
//!     fn on_best_solution_changed(&self, _solution: &MySolution, score: &SimpleScore) {
//!         println!("New best: {:?}", score);
//!     }
//! }
//!
//! let mut support = SolverEventSupport::<MySolution>::new();
//! support.add_solver_listener(Arc::new(MyListener));
//! ```

use std::fmt::Debug;
use std::sync::Arc;

use solverforge_core::domain::PlanningSolution;

/// Listener for solver-level events.
///
/// Implement this trait to receive notifications about high-level
/// solver events like best solution changes.
pub trait SolverEventListener<S: PlanningSolution>: Send + Sync + Debug {
    /// Called when a new best solution is found.
    ///
    /// # Arguments
    ///
    /// * `solution` - The new best solution
    /// * `score` - The score of the new best solution
    fn on_best_solution_changed(&self, solution: &S, score: &S::Score);

    /// Called when solving starts.
    fn on_solving_started(&self, _solution: &S) {}

    /// Called when solving ends.
    fn on_solving_ended(&self, _solution: &S, _is_terminated_early: bool) {}
}

/// Listener for phase lifecycle events.
///
/// Implement this trait to receive notifications about phase
/// transitions during solving.
pub trait PhaseLifecycleListener<S: PlanningSolution>: Send + Sync + Debug {
    /// Called when a phase starts.
    ///
    /// # Arguments
    ///
    /// * `phase_index` - The index of the phase (0-based)
    /// * `phase_type` - The type name of the phase
    fn on_phase_started(&self, phase_index: usize, phase_type: &str);

    /// Called when a phase ends.
    ///
    /// # Arguments
    ///
    /// * `phase_index` - The index of the phase (0-based)
    /// * `phase_type` - The type name of the phase
    fn on_phase_ended(&self, phase_index: usize, phase_type: &str);
}

/// Listener for step-level events within a phase.
///
/// Implement this trait to receive fine-grained notifications
/// about individual solving steps.
pub trait StepLifecycleListener<S: PlanningSolution>: Send + Sync + Debug {
    /// Called when a step starts.
    ///
    /// # Arguments
    ///
    /// * `step_index` - The index of the step within the current phase
    fn on_step_started(&self, step_index: u64);

    /// Called when a step ends.
    ///
    /// # Arguments
    ///
    /// * `step_index` - The index of the step within the current phase
    /// * `score` - The score after this step
    fn on_step_ended(&self, step_index: u64, score: &S::Score);
}

/// Central event broadcaster for solver events.
///
/// Manages listener registration and event distribution.
/// All listener methods are called synchronously in registration order.
pub struct SolverEventSupport<S: PlanningSolution> {
    /// Solver-level event listeners.
    solver_listeners: Vec<Arc<dyn SolverEventListener<S>>>,

    /// Phase lifecycle listeners.
    phase_listeners: Vec<Arc<dyn PhaseLifecycleListener<S>>>,

    /// Step lifecycle listeners.
    step_listeners: Vec<Arc<dyn StepLifecycleListener<S>>>,
}

impl<S: PlanningSolution> SolverEventSupport<S> {
    /// Creates a new event support instance.
    pub fn new() -> Self {
        Self {
            solver_listeners: Vec::new(),
            phase_listeners: Vec::new(),
            step_listeners: Vec::new(),
        }
    }

    // === Listener Registration ===

    /// Adds a solver-level event listener.
    pub fn add_solver_listener(&mut self, listener: Arc<dyn SolverEventListener<S>>) {
        self.solver_listeners.push(listener);
    }

    /// Adds a phase lifecycle listener.
    pub fn add_phase_listener(&mut self, listener: Arc<dyn PhaseLifecycleListener<S>>) {
        self.phase_listeners.push(listener);
    }

    /// Adds a step lifecycle listener.
    pub fn add_step_listener(&mut self, listener: Arc<dyn StepLifecycleListener<S>>) {
        self.step_listeners.push(listener);
    }

    /// Removes all listeners.
    pub fn clear_listeners(&mut self) {
        self.solver_listeners.clear();
        self.phase_listeners.clear();
        self.step_listeners.clear();
    }

    // === Event Firing ===

    /// Fires the best solution changed event.
    pub fn fire_best_solution_changed(&self, solution: &S, score: &S::Score) {
        for listener in &self.solver_listeners {
            listener.on_best_solution_changed(solution, score);
        }
    }

    /// Fires the solving started event.
    pub fn fire_solving_started(&self, solution: &S) {
        for listener in &self.solver_listeners {
            listener.on_solving_started(solution);
        }
    }

    /// Fires the solving ended event.
    pub fn fire_solving_ended(&self, solution: &S, is_terminated_early: bool) {
        for listener in &self.solver_listeners {
            listener.on_solving_ended(solution, is_terminated_early);
        }
    }

    /// Fires the phase started event.
    pub fn fire_phase_started(&self, phase_index: usize, phase_type: &str) {
        for listener in &self.phase_listeners {
            listener.on_phase_started(phase_index, phase_type);
        }
    }

    /// Fires the phase ended event.
    pub fn fire_phase_ended(&self, phase_index: usize, phase_type: &str) {
        for listener in &self.phase_listeners {
            listener.on_phase_ended(phase_index, phase_type);
        }
    }

    /// Fires the step started event.
    pub fn fire_step_started(&self, step_index: u64) {
        for listener in &self.step_listeners {
            listener.on_step_started(step_index);
        }
    }

    /// Fires the step ended event.
    pub fn fire_step_ended(&self, step_index: u64, score: &S::Score) {
        for listener in &self.step_listeners {
            listener.on_step_ended(step_index, score);
        }
    }

    // === Query Methods ===

    /// Returns the number of solver listeners.
    pub fn solver_listener_count(&self) -> usize {
        self.solver_listeners.len()
    }

    /// Returns the number of phase listeners.
    pub fn phase_listener_count(&self) -> usize {
        self.phase_listeners.len()
    }

    /// Returns the number of step listeners.
    pub fn step_listener_count(&self) -> usize {
        self.step_listeners.len()
    }

    /// Returns true if there are any listeners registered.
    pub fn has_listeners(&self) -> bool {
        !self.solver_listeners.is_empty()
            || !self.phase_listeners.is_empty()
            || !self.step_listeners.is_empty()
    }
}

impl<S: PlanningSolution> Default for SolverEventSupport<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: PlanningSolution> Debug for SolverEventSupport<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SolverEventSupport")
            .field("solver_listeners", &self.solver_listeners.len())
            .field("phase_listeners", &self.phase_listeners.len())
            .field("step_listeners", &self.step_listeners.len())
            .finish()
    }
}

/// A logging listener that prints events to stdout.
///
/// Useful for debugging and understanding solver behavior.
#[derive(Debug, Clone, Default)]
pub struct LoggingEventListener {
    /// Prefix for log messages.
    prefix: String,
}

impl LoggingEventListener {
    /// Creates a new logging listener.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a logging listener with a custom prefix.
    pub fn with_prefix(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
        }
    }
}

impl<S: PlanningSolution> SolverEventListener<S> for LoggingEventListener {
    fn on_best_solution_changed(&self, _solution: &S, score: &S::Score) {
        println!("{}[Event] New best solution found with score: {:?}", self.prefix, score);
    }

    fn on_solving_started(&self, _solution: &S) {
        println!("{}[Event] Solving started", self.prefix);
    }

    fn on_solving_ended(&self, _solution: &S, is_terminated_early: bool) {
        if is_terminated_early {
            println!("{}[Event] Solving ended (terminated early)", self.prefix);
        } else {
            println!("{}[Event] Solving ended", self.prefix);
        }
    }
}

impl<S: PlanningSolution> PhaseLifecycleListener<S> for LoggingEventListener {
    fn on_phase_started(&self, phase_index: usize, phase_type: &str) {
        println!(
            "{}[Event] Phase {} ({}) started",
            self.prefix, phase_index, phase_type
        );
    }

    fn on_phase_ended(&self, phase_index: usize, phase_type: &str) {
        println!(
            "{}[Event] Phase {} ({}) ended",
            self.prefix, phase_index, phase_type
        );
    }
}

impl<S: PlanningSolution> StepLifecycleListener<S> for LoggingEventListener {
    fn on_step_started(&self, step_index: u64) {
        println!("{}[Event] Step {} started", self.prefix, step_index);
    }

    fn on_step_ended(&self, step_index: u64, score: &S::Score) {
        println!(
            "{}[Event] Step {} ended with score: {:?}",
            self.prefix, step_index, score
        );
    }
}

/// A counting listener that tracks event occurrences.
///
/// Useful for testing and statistics collection.
#[derive(Debug, Default)]
pub struct CountingEventListener {
    best_solution_count: std::sync::atomic::AtomicUsize,
    solving_started_count: std::sync::atomic::AtomicUsize,
    solving_ended_count: std::sync::atomic::AtomicUsize,
    phase_started_count: std::sync::atomic::AtomicUsize,
    phase_ended_count: std::sync::atomic::AtomicUsize,
    step_started_count: std::sync::atomic::AtomicUsize,
    step_ended_count: std::sync::atomic::AtomicUsize,
}

impl CountingEventListener {
    /// Creates a new counting listener.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the number of best solution changed events.
    pub fn best_solution_count(&self) -> usize {
        self.best_solution_count
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Returns the number of solving started events.
    pub fn solving_started_count(&self) -> usize {
        self.solving_started_count
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Returns the number of solving ended events.
    pub fn solving_ended_count(&self) -> usize {
        self.solving_ended_count
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Returns the number of phase started events.
    pub fn phase_started_count(&self) -> usize {
        self.phase_started_count
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Returns the number of phase ended events.
    pub fn phase_ended_count(&self) -> usize {
        self.phase_ended_count
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Returns the number of step started events.
    pub fn step_started_count(&self) -> usize {
        self.step_started_count
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Returns the number of step ended events.
    pub fn step_ended_count(&self) -> usize {
        self.step_ended_count
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Resets all counters to zero.
    pub fn reset(&self) {
        self.best_solution_count
            .store(0, std::sync::atomic::Ordering::SeqCst);
        self.solving_started_count
            .store(0, std::sync::atomic::Ordering::SeqCst);
        self.solving_ended_count
            .store(0, std::sync::atomic::Ordering::SeqCst);
        self.phase_started_count
            .store(0, std::sync::atomic::Ordering::SeqCst);
        self.phase_ended_count
            .store(0, std::sync::atomic::Ordering::SeqCst);
        self.step_started_count
            .store(0, std::sync::atomic::Ordering::SeqCst);
        self.step_ended_count
            .store(0, std::sync::atomic::Ordering::SeqCst);
    }
}

impl<S: PlanningSolution> SolverEventListener<S> for CountingEventListener {
    fn on_best_solution_changed(&self, _solution: &S, _score: &S::Score) {
        self.best_solution_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    fn on_solving_started(&self, _solution: &S) {
        self.solving_started_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    fn on_solving_ended(&self, _solution: &S, _is_terminated_early: bool) {
        self.solving_ended_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

impl<S: PlanningSolution> PhaseLifecycleListener<S> for CountingEventListener {
    fn on_phase_started(&self, _phase_index: usize, _phase_type: &str) {
        self.phase_started_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    fn on_phase_ended(&self, _phase_index: usize, _phase_type: &str) {
        self.phase_ended_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

impl<S: PlanningSolution> StepLifecycleListener<S> for CountingEventListener {
    fn on_step_started(&self, _step_index: u64) {
        self.step_started_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    fn on_step_ended(&self, _step_index: u64, _score: &S::Score) {
        self.step_ended_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

#[cfg(test)]
#[path = "event_tests.rs"]
mod tests;
