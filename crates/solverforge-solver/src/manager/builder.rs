//! Builder for SolverManager configuration.
//!
//! This module provides the builder pattern for configuring a [`SolverManager`].
//! The builder allows fluent configuration of:
//!
//! - Construction heuristic phases
//! - Local search phases with various acceptors
//! - Termination conditions (time limits, step limits)
//!
//! # Example
//!
//! ```
//! use solverforge_solver::manager::{SolverManagerBuilder, LocalSearchType, ConstructionType};
//! use solverforge_core::domain::PlanningSolution;
//! use solverforge_core::score::SimpleScore;
//! use std::time::Duration;
//!
//! #[derive(Clone)]
//! struct Schedule { score: Option<SimpleScore> }
//!
//! impl PlanningSolution for Schedule {
//!     type Score = SimpleScore;
//!     fn score(&self) -> Option<Self::Score> { self.score }
//!     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
//! }
//!
//! let manager = SolverManagerBuilder::new(|_: &Schedule| SimpleScore::of(0))
//!     .with_construction_heuristic()
//!     .with_local_search(LocalSearchType::HillClimbing)
//!     .with_time_limit(Duration::from_secs(60))
//!     .build()
//!     .unwrap();
//! ```

use std::time::Duration;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::SolverForgeError;

use crate::termination::{
    OrCompositeTermination, StepCountTermination, Termination, TimeTermination,
};

use super::config::{ConstructionType, LocalSearchType, PhaseConfig};
use super::{SolverPhaseFactory, SolverManager};

/// Builder for creating a [`SolverManager`] with fluent configuration.
///
/// The builder pattern allows configuring phases, termination conditions,
/// and other solver settings before creating the manager.
///
/// # Basic Usage
///
/// ```
/// use solverforge_solver::manager::{SolverManagerBuilder, LocalSearchType};
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SimpleScore;
/// use std::time::Duration;
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
/// let manager = SolverManagerBuilder::new(|p: &Problem| SimpleScore::of(-p.value))
///     .with_construction_heuristic()
///     .with_local_search(LocalSearchType::HillClimbing)
///     .with_time_limit(Duration::from_secs(30))
///     .build()
///     .expect("Failed to build manager");
/// ```
///
/// # Configuration Options
///
/// The builder supports:
/// - Construction heuristic phases (first fit, best fit)
/// - Local search phases (hill climbing, tabu search, simulated annealing, late acceptance)
/// - Time limits
/// - Step limits
///
/// # Multi-Phase Configuration
///
/// ```
/// use solverforge_solver::manager::{SolverManagerBuilder, LocalSearchType, ConstructionType};
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SimpleScore;
/// use std::time::Duration;
///
/// # #[derive(Clone)]
/// # struct Problem { score: Option<SimpleScore> }
/// # impl PlanningSolution for Problem {
/// #     type Score = SimpleScore;
/// #     fn score(&self) -> Option<Self::Score> { self.score }
/// #     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// # }
/// let manager = SolverManagerBuilder::new(|_: &Problem| SimpleScore::of(0))
///     // First phase: construct initial solution
///     .with_construction_heuristic_type(ConstructionType::BestFit)
///     // Second phase: improve with tabu search
///     .with_local_search_steps(LocalSearchType::TabuSearch { tabu_size: 7 }, 1000)
///     // Third phase: fine-tune with hill climbing
///     .with_local_search(LocalSearchType::HillClimbing)
///     // Global termination
///     .with_time_limit(Duration::from_secs(60))
///     .build()
///     .unwrap();
/// ```
///
/// # Zero-Erasure Design
///
/// The score calculator is stored as a concrete generic type parameter `C`,
/// not as `Arc<dyn Fn>`. This eliminates virtual dispatch overhead.
pub struct SolverManagerBuilder<S, C>
where
    S: PlanningSolution,
    C: Fn(&S) -> S::Score + Send + Sync,
{
    score_calculator: C,
    phase_configs: Vec<PhaseConfig>,
    time_limit: Option<Duration>,
    step_limit: Option<u64>,
    _phantom: std::marker::PhantomData<S>,
}

impl<S, C> SolverManagerBuilder<S, C>
where
    S: PlanningSolution,
    C: Fn(&S) -> S::Score + Send + Sync + 'static,
{
    /// Creates a new builder with the given score calculator (zero-erasure).
    ///
    /// The score calculator is a function that computes the score for a solution.
    /// Higher scores are better (for minimization, use negative values).
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::manager::SolverManagerBuilder;
    /// use solverforge_core::domain::PlanningSolution;
    /// use solverforge_core::score::SimpleScore;
    ///
    /// # #[derive(Clone)]
    /// # struct Problem { cost: i64, score: Option<SimpleScore> }
    /// # impl PlanningSolution for Problem {
    /// #     type Score = SimpleScore;
    /// #     fn score(&self) -> Option<Self::Score> { self.score }
    /// #     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
    /// # }
    /// // For minimization, negate the cost
    /// let builder = SolverManagerBuilder::new(|p: &Problem| {
    ///     SimpleScore::of(-p.cost)
    /// });
    /// ```
    pub fn new(score_calculator: C) -> Self {
        Self {
            score_calculator,
            phase_configs: Vec::new(),
            time_limit: None,
            step_limit: None,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Adds a construction heuristic phase with default (FirstFit) configuration.
    ///
    /// This phase will build an initial solution by assigning values to
    /// uninitialized planning variables using the first valid value found.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::manager::SolverManagerBuilder;
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
    /// let builder = SolverManagerBuilder::new(|_: &Problem| SimpleScore::of(0))
    ///     .with_construction_heuristic();
    /// ```
    pub fn with_construction_heuristic(mut self) -> Self {
        self.phase_configs.push(PhaseConfig::ConstructionHeuristic {
            construction_type: ConstructionType::FirstFit,
        });
        self
    }

    /// Adds a construction heuristic phase with specific configuration.
    ///
    /// Use this to choose between [`ConstructionType::FirstFit`] (fast) and
    /// [`ConstructionType::BestFit`] (better quality initial solution).
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::manager::{SolverManagerBuilder, ConstructionType};
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
    /// let builder = SolverManagerBuilder::new(|_: &Problem| SimpleScore::of(0))
    ///     .with_construction_heuristic_type(ConstructionType::BestFit);
    /// ```
    pub fn with_construction_heuristic_type(mut self, construction_type: ConstructionType) -> Self {
        self.phase_configs.push(PhaseConfig::ConstructionHeuristic {
            construction_type,
        });
        self
    }

    /// Adds a local search phase.
    ///
    /// Local search improves an existing solution by exploring neighboring
    /// solutions. The search type determines the acceptance criteria.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::manager::{SolverManagerBuilder, LocalSearchType};
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
    /// let builder = SolverManagerBuilder::new(|_: &Problem| SimpleScore::of(0))
    ///     .with_local_search(LocalSearchType::TabuSearch { tabu_size: 7 });
    /// ```
    pub fn with_local_search(mut self, search_type: LocalSearchType) -> Self {
        self.phase_configs.push(PhaseConfig::LocalSearch {
            search_type,
            step_limit: None,
        });
        self
    }

    /// Adds a local search phase with a step limit.
    ///
    /// The phase will terminate after the specified number of steps,
    /// allowing for multi-phase configurations where different search
    /// strategies are used in sequence.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::manager::{SolverManagerBuilder, LocalSearchType};
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
    /// let builder = SolverManagerBuilder::new(|_: &Problem| SimpleScore::of(0))
    ///     // First, use simulated annealing for 500 steps
    ///     .with_local_search_steps(
    ///         LocalSearchType::SimulatedAnnealing {
    ///             starting_temp: 1.0,
    ///             decay_rate: 0.99,
    ///         },
    ///         500,
    ///     )
    ///     // Then switch to hill climbing
    ///     .with_local_search(LocalSearchType::HillClimbing);
    /// ```
    pub fn with_local_search_steps(
        mut self,
        search_type: LocalSearchType,
        step_limit: u64,
    ) -> Self {
        self.phase_configs.push(PhaseConfig::LocalSearch {
            search_type,
            step_limit: Some(step_limit),
        });
        self
    }

    /// Sets the global time limit for solving.
    ///
    /// The solver will terminate after this duration, regardless of
    /// which phase is currently executing.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::manager::SolverManagerBuilder;
    /// use solverforge_core::domain::PlanningSolution;
    /// use solverforge_core::score::SimpleScore;
    /// use std::time::Duration;
    ///
    /// # #[derive(Clone)]
    /// # struct Problem { score: Option<SimpleScore> }
    /// # impl PlanningSolution for Problem {
    /// #     type Score = SimpleScore;
    /// #     fn score(&self) -> Option<Self::Score> { self.score }
    /// #     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
    /// # }
    /// let builder = SolverManagerBuilder::new(|_: &Problem| SimpleScore::of(0))
    ///     .with_time_limit(Duration::from_secs(60));
    /// ```
    pub fn with_time_limit(mut self, duration: Duration) -> Self {
        self.time_limit = Some(duration);
        self
    }

    /// Sets the global step limit for solving.
    ///
    /// The solver will terminate after this many steps total across all phases.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::manager::SolverManagerBuilder;
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
    /// let builder = SolverManagerBuilder::new(|_: &Problem| SimpleScore::of(0))
    ///     .with_step_limit(10000);
    /// ```
    pub fn with_step_limit(mut self, steps: u64) -> Self {
        self.step_limit = Some(steps);
        self
    }

    /// Builds the [`SolverManager`].
    ///
    /// This creates a basic `SolverManager` with the configured termination
    /// conditions. For full functionality with phases, use the typed phase
    /// factories from [`super::phase_factory`].
    ///
    /// # Errors
    ///
    /// Currently this method always succeeds, but returns a `Result` for
    /// forward compatibility with validation.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::manager::{SolverManagerBuilder, LocalSearchType};
    /// use solverforge_core::domain::PlanningSolution;
    /// use solverforge_core::score::SimpleScore;
    /// use std::time::Duration;
    ///
    /// # #[derive(Clone)]
    /// # struct Problem { score: Option<SimpleScore> }
    /// # impl PlanningSolution for Problem {
    /// #     type Score = SimpleScore;
    /// #     fn score(&self) -> Option<Self::Score> { self.score }
    /// #     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
    /// # }
    /// let manager = SolverManagerBuilder::new(|_: &Problem| SimpleScore::of(0))
    ///     .with_construction_heuristic()
    ///     .with_local_search(LocalSearchType::HillClimbing)
    ///     .with_time_limit(Duration::from_secs(30))
    ///     .build()
    ///     .expect("Failed to build manager");
    ///
    /// // Manager is ready to create solvers
    /// let solver = manager.create_solver();
    /// ```
    pub fn build(self) -> Result<SolverManager<S, C>, SolverForgeError> {
        // Build termination factory
        let termination_factory = self.build_termination_factory();

        // For now, phase factories are empty - users need to add phases manually
        // or use the typed phase constructors. Full auto-configuration requires
        // macro enhancements to generate the necessary metadata.
        let phase_factories: Vec<Box<dyn SolverPhaseFactory<S>>> = Vec::new();

        // Store phase configs for future use
        // (will be used when PhaseFactory auto-configuration is implemented)
        let _ = self.phase_configs;

        Ok(SolverManager::new(
            self.score_calculator,
            phase_factories,
            termination_factory,
        ))
    }

    #[allow(clippy::type_complexity)]
    fn build_termination_factory(
        &self,
    ) -> Option<Box<dyn Fn() -> Box<dyn Termination<S>> + Send + Sync>> {
        let time_limit = self.time_limit;
        let step_limit = self.step_limit;

        if time_limit.is_none() && step_limit.is_none() {
            return None;
        }

        Some(Box::new(move || {
            let mut terminations: Vec<Box<dyn Termination<S>>> = Vec::new();

            if let Some(duration) = time_limit {
                terminations.push(Box::new(TimeTermination::new(duration)));
            }

            if let Some(steps) = step_limit {
                terminations.push(Box::new(StepCountTermination::new(steps)));
            }

            match terminations.len() {
                0 => unreachable!(),
                1 => terminations.remove(0),
                _ => Box::new(OrCompositeTermination::new(terminations)),
            }
        }))
    }
}
