//! SolverManager for async job management.
//!
//! Provides the high-level API for:
//! - Starting solve jobs that stream solutions via tokio channels
//! - Tracking solver status per job
//! - Early termination of solving jobs
//!
//! # Zero-Erasure Design
//!
//! This implementation uses tokio channels for ownership transfer.
//! The solver sends owned solutions through the channel - no Clone required.
//! Fixed-size slot arrays avoid heap indirection.

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use solverforge_core::score::Score;
use tokio::sync::mpsc;

/// Maximum concurrent jobs per SolverManager instance.
pub const MAX_JOBS: usize = 16;

/// Slot states for job lifecycle.
const SLOT_FREE: u8 = 0;
const SLOT_SOLVING: u8 = 1;
const SLOT_DONE: u8 = 2;

/// Trait for solutions that can be solved with channel-based solution streaming.
///
/// This trait is implemented by the `#[planning_solution]` macro when
/// `constraints` is specified. The solver sends owned solutions through
/// the channel - no Clone required.
///
/// Solver progress is logged via `tracing` at INFO/DEBUG levels.
///
/// # Type Parameters
///
/// The solution must be `Send + 'static` to support async job execution.
/// Note: `Clone` is NOT required - ownership is transferred via channel.
pub trait Solvable: solverforge_core::domain::PlanningSolution + Send + 'static {
    /// Solves the solution, sending each new best through the channel.
    ///
    /// The final solution is sent through the channel before this returns.
    /// Ownership of solutions transfers through the channel.
    ///
    /// # Arguments
    ///
    /// * `terminate` - Optional flag to request early termination
    /// * `sender` - Channel to send each new best solution (ownership transferred)
    fn solve_with_listener(
        self,
        terminate: Option<&AtomicBool>,
        sender: mpsc::UnboundedSender<(Self, Self::Score)>,
    );
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

/// A single job slot for tracking solve state.
struct JobSlot {
    /// Current slot state (FREE, SOLVING, DONE).
    state: AtomicU8,
    /// Termination flag - solver checks this periodically.
    terminate: AtomicBool,
}

impl JobSlot {
    /// Creates an empty job slot.
    const fn new() -> Self {
        Self {
            state: AtomicU8::new(SLOT_FREE),
            terminate: AtomicBool::new(false),
        }
    }

    /// Resets the slot to free state.
    fn reset(&self) {
        self.terminate.store(false, Ordering::Release);
        self.state.store(SLOT_FREE, Ordering::Release);
    }
}

/// Manages async solve jobs with channel-based solution streaming.
///
/// Uses fixed-size slot array for job tracking. Solutions stream through
/// tokio channels - the solver sends owned solutions, users receive them
/// without cloning.
///
/// # Type Parameters
///
/// * `S` - Solution type that implements `Solvable`
///
/// # Thread Safety
///
/// `SolverManager` is thread-safe. Jobs can be started, queried, and terminated
/// from any thread.
pub struct SolverManager<S: Solvable> {
    slots: [JobSlot; MAX_JOBS],
    _phantom: std::marker::PhantomData<fn() -> S>,
}

impl<S: Solvable> Default for SolverManager<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Solvable> SolverManager<S>
where
    S::Score: Score,
{
    /// Creates a new SolverManager with empty slots.
    pub const fn new() -> Self {
        Self {
            slots: [
                JobSlot::new(),
                JobSlot::new(),
                JobSlot::new(),
                JobSlot::new(),
                JobSlot::new(),
                JobSlot::new(),
                JobSlot::new(),
                JobSlot::new(),
                JobSlot::new(),
                JobSlot::new(),
                JobSlot::new(),
                JobSlot::new(),
                JobSlot::new(),
                JobSlot::new(),
                JobSlot::new(),
                JobSlot::new(),
            ],
            _phantom: std::marker::PhantomData,
        }
    }

    /// Starts solving and returns a receiver for streaming solutions.
    ///
    /// The solver runs asynchronously via rayon. Solutions stream through
    /// the returned receiver as they're found. Each solution is owned -
    /// no cloning occurs.
    ///
    /// # Arguments
    ///
    /// * `solution` - The starting solution (ownership transferred)
    ///
    /// # Returns
    ///
    /// A tuple of (job_id, receiver). The receiver yields `(solution, score)`
    /// pairs as new best solutions are found.
    ///
    /// # Panics
    ///
    /// Panics if no free slots are available.
    pub fn solve(
        &'static self,
        solution: S,
    ) -> (usize, mpsc::UnboundedReceiver<(S, S::Score)>) {
        let (sender, receiver) = mpsc::unbounded_channel();

        // Find a free slot
        let slot_idx = self
            .slots
            .iter()
            .position(|s| {
                s.state
                    .compare_exchange(SLOT_FREE, SLOT_SOLVING, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
            })
            .expect("No free job slots available");

        let slot = &self.slots[slot_idx];
        slot.terminate.store(false, Ordering::SeqCst);

        // Spawn the solver via rayon
        rayon::spawn(move || {
            let terminate_ref = &slot.terminate;

            // solve_with_listener sends all solutions (including final) through the channel
            solution.solve_with_listener(Some(terminate_ref), sender);

            slot.state.store(SLOT_DONE, Ordering::Release);
        });

        (slot_idx, receiver)
    }

    /// Gets the solver status for a job.
    pub fn get_status(&self, job_id: usize) -> SolverStatus {
        if job_id >= MAX_JOBS {
            return SolverStatus::NotSolving;
        }
        match self.slots[job_id].state.load(Ordering::Acquire) {
            SLOT_SOLVING => SolverStatus::Solving,
            _ => SolverStatus::NotSolving,
        }
    }

    /// Requests early termination of a job.
    ///
    /// Returns `true` if the job was found and is currently solving.
    pub fn terminate_early(&self, job_id: usize) -> bool {
        if job_id >= MAX_JOBS {
            return false;
        }

        let slot = &self.slots[job_id];
        if slot.state.load(Ordering::Acquire) == SLOT_SOLVING {
            slot.terminate.store(true, Ordering::SeqCst);
            true
        } else {
            false
        }
    }

    /// Frees a job slot after solving completes.
    ///
    /// Call this after the receiver is drained to allow reuse of the slot.
    pub fn free_slot(&self, job_id: usize) {
        if job_id < MAX_JOBS {
            self.slots[job_id].reset();
        }
    }

    /// Returns the number of active (solving) jobs.
    pub fn active_job_count(&self) -> usize {
        self.slots
            .iter()
            .filter(|s| s.state.load(Ordering::Relaxed) == SLOT_SOLVING)
            .count()
    }
}
