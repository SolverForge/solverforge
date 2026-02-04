//! Async solver manager for dynamic solutions.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use solverforge_core::score::HardSoftScore;

use crate::constraint_set::DynamicConstraintSet;
use crate::solution::DynamicSolution;
use crate::solve::{solve_with_controls, SolveConfig, SolveResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolveStatus {
    NotStarted,
    Solving,
    Terminated,
}

/// Async solver manager that runs solve in a background thread.
pub struct DynamicSolverManager {
    terminate_flag: Arc<AtomicBool>,
    status: Arc<Mutex<SolveStatus>>,
    best_solution: Arc<Mutex<Option<DynamicSolution>>>,
    best_score: Arc<Mutex<Option<HardSoftScore>>>,
    result: Arc<Mutex<Option<SolveResult>>>,
    handle: Option<JoinHandle<()>>,
}

impl DynamicSolverManager {
    pub fn new() -> Self {
        Self {
            terminate_flag: Arc::new(AtomicBool::new(false)),
            status: Arc::new(Mutex::new(SolveStatus::NotStarted)),
            best_solution: Arc::new(Mutex::new(None)),
            best_score: Arc::new(Mutex::new(None)),
            result: Arc::new(Mutex::new(None)),
            handle: None,
        }
    }

    pub fn solve_async(
        &mut self,
        solution: DynamicSolution,
        constraints: DynamicConstraintSet,
        config: SolveConfig,
    ) {
        // Reset state
        self.terminate_flag.store(false, Ordering::SeqCst);
        *self.status.lock().unwrap() = SolveStatus::Solving;
        *self.best_solution.lock().unwrap() = None;
        *self.best_score.lock().unwrap() = None;
        *self.result.lock().unwrap() = None;

        let status = Arc::clone(&self.status);
        let best_solution = Arc::clone(&self.best_solution);
        let terminate_flag = Arc::clone(&self.terminate_flag);
        let result_holder = Arc::clone(&self.result);

        let handle = thread::spawn(move || {
            let result = solve_with_controls(
                solution,
                constraints,
                config,
                &terminate_flag, // Deref Arc to get &AtomicBool
                &best_solution,  // Solver writes here, Python polls it
            );

            // Write final solution to best_solution snapshot so get_best_solution() returns it
            *best_solution.lock().unwrap() = Some(result.solution.clone());
            *result_holder.lock().unwrap() = Some(result);
            *status.lock().unwrap() = SolveStatus::Terminated;
        });

        self.handle = Some(handle);
    }

    pub fn status(&self) -> SolveStatus {
        *self.status.lock().unwrap()
    }

    pub fn get_best_solution(&self) -> Option<DynamicSolution> {
        self.best_solution.lock().unwrap().clone()
    }

    pub fn get_result(&self) -> Option<SolveResult> {
        self.result.lock().unwrap().clone()
    }

    pub fn terminate(&mut self) {
        self.terminate_flag.store(true, Ordering::SeqCst);
        // Wait for thread to finish
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        *self.status.lock().unwrap() = SolveStatus::Terminated;
    }

    pub fn is_terminating(&self) -> bool {
        self.terminate_flag.load(Ordering::SeqCst)
    }
}

impl Default for DynamicSolverManager {
    fn default() -> Self {
        Self::new()
    }
}
