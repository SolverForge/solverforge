//! Solver implementation.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::phase::Phase;
use crate::scope::SolverScope;
use crate::termination::Termination;

/// The main solver that optimizes planning solutions.
///
/// Generic over phase and termination types for zero type erasure.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `D` - The score director type
/// * `P` - The phase type
/// * `T` - The termination type
pub struct Solver<S, D, P, T>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
{
    phase: P,
    termination: Option<T>,
    terminate_early_flag: Arc<AtomicBool>,
    solving: Arc<AtomicBool>,
    _marker: std::marker::PhantomData<(S, D)>,
}

impl<S, D, P, T> Solver<S, D, P, T>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    P: Phase<S, D>,
    T: Termination<S, D>,
{
    /// Creates a new solver with a phase and optional termination.
    pub fn new(phase: P, termination: Option<T>) -> Self {
        Solver {
            phase,
            termination,
            terminate_early_flag: Arc::new(AtomicBool::new(false)),
            solving: Arc::new(AtomicBool::new(false)),
            _marker: std::marker::PhantomData,
        }
    }

    /// Solves the planning problem using the provided score director.
    pub fn solve(mut self, score_director: D) -> S {
        self.solving.store(true, Ordering::SeqCst);
        self.terminate_early_flag.store(false, Ordering::SeqCst);

        let mut solver_scope = SolverScope::new(score_director);
        solver_scope.set_terminate_early_flag(self.terminate_early_flag.clone());
        solver_scope.start_solving();

        let should_terminate = self.terminate_early_flag.load(Ordering::SeqCst)
            || self
                .termination
                .as_ref()
                .map_or(false, |t| t.is_terminated(&solver_scope));

        if !should_terminate {
            self.phase.solve(&mut solver_scope);
        }

        self.solving.store(false, Ordering::SeqCst);
        solver_scope.take_best_or_working_solution()
    }

    /// Requests early termination of the solver.
    pub fn terminate_early(&self) -> bool {
        if self.solving.load(Ordering::SeqCst) {
            self.terminate_early_flag.store(true, Ordering::SeqCst);
            true
        } else {
            false
        }
    }

    /// Returns true if the solver is currently solving.
    pub fn is_solving(&self) -> bool {
        self.solving.load(Ordering::SeqCst)
    }
}

impl<S, D, P> Solver<S, D, P, ()>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    P: Phase<S, D>,
{
    /// Creates a solver with only a phase (no termination).
    pub fn with_phase(phase: P) -> Self {
        Solver {
            phase,
            termination: None,
            terminate_early_flag: Arc::new(AtomicBool::new(false)),
            solving: Arc::new(AtomicBool::new(false)),
            _marker: std::marker::PhantomData,
        }
    }
}
