//! Benchmark runner.

use std::marker::PhantomData;
use std::sync::Arc;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;
use solverforge_solver::solver::Solver;
use solverforge_solver::statistics::StatisticsCollector;

use crate::config::BenchmarkConfig;
use crate::result::{BenchmarkResult, BenchmarkRun};

/// Zero-erasure benchmark runner.
///
/// The benchmark runner executes a solver against a problem instance multiple times,
/// collecting statistics for each run. All factory functions are stored as concrete
/// generic type parameters to avoid virtual dispatch overhead.
///
/// # Type Parameters
///
/// * `S` - The planning solution type
/// * `Dir` - The score director type
/// * `P` - Problem factory: `Fn() -> S`
/// * `D` - Score director factory: `Fn(S) -> Dir`
/// * `F` - Solver factory: `Fn() -> Solver<S, Dir>`
pub struct Benchmark<S, Dir, P, D, F>
where
    S: PlanningSolution,
    Dir: ScoreDirector<S>,
    P: Fn() -> S,
    D: Fn(S) -> Dir,
    F: Fn() -> Solver<S, Dir>,
{
    config: BenchmarkConfig,
    solver_name: String,
    problem_name: String,
    problem_factory: P,
    director_factory: D,
    solver_factory: F,
    _phantom: PhantomData<(S, Dir)>,
}

impl<S, Dir, P, D, F> Benchmark<S, Dir, P, D, F>
where
    S: PlanningSolution,
    Dir: ScoreDirector<S>,
    P: Fn() -> S,
    D: Fn(S) -> Dir,
    F: Fn() -> Solver<S, Dir>,
{
    /// Creates a new benchmark.
    ///
    /// # Arguments
    ///
    /// * `config` - Benchmark configuration (warmup count, run count, etc.)
    /// * `solver_name` - Name identifying the solver configuration
    /// * `problem_name` - Name identifying the problem instance
    /// * `problem_factory` - Factory function creating fresh problem instances
    /// * `director_factory` - Factory function creating score directors
    /// * `solver_factory` - Factory function creating solvers
    pub fn new(
        config: BenchmarkConfig,
        solver_name: impl Into<String>,
        problem_name: impl Into<String>,
        problem_factory: P,
        director_factory: D,
        solver_factory: F,
    ) -> Self {
        Self {
            config,
            solver_name: solver_name.into(),
            problem_name: problem_name.into(),
            problem_factory,
            director_factory,
            solver_factory,
            _phantom: PhantomData,
        }
    }

    /// Runs the benchmark and returns aggregated results.
    ///
    /// Executes warmup runs first (not measured), then measurement runs.
    /// Statistics are collected for each measurement run.
    pub fn run(&self) -> BenchmarkResult<S::Score> {
        // Warmup runs
        for _ in 0..self.config.warmup_count() {
            self.run_once(None);
        }

        // Measurement runs
        let mut result =
            BenchmarkResult::new(self.config.name(), &self.solver_name, &self.problem_name);

        for run_index in 0..self.config.run_count() {
            let collector = Arc::new(StatisticsCollector::new());
            let (solution, stats) = self.run_once(Some(collector));
            let final_score = solution.score().unwrap_or_else(|| {
                // Calculate score if not set
                let working = solution;
                let mut temp_director = (self.director_factory)(working);
                temp_director.calculate_score()
            });

            let run = BenchmarkRun::from_statistics(run_index, stats, final_score);
            result.add_run(run);
        }

        result
    }

    /// Executes a single run.
    fn run_once(
        &self,
        collector: Option<Arc<StatisticsCollector<S::Score>>>,
    ) -> (
        S,
        solverforge_solver::statistics::SolverStatistics<S::Score>,
    ) {
        let problem = (self.problem_factory)();
        let director = (self.director_factory)(problem);
        let solver = (self.solver_factory)();

        // Run solver
        let result = solver.solve(director);

        // Get statistics
        let stats = collector
            .map(|c| {
                // Try to unwrap Arc; if others hold references, take snapshot
                match Arc::try_unwrap(c) {
                    Ok(c) => c.into_statistics(),
                    Err(arc) => arc.snapshot(),
                }
            })
            .unwrap_or_default();

        (result, stats)
    }
}

/// Builder for creating benchmarks with fluent API.
pub struct BenchmarkBuilder<S: PlanningSolution> {
    config: BenchmarkConfig,
    solver_name: String,
    problem_name: String,
    _phantom: PhantomData<S>,
}

impl<S: PlanningSolution> BenchmarkBuilder<S> {
    /// Creates a new benchmark builder.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            config: BenchmarkConfig::new(name),
            solver_name: "default".to_string(),
            problem_name: "default".to_string(),
            _phantom: PhantomData,
        }
    }

    /// Sets the solver name.
    pub fn with_solver_name(mut self, name: impl Into<String>) -> Self {
        self.solver_name = name.into();
        self
    }

    /// Sets the problem name.
    pub fn with_problem_name(mut self, name: impl Into<String>) -> Self {
        self.problem_name = name.into();
        self
    }

    /// Sets the warmup count.
    pub fn with_warmup_count(mut self, count: usize) -> Self {
        self.config = self.config.with_warmup_count(count);
        self
    }

    /// Sets the run count.
    pub fn with_run_count(mut self, count: usize) -> Self {
        self.config = self.config.with_run_count(count);
        self
    }

    /// Builds the benchmark with the given factories.
    pub fn build<Dir, P, D, F>(
        self,
        problem_factory: P,
        director_factory: D,
        solver_factory: F,
    ) -> Benchmark<S, Dir, P, D, F>
    where
        Dir: ScoreDirector<S>,
        P: Fn() -> S,
        D: Fn(S) -> Dir,
        F: Fn() -> Solver<S, Dir>,
    {
        Benchmark::new(
            self.config,
            self.solver_name,
            self.problem_name,
            problem_factory,
            director_factory,
            solver_factory,
        )
    }
}
