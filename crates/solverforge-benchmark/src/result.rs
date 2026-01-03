//! Benchmark result types.

use std::time::Duration;

use solverforge_core::score::Score;
use solverforge_solver::statistics::{ScoreImprovement, SolverStatistics};

/// Result of a single benchmark run.
///
/// Contains timing, score, and statistics for one solver execution.
#[derive(Debug, Clone)]
pub struct BenchmarkRun<Sc: Score> {
    /// Run index (0-based).
    pub run_index: usize,
    /// Total solve time.
    pub solve_time: Duration,
    /// Final score achieved.
    pub final_score: Sc,
    /// Score progression over time.
    pub score_history: Vec<ScoreImprovement<Sc>>,
    /// Total moves evaluated.
    pub moves_evaluated: u64,
    /// Total moves accepted.
    pub moves_accepted: u64,
    /// Number of score calculations.
    pub score_calculations: u64,
}

impl<Sc: Score> BenchmarkRun<Sc> {
    /// Creates a benchmark run from solver statistics and final score.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_benchmark::BenchmarkRun;
    /// use solverforge_solver::statistics::SolverStatistics;
    /// use solverforge_core::score::SimpleScore;
    ///
    /// let stats = SolverStatistics::<SimpleScore>::new();
    /// let run = BenchmarkRun::from_statistics(0, stats, SimpleScore::of(0));
    /// assert_eq!(run.run_index, 0);
    /// ```
    pub fn from_statistics(run_index: usize, stats: SolverStatistics<Sc>, final_score: Sc) -> Self {
        Self {
            run_index,
            solve_time: stats.total_duration,
            final_score,
            score_history: stats.score_history,
            moves_evaluated: stats.total_moves_evaluated,
            moves_accepted: stats.total_moves_accepted,
            score_calculations: stats.score_calculation_count,
        }
    }

    /// Returns moves per second.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_benchmark::BenchmarkRun;
    /// use solverforge_core::score::SimpleScore;
    /// use std::time::Duration;
    ///
    /// let mut run = BenchmarkRun {
    ///     run_index: 0,
    ///     solve_time: Duration::from_secs(2),
    ///     final_score: SimpleScore::of(0),
    ///     score_history: vec![],
    ///     moves_evaluated: 1000,
    ///     moves_accepted: 500,
    ///     score_calculations: 1000,
    /// };
    ///
    /// assert!((run.moves_per_second() - 500.0).abs() < 0.001);
    /// ```
    pub fn moves_per_second(&self) -> f64 {
        if self.solve_time.is_zero() {
            0.0
        } else {
            self.moves_evaluated as f64 / self.solve_time.as_secs_f64()
        }
    }

    /// Returns acceptance rate (accepted / evaluated).
    pub fn acceptance_rate(&self) -> f64 {
        if self.moves_evaluated == 0 {
            0.0
        } else {
            self.moves_accepted as f64 / self.moves_evaluated as f64
        }
    }
}

/// Aggregated results from multiple benchmark runs.
///
/// Contains individual runs and computed statistics.
#[derive(Debug, Clone)]
pub struct BenchmarkResult<Sc: Score> {
    /// Benchmark name.
    pub name: String,
    /// Solver configuration name.
    pub solver_name: String,
    /// Problem instance name.
    pub problem_name: String,
    /// Individual runs.
    pub runs: Vec<BenchmarkRun<Sc>>,
}

impl<Sc: Score> BenchmarkResult<Sc> {
    /// Creates a new benchmark result.
    pub fn new(
        name: impl Into<String>,
        solver_name: impl Into<String>,
        problem_name: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            solver_name: solver_name.into(),
            problem_name: problem_name.into(),
            runs: Vec::new(),
        }
    }

    /// Adds a run to the results.
    pub fn add_run(&mut self, run: BenchmarkRun<Sc>) {
        self.runs.push(run);
    }

    /// Returns the number of runs.
    pub fn run_count(&self) -> usize {
        self.runs.len()
    }

    /// Returns the best score across all runs.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_benchmark::{BenchmarkResult, BenchmarkRun};
    /// use solverforge_core::score::SimpleScore;
    /// use std::time::Duration;
    ///
    /// let mut result = BenchmarkResult::<SimpleScore>::new("Test", "HC", "Problem1");
    /// result.add_run(BenchmarkRun {
    ///     run_index: 0,
    ///     solve_time: Duration::from_millis(100),
    ///     final_score: SimpleScore::of(-5),
    ///     score_history: vec![],
    ///     moves_evaluated: 100,
    ///     moves_accepted: 50,
    ///     score_calculations: 100,
    /// });
    /// result.add_run(BenchmarkRun {
    ///     run_index: 1,
    ///     solve_time: Duration::from_millis(100),
    ///     final_score: SimpleScore::of(0),
    ///     score_history: vec![],
    ///     moves_evaluated: 100,
    ///     moves_accepted: 50,
    ///     score_calculations: 100,
    /// });
    ///
    /// assert_eq!(*result.best_score().unwrap(), SimpleScore::of(0));
    /// ```
    pub fn best_score(&self) -> Option<&Sc> {
        self.runs.iter().map(|r| &r.final_score).max()
    }

    /// Returns the worst score across all runs.
    pub fn worst_score(&self) -> Option<&Sc> {
        self.runs.iter().map(|r| &r.final_score).min()
    }

    /// Returns the average solve time.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_benchmark::{BenchmarkResult, BenchmarkRun};
    /// use solverforge_core::score::SimpleScore;
    /// use std::time::Duration;
    ///
    /// let mut result = BenchmarkResult::<SimpleScore>::new("Test", "HC", "Problem1");
    /// result.add_run(BenchmarkRun {
    ///     run_index: 0,
    ///     solve_time: Duration::from_millis(100),
    ///     final_score: SimpleScore::of(0),
    ///     score_history: vec![],
    ///     moves_evaluated: 100,
    ///     moves_accepted: 50,
    ///     score_calculations: 100,
    /// });
    /// result.add_run(BenchmarkRun {
    ///     run_index: 1,
    ///     solve_time: Duration::from_millis(200),
    ///     final_score: SimpleScore::of(0),
    ///     score_history: vec![],
    ///     moves_evaluated: 100,
    ///     moves_accepted: 50,
    ///     score_calculations: 100,
    /// });
    ///
    /// assert_eq!(result.avg_solve_time(), Duration::from_millis(150));
    /// ```
    pub fn avg_solve_time(&self) -> Duration {
        if self.runs.is_empty() {
            return Duration::ZERO;
        }
        let total: Duration = self.runs.iter().map(|r| r.solve_time).sum();
        total / self.runs.len() as u32
    }

    /// Returns the minimum solve time.
    pub fn min_solve_time(&self) -> Duration {
        self.runs
            .iter()
            .map(|r| r.solve_time)
            .min()
            .unwrap_or(Duration::ZERO)
    }

    /// Returns the maximum solve time.
    pub fn max_solve_time(&self) -> Duration {
        self.runs
            .iter()
            .map(|r| r.solve_time)
            .max()
            .unwrap_or(Duration::ZERO)
    }

    /// Returns the average moves per second.
    pub fn avg_moves_per_second(&self) -> f64 {
        if self.runs.is_empty() {
            return 0.0;
        }
        let total: f64 = self.runs.iter().map(|r| r.moves_per_second()).sum();
        total / self.runs.len() as f64
    }

    /// Returns the average acceptance rate.
    pub fn avg_acceptance_rate(&self) -> f64 {
        if self.runs.is_empty() {
            return 0.0;
        }
        let total: f64 = self.runs.iter().map(|r| r.acceptance_rate()).sum();
        total / self.runs.len() as f64
    }
}
