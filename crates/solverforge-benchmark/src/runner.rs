//! Benchmark runner.

use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;
use solverforge_solver::stats::SolverStats;

use crate::config::BenchmarkConfig;
use crate::result::{BenchmarkResult, BenchmarkRun};

/// Result of a solve operation, containing both the solution and statistics.
///
/// This enables zero-erasure benchmarking by returning stats alongside the solution
/// instead of requiring shared state via Arc.
///
/// # Example
///
/// ```
/// use solverforge_benchmark::SolveResult;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SimpleScore;
/// use solverforge_solver::stats::SolverStats;
///
/// #[derive(Clone)]
/// struct MySolution {
///     score: Option<SimpleScore>,
/// }
///
/// impl PlanningSolution for MySolution {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// let solution = MySolution { score: Some(SimpleScore::of(0)) };
/// let mut stats = SolverStats::default();
/// stats.moves_evaluated = 1000;
/// stats.moves_accepted = 500;
///
/// let result = SolveResult::new(solution, stats);
/// assert_eq!(result.stats.moves_evaluated, 1000);
/// ```
pub struct SolveResult<S: PlanningSolution> {
    /// The final solution.
    pub solution: S,
    /// Statistics collected during solving.
    pub stats: SolverStats,
}

impl<S: PlanningSolution> SolveResult<S> {
    /// Creates a new solve result.
    pub fn new(solution: S, stats: SolverStats) -> Self {
        Self { solution, stats }
    }
}

/// Trait for types that can solve a problem with a score director.
///
/// This enables benchmarking any solver implementation. The solve method
/// returns both the solution and statistics to support zero-erasure benchmarking.
///
/// # Example
///
/// ```
/// use solverforge_benchmark::{Solvable, SolveResult};
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SimpleScore;
/// use solverforge_scoring::ScoreDirector;
/// use solverforge_solver::stats::SolverStats;
///
/// #[derive(Clone)]
/// struct MySolution {
///     score: Option<SimpleScore>,
/// }
///
/// impl PlanningSolution for MySolution {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// struct MySolver;
///
/// impl<D: ScoreDirector<MySolution>> Solvable<MySolution, D> for MySolver {
///     fn solve(&mut self, mut director: D) -> SolveResult<MySolution> {
///         let mut solution = director.working_solution().clone();
///         let score = director.calculate_score();
///         solution.set_score(Some(score));
///
///         let mut stats = SolverStats::default();
///         stats.moves_evaluated = 100;
///         stats.moves_accepted = 50;
///         stats.score_calculations = 100;
///
///         SolveResult::new(solution, stats)
///     }
/// }
/// ```
pub trait Solvable<S: PlanningSolution, Dir: ScoreDirector<S>> {
    /// Solves the problem using the provided score director.
    ///
    /// Returns the final solution along with statistics collected during solving.
    fn solve(&mut self, score_director: Dir) -> SolveResult<S>;
}

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
/// * `Slv` - The solver type
/// * `P` - Problem factory: `Fn() -> S`
/// * `D` - Score director factory: `Fn(S) -> Dir`
/// * `F` - Solver factory: `Fn() -> Slv`
///
/// # Example
///
/// ```
/// use solverforge_benchmark::{Benchmark, BenchmarkConfig, Solvable, SolveResult};
/// use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
/// use solverforge_core::score::SimpleScore;
/// use solverforge_scoring::{ScoreDirector, SimpleScoreDirector};
/// use solverforge_solver::stats::SolverStats;
/// use std::any::TypeId;
///
/// #[derive(Clone)]
/// struct MySolution {
///     score: Option<SimpleScore>,
/// }
///
/// impl PlanningSolution for MySolution {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// type MyDirector = SimpleScoreDirector<MySolution, fn(&MySolution) -> SimpleScore>;
///
/// fn calc(_: &MySolution) -> SimpleScore { SimpleScore::of(0) }
///
/// struct MySolver;
///
/// impl Solvable<MySolution, MyDirector> for MySolver {
///     fn solve(&mut self, mut director: MyDirector) -> SolveResult<MySolution> {
///         let mut solution = director.working_solution().clone();
///         solution.set_score(Some(director.calculate_score()));
///         let mut stats = SolverStats::default();
///         stats.start();
///         stats.moves_evaluated = 100;
///         SolveResult::new(solution, stats)
///     }
/// }
///
/// let config = BenchmarkConfig::new("Test")
///     .with_warmup_count(0)
///     .with_run_count(2);
///
/// let benchmark = Benchmark::new(
///     config,
///     "HC",
///     "Problem",
///     || MySolution { score: None },
///     |s| {
///         let desc = SolutionDescriptor::new("MySolution", TypeId::of::<MySolution>());
///         SimpleScoreDirector::with_calculator(s, desc, calc as fn(&MySolution) -> SimpleScore)
///     },
///     || MySolver,
/// );
///
/// let result = benchmark.run();
/// assert_eq!(result.run_count(), 2);
/// ```
pub struct Benchmark<S, Dir, Slv, P, D, F>
where
    S: PlanningSolution,
    Dir: ScoreDirector<S>,
    Slv: Solvable<S, Dir>,
    P: Fn() -> S,
    D: Fn(S) -> Dir,
    F: Fn() -> Slv,
{
    config: BenchmarkConfig,
    solver_name: String,
    problem_name: String,
    problem_factory: P,
    director_factory: D,
    solver_factory: F,
    _phantom: PhantomData<(S, Dir, Slv)>,
}

impl<S, Dir, Slv, P, D, F> Benchmark<S, Dir, Slv, P, D, F>
where
    S: PlanningSolution,
    Dir: ScoreDirector<S>,
    Slv: Solvable<S, Dir>,
    P: Fn() -> S,
    D: Fn(S) -> Dir,
    F: Fn() -> Slv,
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
        // Warmup runs (not measured)
        for _ in 0..self.config.warmup_count() {
            self.run_once();
        }

        // Measurement runs
        let mut result =
            BenchmarkResult::new(self.config.name(), &self.solver_name, &self.problem_name);

        for run_index in 0..self.config.run_count() {
            let solve_result = self.run_once();
            let solve_time = solve_result.stats.elapsed();

            let final_score = solve_result.solution.score().unwrap_or_else(|| {
                // Calculate score if not set
                let mut director = (self.director_factory)(solve_result.solution);
                director.calculate_score()
            });

            let run = BenchmarkRun::new(
                run_index,
                solve_time,
                final_score,
                solve_result.stats.moves_evaluated,
                solve_result.stats.moves_accepted,
                solve_result.stats.score_calculations,
            );
            result.add_run(run);
        }

        result
    }

    /// Executes a single run.
    fn run_once(&self) -> SolveResult<S> {
        let problem = (self.problem_factory)();
        let director = (self.director_factory)(problem);
        let mut solver = (self.solver_factory)();
        solver.solve(director)
    }
}

/// Builder for creating benchmarks with fluent API.
///
/// # Example
///
/// ```
/// use solverforge_benchmark::{BenchmarkBuilder, Solvable, SolveResult};
/// use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
/// use solverforge_core::score::SimpleScore;
/// use solverforge_scoring::{ScoreDirector, SimpleScoreDirector};
/// use solverforge_solver::stats::SolverStats;
/// use std::any::TypeId;
///
/// #[derive(Clone)]
/// struct MySolution {
///     score: Option<SimpleScore>,
/// }
///
/// impl PlanningSolution for MySolution {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// type MyDirector = SimpleScoreDirector<MySolution, fn(&MySolution) -> SimpleScore>;
///
/// fn calc(_: &MySolution) -> SimpleScore { SimpleScore::of(0) }
///
/// struct MySolver;
///
/// impl Solvable<MySolution, MyDirector> for MySolver {
///     fn solve(&mut self, mut director: MyDirector) -> SolveResult<MySolution> {
///         let mut solution = director.working_solution().clone();
///         solution.set_score(Some(director.calculate_score()));
///         SolveResult::new(solution, SolverStats::default())
///     }
/// }
///
/// let benchmark = BenchmarkBuilder::<MySolution, MyDirector>::new("Test")
///     .with_solver_name("HC")
///     .with_problem_name("Problem1")
///     .with_warmup_count(0)
///     .with_run_count(1)
///     .build(
///         || MySolution { score: None },
///         |s| {
///             let desc = SolutionDescriptor::new("MySolution", TypeId::of::<MySolution>());
///             SimpleScoreDirector::with_calculator(s, desc, calc as fn(&MySolution) -> SimpleScore)
///         },
///         || MySolver,
///     );
/// ```
pub struct BenchmarkBuilder<S, Dir>
where
    S: PlanningSolution,
    Dir: ScoreDirector<S>,
{
    config: BenchmarkConfig,
    solver_name: String,
    problem_name: String,
    _phantom: PhantomData<(S, Dir)>,
}

impl<S, Dir> BenchmarkBuilder<S, Dir>
where
    S: PlanningSolution,
    Dir: ScoreDirector<S>,
{
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
    pub fn build<Slv, P, D, F>(
        self,
        problem_factory: P,
        director_factory: D,
        solver_factory: F,
    ) -> Benchmark<S, Dir, Slv, P, D, F>
    where
        Slv: Solvable<S, Dir>,
        P: Fn() -> S,
        D: Fn(S) -> Dir,
        F: Fn() -> Slv,
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
