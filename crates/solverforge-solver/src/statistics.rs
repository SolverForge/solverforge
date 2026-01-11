//! Solver statistics collection and reporting.
//!
//! This module provides types for tracking solver performance metrics during
//! solving, including move counts, step counts, timing, and score progression.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use solverforge_core::score::Score;

/// Statistics for a single solver phase.
#[derive(Debug, Clone)]
pub struct PhaseStatistics<Sc: Score> {
    /// Index of this phase (0-based).
    pub phase_index: usize,
    /// Type name of the phase (e.g., "ConstructionHeuristic", "LocalSearch").
    pub phase_type: String,
    /// Time spent in this phase.
    pub duration: Duration,
    /// Number of steps taken in this phase.
    pub step_count: u64,
    /// Number of moves evaluated.
    pub moves_evaluated: u64,
    /// Number of moves accepted.
    pub moves_accepted: u64,
    /// Score at the start of the phase.
    pub starting_score: Option<Sc>,
    /// Score at the end of the phase.
    pub ending_score: Option<Sc>,
}

impl<Sc: Score> PhaseStatistics<Sc> {
    /// Creates empty phase statistics.
    pub fn new(phase_index: usize, phase_type: impl Into<String>) -> Self {
        Self {
            phase_index,
            phase_type: phase_type.into(),
            duration: Duration::ZERO,
            step_count: 0,
            moves_evaluated: 0,
            moves_accepted: 0,
            starting_score: None,
            ending_score: None,
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

    /// Returns the average time per step.
    pub fn avg_time_per_step(&self) -> Duration {
        if self.step_count == 0 {
            Duration::ZERO
        } else {
            self.duration / self.step_count as u32
        }
    }
}

/// Record of a score improvement event.
#[derive(Debug, Clone)]
pub struct ScoreImprovement<Sc: Score> {
    /// Time since solving started when improvement occurred.
    pub time_offset: Duration,
    /// Step number when improvement occurred.
    pub step_count: u64,
    /// The new (improved) score.
    pub score: Sc,
}

/// Complete statistics for a solver run.
#[derive(Debug, Clone)]
pub struct SolverStatistics<Sc: Score> {
    /// Total time spent solving.
    pub total_duration: Duration,
    /// Total steps taken across all phases.
    pub total_step_count: u64,
    /// Total moves evaluated across all phases.
    pub total_moves_evaluated: u64,
    /// Total moves accepted across all phases.
    pub total_moves_accepted: u64,
    /// Number of score calculations performed.
    pub score_calculation_count: u64,
    /// Statistics for each phase.
    pub phase_statistics: Vec<PhaseStatistics<Sc>>,
    /// History of score improvements.
    pub score_history: Vec<ScoreImprovement<Sc>>,
}

impl<Sc: Score> SolverStatistics<Sc> {
    /// Creates empty solver statistics.
    pub fn new() -> Self {
        Self {
            total_duration: Duration::ZERO,
            total_step_count: 0,
            total_moves_evaluated: 0,
            total_moves_accepted: 0,
            score_calculation_count: 0,
            phase_statistics: Vec::new(),
            score_history: Vec::new(),
        }
    }

    /// Returns the overall acceptance rate.
    pub fn acceptance_rate(&self) -> f64 {
        if self.total_moves_evaluated == 0 {
            0.0
        } else {
            self.total_moves_accepted as f64 / self.total_moves_evaluated as f64
        }
    }

    /// Returns the number of phases.
    pub fn phase_count(&self) -> usize {
        self.phase_statistics.len()
    }

    /// Returns the best score achieved (last in history, or None).
    pub fn best_score(&self) -> Option<&Sc> {
        self.score_history.last().map(|s| &s.score)
    }

    /// Returns the number of score improvements recorded.
    pub fn improvement_count(&self) -> usize {
        self.score_history.len()
    }
}

impl<Sc: Score> Default for SolverStatistics<Sc> {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe collector for solver statistics.
///
/// Use this to record statistics during solving. After solving, call
/// `into_statistics()` to get the final `SolverStatistics`.
pub struct StatisticsCollector<Sc: Score> {
    /// When solving started.
    start_time: Instant,
    /// Atomic counter for moves evaluated.
    moves_evaluated: AtomicU64,
    /// Atomic counter for moves accepted.
    moves_accepted: AtomicU64,
    /// Atomic counter for steps taken.
    step_count: AtomicU64,
    /// Atomic counter for score calculations.
    score_calculations: AtomicU64,
    /// Phase statistics (protected by mutex for complex updates).
    phases: Mutex<Vec<PhaseStatistics<Sc>>>,
    /// Score improvement history (protected by mutex).
    score_history: Mutex<Vec<ScoreImprovement<Sc>>>,
}

impl<Sc: Score> StatisticsCollector<Sc> {
    /// Creates a new statistics collector.
    ///
    /// The start time is recorded when this is called.
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            moves_evaluated: AtomicU64::new(0),
            moves_accepted: AtomicU64::new(0),
            step_count: AtomicU64::new(0),
            score_calculations: AtomicU64::new(0),
            phases: Mutex::new(Vec::new()),
            score_history: Mutex::new(Vec::new()),
        }
    }

    /// Records a move evaluation.
    ///
    /// Call this each time a move is evaluated, regardless of whether
    /// it was accepted.
    pub fn record_move_evaluated(&self) {
        self.moves_evaluated.fetch_add(1, Ordering::Relaxed);
    }

    /// Records an accepted move.
    ///
    /// Call this when a move is accepted (in addition to `record_move_evaluated`).
    pub fn record_move_accepted(&self) {
        self.moves_accepted.fetch_add(1, Ordering::Relaxed);
    }

    /// Records both evaluation and acceptance for a move.
    ///
    /// Convenience method when you know the move was accepted.
    pub fn record_move(&self, accepted: bool) {
        self.record_move_evaluated();
        if accepted {
            self.record_move_accepted();
        }
    }

    /// Records a step completion.
    pub fn record_step(&self) {
        self.step_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Records a score calculation.
    pub fn record_score_calculation(&self) {
        self.score_calculations.fetch_add(1, Ordering::Relaxed);
    }

    /// Records a score improvement.
    ///
    /// Call this when a new best score is found.
    pub fn record_improvement(&self, score: Sc) {
        let time_offset = self.start_time.elapsed();
        let step_count = self.step_count.load(Ordering::Relaxed);

        let improvement = ScoreImprovement {
            time_offset,
            step_count,
            score,
        };

        if let Ok(mut history) = self.score_history.lock() {
            history.push(improvement);
        }
    }

    /// Starts a new phase and returns its index.
    ///
    /// Call this at the beginning of each phase.
    pub fn start_phase(&self, phase_type: impl Into<String>) -> usize {
        let mut phases = self.phases.lock().unwrap();
        let index = phases.len();
        phases.push(PhaseStatistics::new(index, phase_type));
        index
    }

    /// Ends the current phase with the given statistics.
    ///
    /// Call this at the end of each phase.
    #[allow(clippy::too_many_arguments)]
    pub fn end_phase(
        &self,
        phase_index: usize,
        duration: Duration,
        step_count: u64,
        moves_evaluated: u64,
        moves_accepted: u64,
        starting_score: Option<Sc>,
        ending_score: Option<Sc>,
    ) {
        if let Ok(mut phases) = self.phases.lock() {
            if let Some(phase) = phases.get_mut(phase_index) {
                phase.duration = duration;
                phase.step_count = step_count;
                phase.moves_evaluated = moves_evaluated;
                phase.moves_accepted = moves_accepted;
                phase.starting_score = starting_score;
                phase.ending_score = ending_score;
            }
        }
    }

    /// Returns the elapsed time since solving started.
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Returns the current step count.
    pub fn current_step_count(&self) -> u64 {
        self.step_count.load(Ordering::Relaxed)
    }

    /// Returns the current moves evaluated count.
    pub fn current_moves_evaluated(&self) -> u64 {
        self.moves_evaluated.load(Ordering::Relaxed)
    }

    /// Returns the current moves accepted count.
    pub fn current_moves_accepted(&self) -> u64 {
        self.moves_accepted.load(Ordering::Relaxed)
    }

    /// Returns the current score calculation count.
    pub fn current_score_calculations(&self) -> u64 {
        self.score_calculations.load(Ordering::Relaxed)
    }

    /// Converts this collector into final statistics.
    ///
    /// This consumes the collector and returns the complete statistics.
    pub fn into_statistics(self) -> SolverStatistics<Sc> {
        SolverStatistics {
            total_duration: self.start_time.elapsed(),
            total_step_count: self.step_count.load(Ordering::Relaxed),
            total_moves_evaluated: self.moves_evaluated.load(Ordering::Relaxed),
            total_moves_accepted: self.moves_accepted.load(Ordering::Relaxed),
            score_calculation_count: self.score_calculations.load(Ordering::Relaxed),
            phase_statistics: self.phases.into_inner().unwrap(),
            score_history: self.score_history.into_inner().unwrap(),
        }
    }

    /// Takes a snapshot of current statistics without consuming the collector.
    pub fn snapshot(&self) -> SolverStatistics<Sc> {
        SolverStatistics {
            total_duration: self.start_time.elapsed(),
            total_step_count: self.step_count.load(Ordering::Relaxed),
            total_moves_evaluated: self.moves_evaluated.load(Ordering::Relaxed),
            total_moves_accepted: self.moves_accepted.load(Ordering::Relaxed),
            score_calculation_count: self.score_calculations.load(Ordering::Relaxed),
            phase_statistics: self.phases.lock().unwrap().clone(),
            score_history: self.score_history.lock().unwrap().clone(),
        }
    }
}

impl<Sc: Score> Default for StatisticsCollector<Sc> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solverforge_core::score::SimpleScore;

    #[test]
    fn test_phase_statistics_new() {
        let stats: PhaseStatistics<SimpleScore> = PhaseStatistics::new(0, "ConstructionHeuristic");
        assert_eq!(stats.phase_index, 0);
        assert_eq!(stats.phase_type, "ConstructionHeuristic");
        assert_eq!(stats.step_count, 0);
    }

    #[test]
    fn test_phase_statistics_acceptance_rate() {
        let mut stats: PhaseStatistics<SimpleScore> = PhaseStatistics::new(0, "LocalSearch");
        stats.moves_evaluated = 100;
        stats.moves_accepted = 25;
        assert!((stats.acceptance_rate() - 0.25).abs() < f64::EPSILON);
    }

    #[test]
    fn test_phase_statistics_acceptance_rate_zero() {
        let stats: PhaseStatistics<SimpleScore> = PhaseStatistics::new(0, "Test");
        assert_eq!(stats.acceptance_rate(), 0.0);
    }

    #[test]
    fn test_solver_statistics_new() {
        let stats: SolverStatistics<SimpleScore> = SolverStatistics::new();
        assert_eq!(stats.total_step_count, 0);
        assert_eq!(stats.phase_count(), 0);
        assert!(stats.best_score().is_none());
    }

    #[test]
    fn test_collector_record_move() {
        let collector: StatisticsCollector<SimpleScore> = StatisticsCollector::new();

        collector.record_move(true);
        collector.record_move(false);
        collector.record_move(true);

        assert_eq!(collector.current_moves_evaluated(), 3);
        assert_eq!(collector.current_moves_accepted(), 2);
    }

    #[test]
    fn test_collector_record_step() {
        let collector: StatisticsCollector<SimpleScore> = StatisticsCollector::new();

        collector.record_step();
        collector.record_step();
        collector.record_step();

        assert_eq!(collector.current_step_count(), 3);
    }

    #[test]
    fn test_collector_record_improvement() {
        let collector: StatisticsCollector<SimpleScore> = StatisticsCollector::new();

        collector.record_improvement(SimpleScore::of(-10));
        collector.record_improvement(SimpleScore::of(-5));
        collector.record_improvement(SimpleScore::of(0));

        let stats = collector.into_statistics();
        assert_eq!(stats.improvement_count(), 3);
        assert_eq!(*stats.best_score().unwrap(), SimpleScore::of(0));
    }

    #[test]
    fn test_collector_phases() {
        let collector: StatisticsCollector<SimpleScore> = StatisticsCollector::new();

        let phase0 = collector.start_phase("Construction");
        let phase1 = collector.start_phase("LocalSearch");

        assert_eq!(phase0, 0);
        assert_eq!(phase1, 1);

        collector.end_phase(
            phase0,
            Duration::from_millis(100),
            5,
            10,
            5,
            None,
            Some(SimpleScore::of(-5)),
        );

        collector.end_phase(
            phase1,
            Duration::from_millis(200),
            20,
            100,
            50,
            Some(SimpleScore::of(-5)),
            Some(SimpleScore::of(0)),
        );

        let stats = collector.into_statistics();
        assert_eq!(stats.phase_count(), 2);

        let p0 = &stats.phase_statistics[0];
        assert_eq!(p0.phase_type, "Construction");
        assert_eq!(p0.step_count, 5);

        let p1 = &stats.phase_statistics[1];
        assert_eq!(p1.phase_type, "LocalSearch");
        assert_eq!(p1.moves_evaluated, 100);
        assert!((p1.acceptance_rate() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_collector_snapshot() {
        let collector: StatisticsCollector<SimpleScore> = StatisticsCollector::new();

        collector.record_step();
        collector.record_step();

        let snapshot = collector.snapshot();
        assert_eq!(snapshot.total_step_count, 2);

        // Can still use collector after snapshot
        collector.record_step();
        assert_eq!(collector.current_step_count(), 3);
    }

    #[test]
    fn test_collector_thread_safety() {
        let collector: StatisticsCollector<SimpleScore> = StatisticsCollector::new();

        rayon::scope(|s| {
            for _ in 0..4 {
                s.spawn(|_| {
                    for _ in 0..1000 {
                        collector.record_move_evaluated();
                        collector.record_step();
                    }
                });
            }
        });

        assert_eq!(collector.current_moves_evaluated(), 4000);
        assert_eq!(collector.current_step_count(), 4000);
    }

    #[test]
    fn test_solver_statistics_acceptance_rate() {
        let stats: SolverStatistics<SimpleScore> = SolverStatistics {
            total_duration: Duration::from_secs(1),
            total_step_count: 100,
            total_moves_evaluated: 1000,
            total_moves_accepted: 100,
            score_calculation_count: 1000,
            phase_statistics: Vec::new(),
            score_history: Vec::new(),
        };

        assert!((stats.acceptance_rate() - 0.1).abs() < f64::EPSILON);
    }
}
