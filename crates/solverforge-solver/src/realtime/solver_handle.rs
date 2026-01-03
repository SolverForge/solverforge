//! Solver handle for submitting problem changes during solving.

use std::fmt::Debug;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::Arc;

use solverforge_core::domain::PlanningSolution;

use super::problem_change::BoxedProblemChange;
use super::ProblemChange;

/// Result of a problem change submission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProblemChangeResult {
    /// Change was successfully queued.
    Queued,
    /// Solver is not running, change was not queued.
    SolverNotRunning,
    /// Change queue is full (solver is processing slowly).
    QueueFull,
}

/// Handle for interacting with a running solver.
///
/// The solver handle allows submitting problem changes to a solver
/// while it is running. Changes are queued and processed at step
/// boundaries.
///
/// # Example
///
/// ```
/// use solverforge_solver::realtime::{SolverHandle, ProblemChange, ProblemChangeResult};
/// use solverforge_scoring::ScoreDirector;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SimpleScore;
///
/// #[derive(Clone, Debug)]
/// struct Task { id: usize }
///
/// #[derive(Clone, Debug)]
/// struct Solution {
///     tasks: Vec<Task>,
///     score: Option<SimpleScore>,
/// }
///
/// impl PlanningSolution for Solution {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// #[derive(Debug)]
/// struct AddTask { id: usize }
///
/// impl ProblemChange<Solution> for AddTask {
///     fn apply(&self, sd: &mut dyn ScoreDirector<Solution>) {
///         sd.working_solution_mut().tasks.push(Task { id: self.id });
///         sd.trigger_variable_listeners();
///     }
/// }
///
/// // Create a handle (normally obtained from RealtimeSolver)
/// let (handle, _rx) = SolverHandle::<Solution>::new();
///
/// // Submit a change while solver is "running"
/// handle.set_solving(true);
/// let result = handle.add_problem_change(AddTask { id: 42 });
/// assert_eq!(result, ProblemChangeResult::Queued);
///
/// // When solver stops, changes are rejected
/// handle.set_solving(false);
/// let result = handle.add_problem_change(AddTask { id: 43 });
/// assert_eq!(result, ProblemChangeResult::SolverNotRunning);
/// ```
pub struct SolverHandle<S: PlanningSolution> {
    /// Channel for sending problem changes to the solver.
    change_tx: Sender<BoxedProblemChange<S>>,
    /// Flag indicating whether solver is currently running.
    solving: Arc<AtomicBool>,
    /// Flag to request early termination.
    terminate_early: Arc<AtomicBool>,
}

impl<S: PlanningSolution> SolverHandle<S> {
    /// Creates a new solver handle and its corresponding receiver.
    ///
    /// The receiver should be passed to the solver to receive changes.
    pub fn new() -> (Self, ProblemChangeReceiver<S>) {
        let (tx, rx) = mpsc::channel();
        let solving = Arc::new(AtomicBool::new(false));
        let terminate_early = Arc::new(AtomicBool::new(false));

        let handle = Self {
            change_tx: tx,
            solving: Arc::clone(&solving),
            terminate_early: Arc::clone(&terminate_early),
        };

        let receiver = ProblemChangeReceiver {
            change_rx: rx,
            solving,
            terminate_early,
        };

        (handle, receiver)
    }

    /// Submits a problem change to the solver.
    ///
    /// The change is queued and will be processed at the next step boundary.
    /// Returns the result of the submission.
    pub fn add_problem_change<P: ProblemChange<S> + 'static>(
        &self,
        change: P,
    ) -> ProblemChangeResult {
        if !self.solving.load(Ordering::SeqCst) {
            return ProblemChangeResult::SolverNotRunning;
        }

        match self.change_tx.send(Box::new(change)) {
            Ok(()) => ProblemChangeResult::Queued,
            Err(_) => ProblemChangeResult::QueueFull,
        }
    }

    /// Submits a boxed problem change to the solver.
    pub fn add_problem_change_boxed(
        &self,
        change: BoxedProblemChange<S>,
    ) -> ProblemChangeResult {
        if !self.solving.load(Ordering::SeqCst) {
            return ProblemChangeResult::SolverNotRunning;
        }

        match self.change_tx.send(change) {
            Ok(()) => ProblemChangeResult::Queued,
            Err(_) => ProblemChangeResult::QueueFull,
        }
    }

    /// Returns true if the solver is currently running.
    pub fn is_solving(&self) -> bool {
        self.solving.load(Ordering::SeqCst)
    }

    /// Requests early termination of the solver.
    ///
    /// The solver will stop at the next step boundary.
    pub fn terminate_early(&self) {
        self.terminate_early.store(true, Ordering::SeqCst);
    }

    /// Sets the solving flag (used internally by the solver).
    pub fn set_solving(&self, solving: bool) {
        self.solving.store(solving, Ordering::SeqCst);
    }
}

impl<S: PlanningSolution> Clone for SolverHandle<S> {
    fn clone(&self) -> Self {
        Self {
            change_tx: self.change_tx.clone(),
            solving: Arc::clone(&self.solving),
            terminate_early: Arc::clone(&self.terminate_early),
        }
    }
}

impl<S: PlanningSolution> Debug for SolverHandle<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SolverHandle")
            .field("solving", &self.solving.load(Ordering::SeqCst))
            .field("terminate_early", &self.terminate_early.load(Ordering::SeqCst))
            .finish()
    }
}

/// Receiver for problem changes, used by the solver.
pub struct ProblemChangeReceiver<S: PlanningSolution> {
    /// Channel for receiving problem changes.
    change_rx: Receiver<BoxedProblemChange<S>>,
    /// Shared solving flag.
    solving: Arc<AtomicBool>,
    /// Shared terminate early flag.
    terminate_early: Arc<AtomicBool>,
}

impl<S: PlanningSolution> ProblemChangeReceiver<S> {
    /// Tries to receive a pending problem change without blocking.
    ///
    /// Returns `Some(change)` if a change is available, `None` otherwise.
    pub fn try_recv(&self) -> Option<BoxedProblemChange<S>> {
        match self.change_rx.try_recv() {
            Ok(change) => Some(change),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => None,
        }
    }

    /// Receives all pending problem changes without blocking.
    ///
    /// Returns a vector of all queued changes.
    pub fn drain_pending(&self) -> Vec<BoxedProblemChange<S>> {
        let mut changes = Vec::new();
        while let Some(change) = self.try_recv() {
            changes.push(change);
        }
        changes
    }

    /// Returns true if early termination has been requested.
    pub fn is_terminate_early_requested(&self) -> bool {
        self.terminate_early.load(Ordering::SeqCst)
    }

    /// Sets the solving flag.
    pub fn set_solving(&self, solving: bool) {
        self.solving.store(solving, Ordering::SeqCst);
    }

    /// Clears the terminate early flag.
    pub fn clear_terminate_early(&self) {
        self.terminate_early.store(false, Ordering::SeqCst);
    }
}

impl<S: PlanningSolution> Debug for ProblemChangeReceiver<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProblemChangeReceiver")
            .field("solving", &self.solving.load(Ordering::SeqCst))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solverforge_core::score::SimpleScore;
    use solverforge_scoring::ScoreDirector;

    #[derive(Clone, Debug)]
    struct TestSolution {
        value: i32,
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for TestSolution {
        type Score = SimpleScore;
        fn score(&self) -> Option<Self::Score> {
            self.score
        }
        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    #[derive(Debug)]
    struct IncrementValue;

    impl ProblemChange<TestSolution> for IncrementValue {
        fn apply(&self, score_director: &mut dyn ScoreDirector<TestSolution>) {
            score_director.working_solution_mut().value += 1;
        }
    }

    #[test]
    fn handle_creation() {
        let (handle, _rx) = SolverHandle::<TestSolution>::new();
        assert!(!handle.is_solving());
    }

    #[test]
    fn submit_change_when_solving() {
        let (handle, rx) = SolverHandle::<TestSolution>::new();
        handle.set_solving(true);

        let result = handle.add_problem_change(IncrementValue);
        assert_eq!(result, ProblemChangeResult::Queued);

        // Verify change is received
        let changes = rx.drain_pending();
        assert_eq!(changes.len(), 1);
    }

    #[test]
    fn submit_change_when_not_solving() {
        let (handle, _rx) = SolverHandle::<TestSolution>::new();

        let result = handle.add_problem_change(IncrementValue);
        assert_eq!(result, ProblemChangeResult::SolverNotRunning);
    }

    #[test]
    fn multiple_changes() {
        let (handle, rx) = SolverHandle::<TestSolution>::new();
        handle.set_solving(true);

        handle.add_problem_change(IncrementValue);
        handle.add_problem_change(IncrementValue);
        handle.add_problem_change(IncrementValue);

        let changes = rx.drain_pending();
        assert_eq!(changes.len(), 3);
    }

    #[test]
    fn terminate_early() {
        let (handle, rx) = SolverHandle::<TestSolution>::new();

        assert!(!rx.is_terminate_early_requested());
        handle.terminate_early();
        assert!(rx.is_terminate_early_requested());

        rx.clear_terminate_early();
        assert!(!rx.is_terminate_early_requested());
    }

    #[test]
    fn handle_clone() {
        let (handle1, rx) = SolverHandle::<TestSolution>::new();
        let handle2 = handle1.clone();

        handle1.set_solving(true);
        assert!(handle2.is_solving());

        handle2.add_problem_change(IncrementValue);
        let changes = rx.drain_pending();
        assert_eq!(changes.len(), 1);
    }
}
