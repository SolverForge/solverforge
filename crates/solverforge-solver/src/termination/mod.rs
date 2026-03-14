// Termination conditions for solver phases.

mod best_score;
mod composite;
mod diminished_returns;
mod move_count;
mod score_calculation_count;
mod step_count;
mod time;
mod traits;
mod unimproved;

pub use best_score::{BestScoreFeasibleTermination, BestScoreTermination};
pub use composite::{AndTermination, OrTermination};
pub use diminished_returns::DiminishedReturnsTermination;
pub use move_count::MoveCountTermination;
pub use score_calculation_count::ScoreCalculationCountTermination;
pub use step_count::StepCountTermination;
pub use time::TimeTermination;
pub use traits::Termination;
pub use unimproved::{UnimprovedStepCountTermination, UnimprovedTimeTermination};

#[cfg(test)]
mod tests;
