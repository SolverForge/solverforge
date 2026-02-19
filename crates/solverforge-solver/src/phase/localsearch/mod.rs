//! Local search phase
//!
//! Improves an existing solution by iteratively applying moves
//! that are accepted according to an acceptance criterion.

mod acceptor;
mod forager;
mod phase;

pub use acceptor::{
    Acceptor, DiversifiedLateAcceptanceAcceptor, EntityTabuAcceptor, GreatDelugeAcceptor,
    HillClimbingAcceptor, LateAcceptanceAcceptor, MoveTabuAcceptor, SimulatedAnnealingAcceptor,
    StepCountingHillClimbingAcceptor, TabuSearchAcceptor, ValueTabuAcceptor,
};
pub use forager::{
    AcceptedCountForager, BestScoreForager, FirstAcceptedForager, FirstBestScoreImprovingForager,
    FirstLastStepScoreImprovingForager, LocalSearchForager,
};
pub use phase::LocalSearchPhase;

/// Local search phase configuration.
#[derive(Debug, Clone)]
pub struct LocalSearchConfig {
    /// The acceptor type to use.
    pub acceptor_type: AcceptorType,
    /// Maximum number of steps (None = unlimited).
    pub step_limit: Option<u64>,
    /// Number of accepted moves to collect before quitting early.
    pub accepted_count_limit: Option<usize>,
}

impl Default for LocalSearchConfig {
    fn default() -> Self {
        Self {
            acceptor_type: AcceptorType::HillClimbing,
            step_limit: Some(1000),
            accepted_count_limit: Some(1),
        }
    }
}

/// Type of acceptor to use in local search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcceptorType {
    /// Accept only improving moves.
    HillClimbing,
    /// Accept moves with probability based on temperature.
    SimulatedAnnealing,
}
