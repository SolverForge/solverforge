//! Monomorphic enum for local search acceptors.
//!
//! Provides zero-erasure dispatch over all acceptor variants.

use std::fmt::Debug;

use solverforge_config::AcceptorConfig;
use solverforge_core::domain::PlanningSolution;

use super::acceptor::{
    Acceptor, DiversifiedLateAcceptanceAcceptor, EntityTabuAcceptor, GreatDelugeAcceptor,
    HillClimbingAcceptor, LateAcceptanceAcceptor, MoveTabuAcceptor, SimulatedAnnealingAcceptor,
    StepCountingHillClimbingAcceptor, TabuSearchAcceptor, ValueTabuAcceptor,
};

/// Monomorphic enum wrapping all local search acceptor implementations.
pub enum AcceptorImpl<S: PlanningSolution> {
    HillClimbing(HillClimbingAcceptor<S>),
    SimulatedAnnealing(SimulatedAnnealingAcceptor<S>),
    LateAcceptance(LateAcceptanceAcceptor<S>),
    TabuSearch(TabuSearchAcceptor<S>),
    GreatDeluge(GreatDelugeAcceptor<S>),
    DiversifiedLateAcceptance(DiversifiedLateAcceptanceAcceptor<S>),
    StepCountingHillClimbing(StepCountingHillClimbingAcceptor<S>),
    EntityTabu(EntityTabuAcceptor<S>),
    MoveTabu(MoveTabuAcceptor<S>),
    ValueTabu(ValueTabuAcceptor<S>),
}

impl<S: PlanningSolution> AcceptorImpl<S> {
    pub fn from_config(config: &AcceptorConfig) -> Self {
        match config {
            AcceptorConfig::HillClimbing => AcceptorImpl::HillClimbing(HillClimbingAcceptor::new()),
            AcceptorConfig::SimulatedAnnealing(sa) => {
                let temp = parse_temperature(&sa.starting_temperature);
                let decay = sa.decay_rate.unwrap_or(0.99);
                AcceptorImpl::SimulatedAnnealing(SimulatedAnnealingAcceptor::new(temp, decay))
            }
            AcceptorConfig::LateAcceptance(la) => {
                AcceptorImpl::LateAcceptance(LateAcceptanceAcceptor::new(
                    la.late_acceptance_size.unwrap_or(400),
                ))
            }
            AcceptorConfig::TabuSearch(ts) => {
                let size = ts
                    .entity_tabu_size
                    .or(ts.move_tabu_size)
                    .or(ts.value_tabu_size)
                    .unwrap_or(7);
                AcceptorImpl::TabuSearch(TabuSearchAcceptor::new(size))
            }
            AcceptorConfig::GreatDeluge(gd) => AcceptorImpl::GreatDeluge(GreatDelugeAcceptor::new(
                gd.water_level_increase_ratio.unwrap_or(0.0000001),
            )),
        }
    }

    pub fn late_acceptance() -> Self {
        AcceptorImpl::LateAcceptance(LateAcceptanceAcceptor::new(400))
    }
}

fn parse_temperature(s: &Option<String>) -> f64 {
    s.as_ref()
        .and_then(|s| s.trim_end_matches("hard").parse().ok())
        .unwrap_or(1.0)
}

impl<S: PlanningSolution> Debug for AcceptorImpl<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HillClimbing(_) => write!(f, "HillClimbing"),
            Self::SimulatedAnnealing(_) => write!(f, "SimulatedAnnealing"),
            Self::LateAcceptance(_) => write!(f, "LateAcceptance"),
            Self::TabuSearch(_) => write!(f, "TabuSearch"),
            Self::GreatDeluge(_) => write!(f, "GreatDeluge"),
            Self::DiversifiedLateAcceptance(_) => write!(f, "DiversifiedLateAcceptance"),
            Self::StepCountingHillClimbing(_) => write!(f, "StepCountingHillClimbing"),
            Self::EntityTabu(_) => write!(f, "EntityTabu"),
            Self::MoveTabu(_) => write!(f, "MoveTabu"),
            Self::ValueTabu(_) => write!(f, "ValueTabu"),
        }
    }
}

impl<S: PlanningSolution> Acceptor<S> for AcceptorImpl<S> {
    fn is_accepted(&self, last_step_score: &S::Score, move_score: &S::Score) -> bool {
        match self {
            Self::HillClimbing(a) => a.is_accepted(last_step_score, move_score),
            Self::SimulatedAnnealing(a) => a.is_accepted(last_step_score, move_score),
            Self::LateAcceptance(a) => a.is_accepted(last_step_score, move_score),
            Self::TabuSearch(a) => a.is_accepted(last_step_score, move_score),
            Self::GreatDeluge(a) => a.is_accepted(last_step_score, move_score),
            Self::DiversifiedLateAcceptance(a) => a.is_accepted(last_step_score, move_score),
            Self::StepCountingHillClimbing(a) => a.is_accepted(last_step_score, move_score),
            Self::EntityTabu(a) => a.is_accepted(last_step_score, move_score),
            Self::MoveTabu(a) => a.is_accepted(last_step_score, move_score),
            Self::ValueTabu(a) => a.is_accepted(last_step_score, move_score),
        }
    }

    fn phase_started(&mut self, initial_score: &S::Score) {
        match self {
            Self::HillClimbing(a) => a.phase_started(initial_score),
            Self::SimulatedAnnealing(a) => a.phase_started(initial_score),
            Self::LateAcceptance(a) => a.phase_started(initial_score),
            Self::TabuSearch(a) => a.phase_started(initial_score),
            Self::GreatDeluge(a) => a.phase_started(initial_score),
            Self::DiversifiedLateAcceptance(a) => a.phase_started(initial_score),
            Self::StepCountingHillClimbing(a) => a.phase_started(initial_score),
            Self::EntityTabu(a) => a.phase_started(initial_score),
            Self::MoveTabu(a) => a.phase_started(initial_score),
            Self::ValueTabu(a) => a.phase_started(initial_score),
        }
    }

    fn phase_ended(&mut self) {
        match self {
            Self::HillClimbing(a) => a.phase_ended(),
            Self::SimulatedAnnealing(a) => a.phase_ended(),
            Self::LateAcceptance(a) => a.phase_ended(),
            Self::TabuSearch(a) => a.phase_ended(),
            Self::GreatDeluge(a) => a.phase_ended(),
            Self::DiversifiedLateAcceptance(a) => a.phase_ended(),
            Self::StepCountingHillClimbing(a) => a.phase_ended(),
            Self::EntityTabu(a) => a.phase_ended(),
            Self::MoveTabu(a) => a.phase_ended(),
            Self::ValueTabu(a) => a.phase_ended(),
        }
    }

    fn step_started(&mut self) {
        match self {
            Self::HillClimbing(a) => a.step_started(),
            Self::SimulatedAnnealing(a) => a.step_started(),
            Self::LateAcceptance(a) => a.step_started(),
            Self::TabuSearch(a) => a.step_started(),
            Self::GreatDeluge(a) => a.step_started(),
            Self::DiversifiedLateAcceptance(a) => a.step_started(),
            Self::StepCountingHillClimbing(a) => a.step_started(),
            Self::EntityTabu(a) => a.step_started(),
            Self::MoveTabu(a) => a.step_started(),
            Self::ValueTabu(a) => a.step_started(),
        }
    }

    fn step_ended(&mut self, step_score: &S::Score) {
        match self {
            Self::HillClimbing(a) => a.step_ended(step_score),
            Self::SimulatedAnnealing(a) => a.step_ended(step_score),
            Self::LateAcceptance(a) => a.step_ended(step_score),
            Self::TabuSearch(a) => a.step_ended(step_score),
            Self::GreatDeluge(a) => a.step_ended(step_score),
            Self::DiversifiedLateAcceptance(a) => a.step_ended(step_score),
            Self::StepCountingHillClimbing(a) => a.step_ended(step_score),
            Self::EntityTabu(a) => a.step_ended(step_score),
            Self::MoveTabu(a) => a.step_ended(step_score),
            Self::ValueTabu(a) => a.step_ended(step_score),
        }
    }
}
