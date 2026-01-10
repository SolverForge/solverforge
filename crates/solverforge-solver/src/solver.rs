//! Solver and SolverFactory implementations

use std::fmt::Debug;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use solverforge_config::SolverConfig;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::SolverForgeError;
use solverforge_scoring::ScoreDirector;

use crate::phase::Phase;
use crate::scope::SolverScope;
use crate::termination::Termination;

/// Factory for creating Solver instances.
pub struct SolverFactory<S: PlanningSolution> {
    config: SolverConfig,
    _phantom: PhantomData<S>,
}

impl<S: PlanningSolution> SolverFactory<S> {
    /// Creates a new SolverFactory from configuration.
    pub fn create(config: SolverConfig) -> Result<Self, SolverForgeError> {
        Ok(SolverFactory {
            config,
            _phantom: PhantomData,
        })
    }

    /// Creates a SolverFactory from a TOML configuration file.
    pub fn from_toml_file(path: impl AsRef<std::path::Path>) -> Result<Self, SolverForgeError> {
        let config = SolverConfig::from_toml_file(path)
            .map_err(|e| SolverForgeError::Config(e.to_string()))?;
        Self::create(config)
    }

    /// Creates a SolverFactory from a YAML configuration file.
    pub fn from_yaml_file(path: impl AsRef<std::path::Path>) -> Result<Self, SolverForgeError> {
        let config = SolverConfig::from_yaml_file(path)
            .map_err(|e| SolverForgeError::Config(e.to_string()))?;
        Self::create(config)
    }

    /// Returns a reference to the configuration.
    pub fn config(&self) -> &SolverConfig {
        &self.config
    }
}

/// The main solver that optimizes planning solutions.
///
/// Uses macro-generated tuple implementations for phases, preserving
/// concrete types through the entire pipeline (zero-erasure architecture).
///
/// # Type Parameters
/// * `P` - Tuple of phases to execute
/// * `T` - Termination condition (use `Option<ConcreteTermination>`)
/// * `S` - Solution type
/// * `D` - Score director type
///
/// # Example
///
/// ```ignore
/// use solverforge_solver::{Solver, TimeTermination};
///
/// let solver: Solver<_, Option<TimeTermination>, _, _> = Solver::new((
///     construction_phase,
///     local_search_phase,
/// )).with_termination(TimeTermination::seconds(30));
///
/// let result = solver.solve(director);
/// ```
pub struct Solver<P, T, S, D> {
    phases: P,
    termination: T,
    terminate_early_flag: Arc<AtomicBool>,
    solving: Arc<AtomicBool>,
    config: Option<SolverConfig>,
    _phantom: PhantomData<fn(S, D)>,
}

impl<P: Debug, T: Debug, S, D> Debug for Solver<P, T, S, D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Solver")
            .field("phases", &self.phases)
            .field("termination", &self.termination)
            .finish()
    }
}

impl<P, S, D> Solver<P, NoTermination, S, D>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
{
    /// Creates a new solver with the given phases tuple and no termination.
    pub fn new(phases: P) -> Self {
        Solver {
            phases,
            termination: NoTermination,
            terminate_early_flag: Arc::new(AtomicBool::new(false)),
            solving: Arc::new(AtomicBool::new(false)),
            config: None,
            _phantom: PhantomData,
        }
    }

    /// Sets the termination condition.
    pub fn with_termination<T>(self, termination: T) -> Solver<P, Option<T>, S, D> {
        Solver {
            phases: self.phases,
            termination: Some(termination),
            terminate_early_flag: self.terminate_early_flag,
            solving: self.solving,
            config: self.config,
            _phantom: PhantomData,
        }
    }
}

impl<P, T, S, D> Solver<P, T, S, D>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
{
    /// Sets configuration.
    pub fn with_config(mut self, config: SolverConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Requests early termination of the solving process.
    ///
    /// This method is thread-safe and can be called from another thread.
    pub fn terminate_early(&self) -> bool {
        if self.solving.load(Ordering::SeqCst) {
            self.terminate_early_flag.store(true, Ordering::SeqCst);
            true
        } else {
            false
        }
    }

    /// Returns true if the solver is currently solving.
    pub fn is_solving(&self) -> bool {
        self.solving.load(Ordering::SeqCst)
    }

    /// Returns the configuration if set.
    pub fn config(&self) -> Option<&SolverConfig> {
        self.config.as_ref()
    }
}

/// Marker type indicating no termination.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoTermination;

/// Marker trait for termination types that can be used in Solver.
pub trait MaybeTermination<S: PlanningSolution, D: ScoreDirector<S>>: Send {
    /// Checks if the solver should terminate.
    fn should_terminate(&self, solver_scope: &SolverScope<S, D>) -> bool;
}

impl<S: PlanningSolution, D: ScoreDirector<S>, T: Termination<S, D>> MaybeTermination<S, D>
    for Option<T>
{
    fn should_terminate(&self, solver_scope: &SolverScope<S, D>) -> bool {
        match self {
            Some(t) => t.is_terminated(solver_scope),
            None => false,
        }
    }
}

impl<S: PlanningSolution, D: ScoreDirector<S>> MaybeTermination<S, D> for NoTermination {
    fn should_terminate(&self, _solver_scope: &SolverScope<S, D>) -> bool {
        false
    }
}

macro_rules! impl_solver {
    ($($idx:tt: $P:ident),+) => {
        impl<S, D, T, $($P),+> Solver<($($P,)+), T, S, D>
        where
            S: PlanningSolution,
            D: ScoreDirector<S>,
            T: MaybeTermination<S, D>,
            $($P: Phase<S, D>,)+
        {
            /// Solves using the provided score director.
            pub fn solve(&mut self, score_director: D) -> S {
                self.solving.store(true, Ordering::SeqCst);
                self.terminate_early_flag.store(false, Ordering::SeqCst);

                let mut solver_scope = SolverScope::new(score_director);
                solver_scope.set_terminate_early_flag(self.terminate_early_flag.clone());
                solver_scope.start_solving();

                // Execute phases with termination checking
                $(
                    if !self.check_termination(&solver_scope) {
                        tracing::debug!(
                            "Starting phase {} ({})",
                            $idx,
                            self.phases.$idx.phase_type_name()
                        );
                        self.phases.$idx.solve(&mut solver_scope);
                        tracing::debug!(
                            "Finished phase {} ({}) with score {:?}",
                            $idx,
                            self.phases.$idx.phase_type_name(),
                            solver_scope.best_score()
                        );
                    }
                )+

                self.solving.store(false, Ordering::SeqCst);
                solver_scope.take_best_or_working_solution()
            }

            fn check_termination(&self, solver_scope: &SolverScope<S, D>) -> bool {
                if self.terminate_early_flag.load(Ordering::SeqCst) {
                    return true;
                }
                self.termination.should_terminate(solver_scope)
            }
        }
    };
}

impl_solver!(0: P0);
impl_solver!(0: P0, 1: P1);
impl_solver!(0: P0, 1: P1, 2: P2);
impl_solver!(0: P0, 1: P1, 2: P2, 3: P3);
impl_solver!(0: P0, 1: P1, 2: P2, 3: P3, 4: P4);
impl_solver!(0: P0, 1: P1, 2: P2, 3: P3, 4: P4, 5: P5);
impl_solver!(0: P0, 1: P1, 2: P2, 3: P3, 4: P4, 5: P5, 6: P6);
impl_solver!(0: P0, 1: P1, 2: P2, 3: P3, 4: P4, 5: P5, 6: P6, 7: P7);

/// Solver status enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolverStatus {
    /// Solver is not currently solving.
    NotSolving,
    /// Solver is initializing.
    SolvingScheduled,
    /// Solver is actively solving.
    SolvingActive,
}
