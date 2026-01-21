//! Benchmark result types.

use std::time::Duration;

use solverforge_core::score::Score;

/// Result of a single benchmark run.
///
/// Contains timing, score, and statistics for one solver execution.
///
/// # Example
///
/// ```
/// use solverforge_benchmark::BenchmarkRun;
/// use solverforge_core::score::SimpleScore;
/// use std::time::Duration;
///
/// let run = BenchmarkRun::new(
///     0,
///     Duration::from_secs(1),
///     SimpleScore::of(0),
///     1000,
///     500,
///     1000,
/// );
/// assert_eq!(run.run_index, 0);
/// assert!((run.moves_per_second() - 1000.0).abs() < 0.001);
/// ```
#[derive(Debug, Clone)]
pub struct BenchmarkRun<Sc: Score> {
    /// Run index (0-based).
    pub run_index: usize,
    /// Total solve time.
    pub solve_time: Duration,
    /// Final score achieved.
    pub final_score: Sc,
    /// Total moves evaluated.
    pub moves_evaluated: u64,
    /// Total moves accepted.
    pub moves_accepted: u64,
    /// Number of score calculations.
    pub score_calculations: u64,
}

impl<Sc: Score> BenchmarkRun<Sc> {
    /// Creates a new benchmark run.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_benchmark::BenchmarkRun;
    /// use solverforge_core::score::SimpleScore;
    /// use std::time::Duration;
    ///
    /// let run = BenchmarkRun::new(
    ///     0,
    ///     Duration::from_millis(500),
    ///     SimpleScore::of(-10),
    ///     2000,
    ///     800,
    ///     2000,
    /// );
    /// assert_eq!(run.run_index, 0);
    /// assert_eq!(run.moves_evaluated, 2000);
    /// assert_eq!(run.moves_accepted, 800);
    /// assert_eq!(run.score_calculations, 2000);
    /// ```
    pub fn new(
        run_index: usize,
        solve_time: Duration,
        final_score: Sc,
        moves_evaluated: u64,
        moves_accepted: u64,
        score_calculations: u64,
    ) -> Self {
        Self {
            run_index,
            solve_time,
            final_score,
            moves_evaluated,
            moves_accepted,
            score_calculations,
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
    /// let run = BenchmarkRun::new(
    ///     0,
    ///     Duration::from_secs(2),
    ///     SimpleScore::of(0),
    ///     1000,
    ///     500,
    ///     1000,
    /// );
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
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_benchmark::BenchmarkRun;
    /// use solverforge_core::score::SimpleScore;
    /// use std::time::Duration;
    ///
    /// let run = BenchmarkRun::new(
    ///     0,
    ///     Duration::from_secs(1),
    ///     SimpleScore::of(0),
    ///     1000,
    ///     250,
    ///     1000,
    /// );
    /// assert!((run.acceptance_rate() - 0.25).abs() < 0.001);
    /// ```
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
///
/// # Example
///
/// ```
/// use solverforge_benchmark::{BenchmarkResult, BenchmarkRun};
/// use solverforge_core::score::SimpleScore;
/// use std::time::Duration;
///
/// let mut result = BenchmarkResult::<SimpleScore>::new("Bench", "HC", "NQueens");
/// result.add_run(BenchmarkRun::new(0, Duration::from_millis(100), SimpleScore::of(-2), 500, 200, 500));
/// result.add_run(BenchmarkRun::new(1, Duration::from_millis(200), SimpleScore::of(0), 1000, 400, 1000));
///
/// assert_eq!(result.run_count(), 2);
/// assert_eq!(*result.best_score().unwrap(), SimpleScore::of(0));
/// assert_eq!(result.avg_solve_time(), Duration::from_millis(150));
/// ```
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
    /// result.add_run(BenchmarkRun::new(0, Duration::from_millis(100), SimpleScore::of(-5), 100, 50, 100));
    /// result.add_run(BenchmarkRun::new(1, Duration::from_millis(100), SimpleScore::of(0), 100, 50, 100));
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
    /// result.add_run(BenchmarkRun::new(0, Duration::from_millis(100), SimpleScore::of(0), 100, 50, 100));
    /// result.add_run(BenchmarkRun::new(1, Duration::from_millis(200), SimpleScore::of(0), 100, 50, 100));
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
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_benchmark::{BenchmarkResult, BenchmarkRun};
    /// use solverforge_core::score::SimpleScore;
    /// use std::time::Duration;
    ///
    /// let mut result = BenchmarkResult::<SimpleScore>::new("Test", "HC", "Problem1");
    /// result.add_run(BenchmarkRun::new(0, Duration::from_secs(1), SimpleScore::of(0), 1000, 500, 1000));
    /// result.add_run(BenchmarkRun::new(1, Duration::from_secs(1), SimpleScore::of(0), 2000, 800, 2000));
    ///
    /// assert!((result.avg_moves_per_second() - 1500.0).abs() < 0.001);
    /// ```
    pub fn avg_moves_per_second(&self) -> f64 {
        if self.runs.is_empty() {
            return 0.0;
        }
        let total: f64 = self.runs.iter().map(|r| r.moves_per_second()).sum();
        total / self.runs.len() as f64
    }

    /// Returns the average acceptance rate.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_benchmark::{BenchmarkResult, BenchmarkRun};
    /// use solverforge_core::score::SimpleScore;
    /// use std::time::Duration;
    ///
    /// let mut result = BenchmarkResult::<SimpleScore>::new("Test", "HC", "Problem1");
    /// result.add_run(BenchmarkRun::new(0, Duration::from_secs(1), SimpleScore::of(0), 1000, 500, 1000));
    /// result.add_run(BenchmarkRun::new(1, Duration::from_secs(1), SimpleScore::of(0), 1000, 300, 1000));
    ///
    /// assert!((result.avg_acceptance_rate() - 0.4).abs() < 0.001);
    /// ```
    pub fn avg_acceptance_rate(&self) -> f64 {
        if self.runs.is_empty() {
            return 0.0;
        }
        let total: f64 = self.runs.iter().map(|r| r.acceptance_rate()).sum();
        total / self.runs.len() as f64
    }
}
