/* Local search phase

Improves an existing solution by iteratively applying moves
that are accepted according to an acceptance criterion.
*/

mod acceptor;
mod cursor_source;
mod evaluation;
mod forager;
mod phase;
pub(crate) mod vnd;

pub(crate) use acceptor::TabuSearchPolicy;
pub use acceptor::{
    Acceptor, DiversifiedLateAcceptanceAcceptor, GreatDelugeAcceptor, HardRegressionPolicy,
    HillClimbingAcceptor, LateAcceptanceAcceptor, SimulatedAnnealingAcceptor,
    SimulatedAnnealingCalibration, StepCountingHillClimbingAcceptor, TabuSearchAcceptor,
};
pub use cursor_source::MoveCursorSource;
#[doc(hidden)]
pub use cursor_source::SelectorCursorSource;
pub use forager::{
    AcceptedCountForager, BestScoreForager, FirstAcceptedForager, FirstBestScoreImprovingForager,
    FirstLastStepScoreImprovingForager, ForagerDecision, LocalSearchForager,
};
pub(crate) use phase::solve_local_search_with_resources;
pub use phase::LocalSearchPhase;
