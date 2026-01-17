//! Acceptors for local search move acceptance.
//!
//! Acceptors determine whether a move should be accepted based on
//! comparing the resulting score with the previous score.
//!
//! # Zero-Erasure Design
//!
//! This module uses [`AcceptorImpl`] enum for runtime acceptor selection
//! without type erasure (no `Box<dyn>`, no `Arc`). The compiler generates
//! code paths for all acceptor variants at compile time.

mod diversified_late_acceptance;
mod entity_tabu;
mod great_deluge;
mod hill_climbing;
mod late_acceptance;
mod move_tabu;
mod simulated_annealing;
mod step_counting;
mod tabu_search;
mod value_tabu;

use std::fmt::Debug;

use solverforge_config::AcceptorConfig;
use solverforge_core::domain::PlanningSolution;

pub use diversified_late_acceptance::DiversifiedLateAcceptanceAcceptor;
pub use entity_tabu::EntityTabuAcceptor;
pub use great_deluge::GreatDelugeAcceptor;
pub use hill_climbing::HillClimbingAcceptor;
pub use late_acceptance::LateAcceptanceAcceptor;
pub use move_tabu::MoveTabuAcceptor;
pub use simulated_annealing::SimulatedAnnealingAcceptor;
pub use step_counting::StepCountingHillClimbingAcceptor;
pub use tabu_search::TabuSearchAcceptor;
pub use value_tabu::ValueTabuAcceptor;

/// Trait for accepting or rejecting moves in local search.
///
/// Acceptors implement different strategies for escaping local optima,
/// such as hill climbing, simulated annealing, or tabu search.
pub trait Acceptor<S: PlanningSolution>: Send + Debug {
    /// Returns true if a move resulting in `move_score` should be accepted,
    /// given the previous step's score.
    fn is_accepted(&self, last_step_score: &S::Score, move_score: &S::Score) -> bool;

    /// Called when a phase starts.
    fn phase_started(&mut self, _initial_score: &S::Score) {}

    /// Called when a phase ends.
    fn phase_ended(&mut self) {}

    /// Called when a step starts.
    fn step_started(&mut self) {}

    /// Called when a step ends with an accepted move.
    fn step_ended(&mut self, _step_score: &S::Score) {}
}

/// Monomorphic acceptor enum - runtime selection without type erasure.
///
/// This enum wraps all 10 acceptor types, enabling runtime configuration
/// selection while preserving concrete types throughout the solver pipeline.
///
/// # Example
///
/// ```
/// use solverforge_solver::phase::localsearch::AcceptorImpl;
/// use solverforge_core::score::SimpleScore;
/// use solverforge_core::domain::PlanningSolution;
///
/// #[derive(Clone)]
/// struct MySolution;
/// impl PlanningSolution for MySolution {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { None }
///     fn set_score(&mut self, _: Option<Self::Score>) {}
/// }
///
/// // Create from default (late acceptance)
/// let acceptor: AcceptorImpl<MySolution> = AcceptorImpl::default();
///
/// // Or explicitly
/// let hill = AcceptorImpl::<MySolution>::hill_climbing();
/// ```
pub enum AcceptorImpl<S: PlanningSolution> {
    /// Hill climbing - accepts only improving moves.
    HillClimbing(HillClimbingAcceptor),
    /// Late acceptance - compares against historical scores.
    LateAcceptance(LateAcceptanceAcceptor<S>),
    /// Tabu search - forbids recently visited solutions.
    TabuSearch(TabuSearchAcceptor<S>),
    /// Simulated annealing - temperature-based acceptance probability.
    SimulatedAnnealing(SimulatedAnnealingAcceptor),
    /// Great deluge - water level acceptance threshold.
    GreatDeluge(GreatDelugeAcceptor<S>),
    /// Step counting hill climbing - limited plateau exploration.
    StepCountingHillClimbing(StepCountingHillClimbingAcceptor<S>),
    /// Diversified late acceptance - late acceptance with best-score tolerance.
    DiversifiedLateAcceptance(DiversifiedLateAcceptanceAcceptor<S>),
    /// Entity tabu - forbids recently moved entities.
    EntityTabu(EntityTabuAcceptor),
    /// Move tabu - forbids recently executed moves.
    MoveTabu(MoveTabuAcceptor),
    /// Value tabu - forbids recently assigned values.
    ValueTabu(ValueTabuAcceptor),
}

impl<S: PlanningSolution> AcceptorImpl<S> {
    /// Creates a hill climbing acceptor.
    pub fn hill_climbing() -> Self {
        Self::HillClimbing(HillClimbingAcceptor::new())
    }

    /// Creates a late acceptance acceptor with the given history size.
    pub fn late_acceptance(size: usize) -> Self {
        Self::LateAcceptance(LateAcceptanceAcceptor::new(size))
    }

    /// Creates a tabu search acceptor with the given tabu size.
    pub fn tabu_search(tabu_size: usize) -> Self {
        Self::TabuSearch(TabuSearchAcceptor::new(tabu_size))
    }

    /// Creates a simulated annealing acceptor.
    pub fn simulated_annealing(starting_temp: f64, decay_rate: f64) -> Self {
        Self::SimulatedAnnealing(SimulatedAnnealingAcceptor::new(starting_temp, decay_rate))
    }

    /// Creates a great deluge acceptor with the given rain speed.
    pub fn great_deluge(rain_speed: f64) -> Self {
        Self::GreatDeluge(GreatDelugeAcceptor::new(rain_speed))
    }

    /// Creates a step counting hill climbing acceptor.
    pub fn step_counting_hill_climbing(step_count_limit: u64) -> Self {
        Self::StepCountingHillClimbing(StepCountingHillClimbingAcceptor::new(step_count_limit))
    }

    /// Creates a diversified late acceptance acceptor.
    pub fn diversified_late_acceptance(late_acceptance_size: usize, tolerance: f64) -> Self {
        Self::DiversifiedLateAcceptance(DiversifiedLateAcceptanceAcceptor::new(
            late_acceptance_size,
            tolerance,
        ))
    }

    /// Creates an entity tabu acceptor.
    pub fn entity_tabu(entity_tabu_size: usize) -> Self {
        Self::EntityTabu(EntityTabuAcceptor::new(entity_tabu_size))
    }

    /// Creates a move tabu acceptor.
    pub fn move_tabu(move_tabu_size: usize) -> Self {
        Self::MoveTabu(MoveTabuAcceptor::new(move_tabu_size))
    }

    /// Creates a value tabu acceptor.
    pub fn value_tabu(value_tabu_size: usize) -> Self {
        Self::ValueTabu(ValueTabuAcceptor::new(value_tabu_size))
    }

    /// Creates an acceptor from configuration.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::phase::localsearch::AcceptorImpl;
    /// use solverforge_config::AcceptorConfig;
    /// use solverforge_core::score::SimpleScore;
    /// use solverforge_core::domain::PlanningSolution;
    ///
    /// #[derive(Clone)]
    /// struct MySolution;
    /// impl PlanningSolution for MySolution {
    ///     type Score = SimpleScore;
    ///     fn score(&self) -> Option<Self::Score> { None }
    ///     fn set_score(&mut self, _: Option<Self::Score>) {}
    /// }
    ///
    /// let config = AcceptorConfig::HillClimbing;
    /// let acceptor = AcceptorImpl::<MySolution>::from_config(Some(&config));
    /// ```
    pub fn from_config(config: Option<&AcceptorConfig>) -> Self {
        match config {
            Some(AcceptorConfig::HillClimbing) => Self::hill_climbing(),
            Some(AcceptorConfig::LateAcceptance(cfg)) => {
                let size = cfg.late_acceptance_size.unwrap_or(400);
                Self::late_acceptance(size)
            }
            Some(AcceptorConfig::TabuSearch(cfg)) => {
                let size = cfg.entity_tabu_size.unwrap_or(7);
                Self::tabu_search(size)
            }
            Some(AcceptorConfig::SimulatedAnnealing(_cfg)) => {
                Self::simulated_annealing(1.0, 0.99)
            }
            Some(AcceptorConfig::GreatDeluge(cfg)) => {
                let ratio = cfg.water_level_increase_ratio.unwrap_or(0.001);
                Self::great_deluge(ratio)
            }
            Some(AcceptorConfig::StepCountingHillClimbing(cfg)) => {
                let limit = cfg.step_count_limit.unwrap_or(100) as u64;
                Self::step_counting_hill_climbing(limit)
            }
            Some(AcceptorConfig::DiversifiedLateAcceptance(cfg)) => {
                let la_size = cfg.late_acceptance_size.unwrap_or(400);
                let pct = cfg.diversity_minimum_percentage.unwrap_or(5);
                let tolerance = f64::from(pct) / 100.0;
                Self::diversified_late_acceptance(la_size, tolerance)
            }
            Some(AcceptorConfig::EntityTabu(cfg)) => {
                let size = cfg.entity_tabu_size.unwrap_or(7);
                Self::entity_tabu(size)
            }
            Some(AcceptorConfig::MoveTabu(cfg)) => {
                let size = cfg.move_tabu_size.unwrap_or(7);
                Self::move_tabu(size)
            }
            Some(AcceptorConfig::ValueTabu(cfg)) => {
                let size = cfg.value_tabu_size.unwrap_or(7);
                Self::value_tabu(size)
            }
            None => Self::default(),
        }
    }
}

impl<S: PlanningSolution> Default for AcceptorImpl<S> {
    fn default() -> Self {
        Self::late_acceptance(400)
    }
}

impl<S: PlanningSolution> Debug for AcceptorImpl<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HillClimbing(a) => a.fmt(f),
            Self::LateAcceptance(a) => a.fmt(f),
            Self::TabuSearch(a) => a.fmt(f),
            Self::SimulatedAnnealing(a) => a.fmt(f),
            Self::GreatDeluge(a) => a.fmt(f),
            Self::StepCountingHillClimbing(a) => a.fmt(f),
            Self::DiversifiedLateAcceptance(a) => a.fmt(f),
            Self::EntityTabu(a) => a.fmt(f),
            Self::MoveTabu(a) => a.fmt(f),
            Self::ValueTabu(a) => a.fmt(f),
        }
    }
}

impl<S: PlanningSolution> Acceptor<S> for AcceptorImpl<S> {
    fn is_accepted(&self, last_step_score: &S::Score, move_score: &S::Score) -> bool {
        match self {
            Self::HillClimbing(a) => {
                <HillClimbingAcceptor as Acceptor<S>>::is_accepted(a, last_step_score, move_score)
            }
            Self::LateAcceptance(a) => a.is_accepted(last_step_score, move_score),
            Self::TabuSearch(a) => a.is_accepted(last_step_score, move_score),
            Self::SimulatedAnnealing(a) => {
                <SimulatedAnnealingAcceptor as Acceptor<S>>::is_accepted(
                    a,
                    last_step_score,
                    move_score,
                )
            }
            Self::GreatDeluge(a) => a.is_accepted(last_step_score, move_score),
            Self::StepCountingHillClimbing(a) => a.is_accepted(last_step_score, move_score),
            Self::DiversifiedLateAcceptance(a) => a.is_accepted(last_step_score, move_score),
            Self::EntityTabu(a) => {
                <EntityTabuAcceptor as Acceptor<S>>::is_accepted(a, last_step_score, move_score)
            }
            Self::MoveTabu(a) => {
                <MoveTabuAcceptor as Acceptor<S>>::is_accepted(a, last_step_score, move_score)
            }
            Self::ValueTabu(a) => {
                <ValueTabuAcceptor as Acceptor<S>>::is_accepted(a, last_step_score, move_score)
            }
        }
    }

    fn phase_started(&mut self, initial_score: &S::Score) {
        match self {
            Self::HillClimbing(a) => {
                <HillClimbingAcceptor as Acceptor<S>>::phase_started(a, initial_score)
            }
            Self::LateAcceptance(a) => a.phase_started(initial_score),
            Self::TabuSearch(a) => a.phase_started(initial_score),
            Self::SimulatedAnnealing(a) => {
                <SimulatedAnnealingAcceptor as Acceptor<S>>::phase_started(a, initial_score)
            }
            Self::GreatDeluge(a) => a.phase_started(initial_score),
            Self::StepCountingHillClimbing(a) => a.phase_started(initial_score),
            Self::DiversifiedLateAcceptance(a) => a.phase_started(initial_score),
            Self::EntityTabu(a) => {
                <EntityTabuAcceptor as Acceptor<S>>::phase_started(a, initial_score)
            }
            Self::MoveTabu(a) => {
                <MoveTabuAcceptor as Acceptor<S>>::phase_started(a, initial_score)
            }
            Self::ValueTabu(a) => {
                <ValueTabuAcceptor as Acceptor<S>>::phase_started(a, initial_score)
            }
        }
    }

    fn phase_ended(&mut self) {
        match self {
            Self::HillClimbing(a) => <HillClimbingAcceptor as Acceptor<S>>::phase_ended(a),
            Self::LateAcceptance(a) => a.phase_ended(),
            Self::TabuSearch(a) => a.phase_ended(),
            Self::SimulatedAnnealing(a) => {
                <SimulatedAnnealingAcceptor as Acceptor<S>>::phase_ended(a)
            }
            Self::GreatDeluge(a) => a.phase_ended(),
            Self::StepCountingHillClimbing(a) => a.phase_ended(),
            Self::DiversifiedLateAcceptance(a) => a.phase_ended(),
            Self::EntityTabu(a) => <EntityTabuAcceptor as Acceptor<S>>::phase_ended(a),
            Self::MoveTabu(a) => <MoveTabuAcceptor as Acceptor<S>>::phase_ended(a),
            Self::ValueTabu(a) => <ValueTabuAcceptor as Acceptor<S>>::phase_ended(a),
        }
    }

    fn step_started(&mut self) {
        match self {
            Self::HillClimbing(a) => <HillClimbingAcceptor as Acceptor<S>>::step_started(a),
            Self::LateAcceptance(a) => a.step_started(),
            Self::TabuSearch(a) => a.step_started(),
            Self::SimulatedAnnealing(a) => {
                <SimulatedAnnealingAcceptor as Acceptor<S>>::step_started(a)
            }
            Self::GreatDeluge(a) => a.step_started(),
            Self::StepCountingHillClimbing(a) => a.step_started(),
            Self::DiversifiedLateAcceptance(a) => a.step_started(),
            Self::EntityTabu(a) => <EntityTabuAcceptor as Acceptor<S>>::step_started(a),
            Self::MoveTabu(a) => <MoveTabuAcceptor as Acceptor<S>>::step_started(a),
            Self::ValueTabu(a) => <ValueTabuAcceptor as Acceptor<S>>::step_started(a),
        }
    }

    fn step_ended(&mut self, step_score: &S::Score) {
        match self {
            Self::HillClimbing(a) => {
                <HillClimbingAcceptor as Acceptor<S>>::step_ended(a, step_score)
            }
            Self::LateAcceptance(a) => a.step_ended(step_score),
            Self::TabuSearch(a) => a.step_ended(step_score),
            Self::SimulatedAnnealing(a) => {
                <SimulatedAnnealingAcceptor as Acceptor<S>>::step_ended(a, step_score)
            }
            Self::GreatDeluge(a) => a.step_ended(step_score),
            Self::StepCountingHillClimbing(a) => a.step_ended(step_score),
            Self::DiversifiedLateAcceptance(a) => a.step_ended(step_score),
            Self::EntityTabu(a) => {
                <EntityTabuAcceptor as Acceptor<S>>::step_ended(a, step_score)
            }
            Self::MoveTabu(a) => <MoveTabuAcceptor as Acceptor<S>>::step_ended(a, step_score),
            Self::ValueTabu(a) => <ValueTabuAcceptor as Acceptor<S>>::step_ended(a, step_score),
        }
    }
}

#[cfg(test)]
mod tests;
