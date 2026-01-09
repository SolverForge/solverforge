//! SolverManager implementation.

#![allow(clippy::type_complexity)]

use solverforge_core::domain::PlanningSolution;

use crate::phase::Phase;
use crate::solver::Solver;
use crate::termination::Termination;

use super::SolverPhaseFactory;

/// High-level solver manager for ergonomic solving.
///
/// `SolverManager` stores solver configuration and can create solvers on demand.
/// For solving, use [`create_solver()`](Self::create_solver) to get a configured
/// [`Solver`] instance, then provide your own `ScoreDirector`.
///
/// # Creating a SolverManager
///
/// Use the builder pattern via [`SolverManager::builder()`]:
///
/// ```
/// use solverforge_solver::manager::SolverManager;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SimpleScore;
///
/// #[derive(Clone)]
/// struct Schedule {
///     tasks: Vec<i64>,
///     score: Option<SimpleScore>,
/// }
///
/// impl PlanningSolution for Schedule {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// // Build a manager (termination configured via solver.toml)
/// let manager = SolverManager::<Schedule>::builder(|s| {
///     // Simple scoring: sum of tasks
///     SimpleScore::of(s.tasks.iter().sum())
/// })
///     .build()
///     .expect("Failed to build manager");
///
/// // Score calculation is available without solving
/// let schedule = Schedule { tasks: vec![1, 2, 3], score: None };
/// let score = manager.calculate_score(&schedule);
/// assert_eq!(score, SimpleScore::of(6));
/// ```
///
/// # Creating Solvers
///
/// The manager creates fresh [`Solver`] instances for each solve:
///
/// ```
/// use solverforge_solver::manager::SolverManager;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SimpleScore;
///
/// # #[derive(Clone)]
/// # struct Schedule { score: Option<SimpleScore> }
/// # impl PlanningSolution for Schedule {
/// #     type Score = SimpleScore;
/// #     fn score(&self) -> Option<Self::Score> { self.score }
/// #     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// # }
/// let manager = SolverManager::<Schedule>::builder(|_| SimpleScore::of(0))
///     .build()
///     .unwrap();
///
/// // Create a solver for this problem instance
/// let solver = manager.create_solver();
///
/// // Each call creates a fresh solver with clean state
/// let solver2 = manager.create_solver();
/// ```
///
/// # Zero-Erasure Design
///
/// The score calculator is stored as a concrete generic type parameter `C`,
/// not as `Arc<dyn Fn>`. This eliminates virtual dispatch overhead for the
/// hot path (score calculation is called millions of times per solve).
///
/// The default `C = fn(&S) -> S::Score` allows writing `SolverManager::<T>::builder(...)`
/// without specifying the calculator type (it's inferred from the builder).
pub struct SolverManager<S: PlanningSolution, C = fn(&S) -> <S as PlanningSolution>::Score>
where
    C: Fn(&S) -> S::Score + Send + Sync,
{
    /// Score calculator function (zero-erasure: concrete generic type).
    score_calculator: C,

    /// Configured phases (as factories that create fresh phases per solve).
    phase_factories: Vec<Box<dyn SolverPhaseFactory<S>>>,

    /// Global termination condition factory.
    termination_factory: Option<Box<dyn Fn() -> Box<dyn Termination<S>> + Send + Sync>>,
}

impl<S: PlanningSolution> SolverManager<S, fn(&S) -> S::Score> {
    /// Creates a new [`SolverManagerBuilder`](super::SolverManagerBuilder) with the given score calculator.
    ///
    /// The score calculator is a function that computes the score for a solution.
    /// This is the entry point for building a `SolverManager`.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::manager::SolverManager;
    /// use solverforge_core::domain::PlanningSolution;
    /// use solverforge_core::score::SimpleScore;
    ///
    /// #[derive(Clone)]
    /// struct Problem { value: i64, score: Option<SimpleScore> }
    ///
    /// impl PlanningSolution for Problem {
    ///     type Score = SimpleScore;
    ///     fn score(&self) -> Option<Self::Score> { self.score }
    ///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
    /// }
    ///
    /// let builder = SolverManager::<Problem>::builder(|p| {
    ///     SimpleScore::of(-p.value.abs()) // Minimize absolute value
    /// });
    /// ```
    pub fn builder<F>(score_calculator: F) -> super::SolverManagerBuilder<S, F>
    where
        F: Fn(&S) -> S::Score + Send + Sync + 'static,
    {
        super::SolverManagerBuilder::new(score_calculator)
    }
}

impl<S, C> SolverManager<S, C>
where
    S: PlanningSolution,
    C: Fn(&S) -> S::Score + Send + Sync,
{
    /// Creates a SolverManager with explicit configuration (zero-erasure).
    pub(crate) fn new(
        score_calculator: C,
        phase_factories: Vec<Box<dyn SolverPhaseFactory<S>>>,
        termination_factory: Option<Box<dyn Fn() -> Box<dyn Termination<S>> + Send + Sync>>,
    ) -> Self {
        Self {
            score_calculator,
            phase_factories,
            termination_factory,
        }
    }

    /// Creates a fresh [`Solver`] instance with configured phases.
    ///
    /// Each call returns a new solver with clean state, suitable for solving
    /// a new problem instance. The solver is configured with termination
    /// conditions and phases from this manager.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::manager::SolverManager;
    /// use solverforge_core::domain::PlanningSolution;
    /// use solverforge_core::score::SimpleScore;
    ///
    /// # #[derive(Clone)]
    /// # struct Problem { score: Option<SimpleScore> }
    /// # impl PlanningSolution for Problem {
    /// #     type Score = SimpleScore;
    /// #     fn score(&self) -> Option<Self::Score> { self.score }
    /// #     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
    /// # }
    /// let manager = SolverManager::<Problem>::builder(|_| SimpleScore::of(0))
    ///     .build()
    ///     .unwrap();
    ///
    /// // Create solver - each call gives a fresh instance
    /// let solver = manager.create_solver();
    /// ```
    pub fn create_solver(&self) -> Solver<S> {
        // Create fresh phases for this solve
        let phases: Vec<Box<dyn Phase<S>>> = self
            .phase_factories
            .iter()
            .map(|f| f.create_phase())
            .collect();

        // Create solver
        let mut solver = Solver::new(phases);

        // Add termination if configured
        if let Some(factory) = &self.termination_factory {
            solver = solver.with_termination(factory());
        }

        solver
    }

    /// Returns a reference to the score calculator function.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::manager::SolverManager;
    /// use solverforge_core::domain::PlanningSolution;
    /// use solverforge_core::score::SimpleScore;
    ///
    /// # #[derive(Clone)]
    /// # struct Problem { value: i64, score: Option<SimpleScore> }
    /// # impl PlanningSolution for Problem {
    /// #     type Score = SimpleScore;
    /// #     fn score(&self) -> Option<Self::Score> { self.score }
    /// #     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
    /// # }
    /// let manager = SolverManager::<Problem>::builder(|p| SimpleScore::of(p.value))
    ///     .build()
    ///     .unwrap();
    ///
    /// let calculator = manager.score_calculator();
    /// let problem = Problem { value: 42, score: None };
    /// let score = calculator(&problem);
    /// assert_eq!(score, SimpleScore::of(42));
    /// ```
    pub fn score_calculator(&self) -> &C {
        &self.score_calculator
    }

    /// Calculates the score for a solution using the configured calculator.
    ///
    /// This is a convenience method equivalent to calling the score calculator
    /// directly.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::manager::SolverManager;
    /// use solverforge_core::domain::PlanningSolution;
    /// use solverforge_core::score::SimpleScore;
    ///
    /// # #[derive(Clone)]
    /// # struct Problem { value: i64, score: Option<SimpleScore> }
    /// # impl PlanningSolution for Problem {
    /// #     type Score = SimpleScore;
    /// #     fn score(&self) -> Option<Self::Score> { self.score }
    /// #     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
    /// # }
    /// let manager = SolverManager::<Problem>::builder(|p| {
    ///     SimpleScore::of(-p.value) // Negate for minimization
    /// })
    ///     .build()
    ///     .unwrap();
    ///
    /// let problem = Problem { value: 10, score: None };
    /// let score = manager.calculate_score(&problem);
    /// assert_eq!(score, SimpleScore::of(-10));
    /// ```
    pub fn calculate_score(&self, solution: &S) -> S::Score {
        (self.score_calculator)(solution)
    }
}
