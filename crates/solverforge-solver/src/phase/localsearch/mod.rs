//! Local search phase
//!
//! Improves an existing solution by iteratively applying moves
//! that are accepted according to an acceptance criterion.

pub(crate) mod acceptor;
pub(crate) mod cursor_source;
mod evaluation;
pub(crate) mod forager;
mod parallel;
pub(crate) mod phase;
pub(crate) mod vnd;

pub(crate) use acceptor::TabuSearchPolicy;
pub(crate) use acceptor::{
    Acceptor, DiversifiedLateAcceptanceAcceptor, GreatDelugeAcceptor, HardRegressionPolicy,
    HillClimbingAcceptor, LateAcceptanceAcceptor, SimulatedAnnealingAcceptor,
    SimulatedAnnealingCalibration, StepCountingHillClimbingAcceptor, TabuSearchAcceptor,
};
pub(crate) use cursor_source::MoveCursorSource;
#[doc(hidden)]
pub(crate) use cursor_source::SelectorCursorSource;
pub(crate) use forager::{
    AcceptedCountForager, BestScoreForager, FirstAcceptedForager, FirstBestScoreImprovingForager,
    FirstLastStepScoreImprovingForager, ForagerDecision, LocalSearchForager,
};
pub(crate) use parallel::resolve_evaluator_count;
pub(crate) use phase::solve_local_search_with_resources;
pub(crate) use phase::LocalSearchPhase;
