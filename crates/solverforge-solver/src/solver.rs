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
/// Generic over `D: ScoreDirector<S>` for zero type erasure.
pub struct Solver<S: PlanningSolution, D: ScoreDirector<S>> {
    phases: Vec<Box<dyn Phase<S, D>>>,
    termination: Option<Box<dyn Termination<S, D>>>,
    terminate_early_flag: Arc<AtomicBool>,
    solving: Arc<AtomicBool>,
}

impl<S: PlanningSolution, D: ScoreDirector<S>> Solver<S, D> {
    pub fn new(phases: Vec<Box<dyn Phase<S, D>>>) -> Self {
        Solver {
            phases,
            termination: None,
            terminate_early_flag: Arc::new(AtomicBool::new(false)),
            solving: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn with_phase(mut self, phase: Box<dyn Phase<S, D>>) -> Self {
        self.phases.push(phase);
        self
    }

    pub fn with_termination(mut self, termination: Box<dyn Termination<S, D>>) -> Self {
        self.termination = Some(termination);
        self
    }

    pub fn solve(mut self, score_director: D) -> S {
        self.solving.store(true, Ordering::SeqCst);
        self.terminate_early_flag.store(false, Ordering::SeqCst);

        let mut solver_scope = SolverScope::new(score_director);
        solver_scope.set_terminate_early_flag(self.terminate_early_flag.clone());
        solver_scope.start_solving();

        for phase in &mut self.phases {
            if self.terminate_early_flag.load(Ordering::SeqCst) {
                break;
            }
            if let Some(ref termination) = self.termination {
                if termination.is_terminated(&solver_scope) {
                    break;
                }
            }
            phase.solve(&mut solver_scope);
        }

        self.solving.store(false, Ordering::SeqCst);
        solver_scope.take_best_or_working_solution()
    }

    pub fn terminate_early(&self) -> bool {
        if self.solving.load(Ordering::SeqCst) {
            self.terminate_early_flag.store(true, Ordering::SeqCst);
            true
        } else {
            false
        }
    }

    pub fn is_solving(&self) -> bool {
        self.solving.load(Ordering::SeqCst)
    }
}
