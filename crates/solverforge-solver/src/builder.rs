//! Builder module for constructing solver components from configuration
//!
//! This module provides the wiring between configuration types and
//! the actual solver implementation.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use solverforge_config::{AcceptorConfig, TerminationConfig};
use solverforge_scoring::{ConstraintSet, ShadowAwareScoreDirector, SolvableSolution, TypedScoreDirector};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;

use crate::phase::localsearch::{
    Acceptor, HillClimbingAcceptor, LateAcceptanceAcceptor, SimulatedAnnealingAcceptor,
    TabuSearchAcceptor,
};
use crate::phase::Phase;
use crate::termination::{
    AndCompositeTermination, BestScoreTermination, OrCompositeTermination, StepCountTermination,
    Termination, TimeTermination, UnimprovedStepCountTermination, UnimprovedTimeTermination,
};

/// Builder for constructing termination conditions from configuration.
pub struct TerminationBuilder;

impl TerminationBuilder {
    /// Builds a termination condition from configuration.
    ///
    /// Multiple termination conditions are combined with OR logic
    /// (any condition being met will terminate solving).
    pub fn build<S: PlanningSolution>(
        config: &TerminationConfig,
    ) -> Option<Box<dyn Termination<S>>> {
        let mut terminations: Vec<Box<dyn Termination<S>>> = Vec::new();

        // Time-based termination
        if let Some(time_limit) = config.time_limit() {
            terminations.push(Box::new(TimeTermination::new(time_limit)));
        }

        // Step count termination
        if let Some(step_limit) = config.step_count_limit {
            terminations.push(Box::new(StepCountTermination::new(step_limit as u64)));
        }

        // Unimproved step count termination
        if let Some(unimproved_limit) = config.unimproved_step_count_limit {
            terminations.push(Box::new(UnimprovedStepCountTermination::<S>::new(
                unimproved_limit as u64,
            )));
        }

        // Unimproved time termination
        if let Some(unimproved_seconds) = config.unimproved_seconds_spent_limit {
            terminations.push(Box::new(UnimprovedTimeTermination::<S>::new(
                Duration::from_secs(unimproved_seconds),
            )));
        }

        // Combine terminations
        match terminations.len() {
            0 => None,
            1 => Some(terminations.remove(0)),
            _ => Some(Box::new(OrCompositeTermination::new(terminations))),
        }
    }

    /// Builds a termination condition that requires ALL conditions to be met.
    pub fn build_and<S: PlanningSolution>(
        config: &TerminationConfig,
    ) -> Option<Box<dyn Termination<S>>> {
        let mut terminations: Vec<Box<dyn Termination<S>>> = Vec::new();

        if let Some(time_limit) = config.time_limit() {
            terminations.push(Box::new(TimeTermination::new(time_limit)));
        }

        if let Some(step_limit) = config.step_count_limit {
            terminations.push(Box::new(StepCountTermination::new(step_limit as u64)));
        }

        match terminations.len() {
            0 => None,
            1 => Some(terminations.remove(0)),
            _ => Some(Box::new(AndCompositeTermination::new(terminations))),
        }
    }

    /// Creates a termination that stops when best score reaches a target.
    pub fn best_score<S, Sc>(target: Sc) -> Box<dyn Termination<S>>
    where
        S: PlanningSolution<Score = Sc>,
        Sc: Score,
    {
        Box::new(BestScoreTermination::new(target))
    }
}

/// Builder for constructing acceptors from configuration.
pub struct AcceptorBuilder;

impl AcceptorBuilder {
    /// Builds an acceptor from configuration.
    pub fn build<S: PlanningSolution>(config: &AcceptorConfig) -> Box<dyn Acceptor<S>> {
        match config {
            AcceptorConfig::HillClimbing => Box::new(HillClimbingAcceptor::new()),

            AcceptorConfig::TabuSearch(tabu_config) => {
                // Use entity tabu size if specified, otherwise default
                let tabu_size = tabu_config
                    .entity_tabu_size
                    .or(tabu_config.move_tabu_size)
                    .unwrap_or(7);
                Box::new(TabuSearchAcceptor::<S>::new(tabu_size))
            }

            AcceptorConfig::SimulatedAnnealing(sa_config) => {
                // Parse starting temperature (default to 1.0 if not specified)
                let starting_temp = sa_config
                    .starting_temperature
                    .as_ref()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(1.0);
                Box::new(SimulatedAnnealingAcceptor::new(starting_temp, 0.99))
            }

            AcceptorConfig::LateAcceptance(la_config) => {
                let size = la_config.late_acceptance_size.unwrap_or(400);
                Box::new(LateAcceptanceAcceptor::<S>::new(size))
            }

            AcceptorConfig::GreatDeluge(_) => {
                // Great deluge not yet implemented, fall back to hill climbing
                tracing::warn!("Great deluge acceptor not yet implemented, using hill climbing");
                Box::new(HillClimbingAcceptor::new())
            }
        }
    }

    /// Creates a default hill climbing acceptor.
    pub fn hill_climbing<S: PlanningSolution>() -> Box<dyn Acceptor<S>> {
        Box::new(HillClimbingAcceptor::new())
    }

    /// Creates a tabu search acceptor with the given size.
    pub fn tabu_search<S: PlanningSolution>(tabu_size: usize) -> Box<dyn Acceptor<S>> {
        Box::new(TabuSearchAcceptor::<S>::new(tabu_size))
    }

    /// Creates a simulated annealing acceptor.
    pub fn simulated_annealing<S: PlanningSolution>(
        starting_temp: f64,
        decay_rate: f64,
    ) -> Box<dyn Acceptor<S>> {
        Box::new(SimulatedAnnealingAcceptor::new(starting_temp, decay_rate))
    }

    /// Creates a late acceptance acceptor.
    pub fn late_acceptance<S: PlanningSolution>(size: usize) -> Box<dyn Acceptor<S>> {
        Box::new(LateAcceptanceAcceptor::<S>::new(size))
    }
}

/// Builder for constructing a complete solver from configuration.
///
/// Note: Phase building now requires typed move selectors, so phases
/// must be constructed directly using the typed phase constructors.
///
/// # Example
///
/// ```
/// use solverforge_solver::{SolverBuilder, StepCountTermination};
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SimpleScore;
///
/// #[derive(Clone, Debug)]
/// struct MySolution { score: Option<SimpleScore> }
///
/// impl PlanningSolution for MySolution {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// let solver = SolverBuilder::<MySolution>::new()
///     .with_termination(Box::new(StepCountTermination::new(100)))
///     .build();
/// ```
pub struct SolverBuilder<S: PlanningSolution> {
    phases: Vec<Box<dyn Phase<S>>>,
    termination: Option<Box<dyn Termination<S>>>,
    terminate_flag: Option<Arc<AtomicBool>>,
}

impl<S: PlanningSolution> SolverBuilder<S> {
    /// Creates a new solver builder.
    pub fn new() -> Self {
        SolverBuilder {
            phases: Vec::new(),
            termination: None,
            terminate_flag: None,
        }
    }

    /// Sets an external termination flag for early termination.
    ///
    /// When the flag is set to `true`, the solver will terminate at the next
    /// opportunity (typically at the end of a step or between phases).
    pub fn with_terminate_flag(mut self, flag: Arc<AtomicBool>) -> Self {
        self.terminate_flag = Some(flag);
        self
    }

    /// Adds a phase to the builder.
    pub fn with_phase(mut self, phase: Box<dyn Phase<S>>) -> Self {
        self.phases.push(phase);
        self
    }

    /// Adds multiple phases to the builder.
    pub fn with_phases(mut self, phases: Vec<Box<dyn Phase<S>>>) -> Self {
        self.phases.extend(phases);
        self
    }

    /// Sets the termination condition.
    pub fn with_termination(mut self, termination: Box<dyn Termination<S>>) -> Self {
        self.termination = Some(termination);
        self
    }

    /// Sets a time-based termination condition.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::SolverBuilder;
    /// use solverforge_core::domain::PlanningSolution;
    /// use solverforge_core::score::SimpleScore;
    /// use std::time::Duration;
    ///
    /// #[derive(Clone, Debug)]
    /// struct MySolution { score: Option<SimpleScore> }
    ///
    /// impl PlanningSolution for MySolution {
    ///     type Score = SimpleScore;
    ///     fn score(&self) -> Option<Self::Score> { self.score }
    ///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
    /// }
    ///
    /// let solver = SolverBuilder::<MySolution>::new()
    ///     .with_time_limit(Duration::from_secs(30))
    ///     .build();
    /// ```
    pub fn with_time_limit(mut self, duration: Duration) -> Self {
        self.termination = Some(Box::new(TimeTermination::new(duration)));
        self
    }

    /// Sets a step count termination condition.
    pub fn with_step_limit(mut self, steps: u64) -> Self {
        self.termination = Some(Box::new(StepCountTermination::new(steps)));
        self
    }

    /// Builds termination from the configuration.
    pub fn with_termination_from_config(mut self, config: &TerminationConfig) -> Self {
        self.termination = TerminationBuilder::build(config);
        self
    }

    /// Builds the solver.
    pub fn build(self) -> crate::solver::Solver<S> {
        let mut solver = crate::solver::Solver::new(self.phases);
        if let Some(termination) = self.termination {
            solver = solver.with_termination(termination);
        }
        solver
    }

    /// Solves the problem with automatic wiring.
    ///
    /// This is the zero-manual-wiring API that auto-creates all infrastructure:
    /// - TypedScoreDirector from the constraints function
    /// - ShadowAwareScoreDirector wrapper for shadow variable updates
    /// - SolverScope with termination flag
    ///
    /// # Type Requirements
    ///
    /// The solution type `S` must implement `SolvableSolution`, which provides:
    /// - `descriptor()` - solution metadata for the solver
    /// - `entity_count()` - entity count function for incremental scoring
    ///
    /// # Arguments
    ///
    /// * `solution` - The initial solution to optimize
    /// * `constraints` - A function returning the constraint set
    ///
    /// # Example
    ///
    /// ```ignore
    /// let result = SolverBuilder::<VehicleRoutePlan>::new()
    ///     .with_phase(construction_phase)
    ///     .with_phase(local_search_phase)
    ///     .with_terminate_flag(terminate_flag)
    ///     .solve(solution, define_constraints);
    /// ```
    pub fn solve<C>(self, solution: S, constraints: fn() -> C) -> S
    where
        S: SolvableSolution + 'static,
        C: ConstraintSet<S, S::Score> + 'static,
    {
        use crate::scope::SolverScope;

        // Auto-wire: Create TypedScoreDirector from constraints
        let descriptor = S::descriptor();
        let constraint_set = constraints();
        let inner_director = TypedScoreDirector::with_descriptor(
            solution,
            constraint_set,
            descriptor,
            S::entity_count,
        );

        // Auto-wire: Wrap with ShadowAwareScoreDirector
        let director = ShadowAwareScoreDirector::new(inner_director);

        // Create solver scope
        let mut solver_scope = SolverScope::new(Box::new(director));

        // Set termination flag if provided
        if let Some(flag) = self.terminate_flag {
            solver_scope.set_terminate_early_flag(flag);
        }

        // Start solving
        solver_scope.start_solving();

        // Execute all phases
        for mut phase in self.phases {
            if solver_scope.is_terminate_early() {
                break;
            }
            phase.solve(&mut solver_scope);
        }

        // Return the best solution (or working solution if no best was set)
        solver_scope.take_best_or_working_solution()
    }
}

impl<S: PlanningSolution> Default for SolverBuilder<S> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solverforge_config::{
        AcceptorConfig, LateAcceptanceConfig, SimulatedAnnealingConfig, TabuSearchConfig,
        TerminationConfig,
    };
    use solverforge_core::score::SimpleScore;

    #[derive(Clone, Debug)]
    struct TestSolution {
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for TestSolution {
        type Score = SimpleScore;
        fn score(&self) -> Option<Self::Score> {
            self.score
        }
        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    #[test]
    fn test_termination_builder_time_limit() {
        let config = TerminationConfig {
            seconds_spent_limit: Some(30),
            ..Default::default()
        };

        let term = TerminationBuilder::build::<TestSolution>(&config);
        assert!(term.is_some());
    }

    #[test]
    fn test_termination_builder_step_limit() {
        let config = TerminationConfig {
            step_count_limit: Some(100),
            ..Default::default()
        };

        let term = TerminationBuilder::build::<TestSolution>(&config);
        assert!(term.is_some());
    }

    #[test]
    fn test_termination_builder_multiple() {
        let config = TerminationConfig {
            seconds_spent_limit: Some(30),
            step_count_limit: Some(100),
            ..Default::default()
        };

        let term = TerminationBuilder::build::<TestSolution>(&config);
        assert!(term.is_some());
    }

    #[test]
    fn test_termination_builder_empty() {
        let config = TerminationConfig::default();
        let term = TerminationBuilder::build::<TestSolution>(&config);
        assert!(term.is_none());
    }

    #[test]
    fn test_termination_builder_unimproved() {
        let config = TerminationConfig {
            unimproved_step_count_limit: Some(50),
            unimproved_seconds_spent_limit: Some(10),
            ..Default::default()
        };

        let term = TerminationBuilder::build::<TestSolution>(&config);
        assert!(term.is_some());
    }

    #[test]
    fn test_acceptor_builder_hill_climbing() {
        let config = AcceptorConfig::HillClimbing;
        let _acceptor: Box<dyn Acceptor<TestSolution>> = AcceptorBuilder::build(&config);
    }

    #[test]
    fn test_acceptor_builder_tabu_search() {
        let config = AcceptorConfig::TabuSearch(TabuSearchConfig {
            entity_tabu_size: Some(10),
            ..Default::default()
        });
        let _acceptor: Box<dyn Acceptor<TestSolution>> = AcceptorBuilder::build(&config);
    }

    #[test]
    fn test_acceptor_builder_simulated_annealing() {
        let config = AcceptorConfig::SimulatedAnnealing(SimulatedAnnealingConfig {
            starting_temperature: Some("1.5".to_string()),
        });
        let _acceptor: Box<dyn Acceptor<TestSolution>> = AcceptorBuilder::build(&config);
    }

    #[test]
    fn test_acceptor_builder_late_acceptance() {
        let config = AcceptorConfig::LateAcceptance(LateAcceptanceConfig {
            late_acceptance_size: Some(500),
        });
        let _acceptor: Box<dyn Acceptor<TestSolution>> = AcceptorBuilder::build(&config);
    }

    #[test]
    fn test_solver_builder() {
        let config = TerminationConfig {
            seconds_spent_limit: Some(30),
            ..Default::default()
        };

        let builder = SolverBuilder::<TestSolution>::new().with_termination_from_config(&config);

        let solver = builder.build();
        assert!(!solver.is_solving());
    }
}
