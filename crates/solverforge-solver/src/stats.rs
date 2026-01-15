//! Solver statistics (zero-erasure).
//!
//! Stack-allocated statistics for solver and phase performance tracking.

use std::time::{Duration, Instant};

/// Solver-level statistics.
///
/// Tracks aggregate metrics across all phases of a solve run.
///
/// # Example
///
/// ```
/// use solverforge_solver::stats::SolverStats;
///
/// let mut stats = SolverStats::default();
/// stats.start();
/// stats.record_step();
/// stats.record_move(true);
/// stats.record_move(false);
///
/// assert_eq!(stats.step_count, 1);
/// assert_eq!(stats.moves_evaluated, 2);
/// assert_eq!(stats.moves_accepted, 1);
/// ```
#[derive(Debug, Default)]
pub struct SolverStats {
    start_time: Option<Instant>,
    /// Total steps taken across all phases.
    pub step_count: u64,
    /// Total moves evaluated across all phases.
    pub moves_evaluated: u64,
    /// Total moves accepted across all phases.
    pub moves_accepted: u64,
    /// Total score calculations performed.
    pub score_calculations: u64,
}

impl SolverStats {
    /// Marks the start of solving.
    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
    }

    /// Returns the elapsed time since solving started.
    pub fn elapsed(&self) -> Duration {
        self.start_time.map(|t| t.elapsed()).unwrap_or_default()
    }

    /// Records a move evaluation and whether it was accepted.
    pub fn record_move(&mut self, accepted: bool) {
        self.moves_evaluated += 1;
        if accepted {
            self.moves_accepted += 1;
        }
    }

    /// Records a step completion.
    pub fn record_step(&mut self) {
        self.step_count += 1;
    }

    /// Records a score calculation.
    pub fn record_score_calculation(&mut self) {
        self.score_calculations += 1;
    }

    /// Returns the moves per second rate.
    pub fn moves_per_second(&self) -> f64 {
        let secs = self.elapsed().as_secs_f64();
        if secs > 0.0 {
            self.moves_evaluated as f64 / secs
        } else {
            0.0
        }
    }

    /// Returns the acceptance rate (accepted / evaluated).
    pub fn acceptance_rate(&self) -> f64 {
        if self.moves_evaluated == 0 {
            0.0
        } else {
            self.moves_accepted as f64 / self.moves_evaluated as f64
        }
    }
}

/// Phase-level statistics.
///
/// Tracks metrics for a single solver phase.
///
/// # Example
///
/// ```
/// use solverforge_solver::stats::PhaseStats;
///
/// let mut stats = PhaseStats::new(0, "LocalSearch");
/// stats.record_step();
/// stats.record_move(true);
///
/// assert_eq!(stats.phase_index, 0);
/// assert_eq!(stats.phase_type, "LocalSearch");
/// assert_eq!(stats.step_count, 1);
/// assert_eq!(stats.moves_accepted, 1);
/// ```
#[derive(Debug)]
pub struct PhaseStats {
    /// Index of this phase (0-based).
    pub phase_index: usize,
    /// Type name of the phase.
    pub phase_type: &'static str,
    start_time: Instant,
    /// Number of steps taken in this phase.
    pub step_count: u64,
    /// Number of moves evaluated in this phase.
    pub moves_evaluated: u64,
    /// Number of moves accepted in this phase.
    pub moves_accepted: u64,
}

impl PhaseStats {
    /// Creates new phase statistics.
    pub fn new(phase_index: usize, phase_type: &'static str) -> Self {
        Self {
            phase_index,
            phase_type,
            start_time: Instant::now(),
            step_count: 0,
            moves_evaluated: 0,
            moves_accepted: 0,
        }
    }

    /// Returns the elapsed time for this phase.
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Returns the elapsed time in milliseconds.
    pub fn elapsed_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }

    /// Records a step completion.
    pub fn record_step(&mut self) {
        self.step_count += 1;
    }

    /// Records a move evaluation and whether it was accepted.
    pub fn record_move(&mut self, accepted: bool) {
        self.moves_evaluated += 1;
        if accepted {
            self.moves_accepted += 1;
        }
    }

    /// Returns the moves per second rate.
    pub fn moves_per_second(&self) -> u64 {
        let secs = self.elapsed().as_secs_f64();
        if secs > 0.0 {
            (self.moves_evaluated as f64 / secs) as u64
        } else {
            0
        }
    }

    /// Returns the acceptance rate (accepted / evaluated).
    pub fn acceptance_rate(&self) -> f64 {
        if self.moves_evaluated == 0 {
            0.0
        } else {
            self.moves_accepted as f64 / self.moves_evaluated as f64
        }
    }
}
