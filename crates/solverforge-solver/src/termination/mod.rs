//! Termination conditions for solver phases.

pub(crate) mod best_score;
pub(crate) mod composite;
pub(crate) mod step_count;
pub(crate) mod time;
pub(crate) mod traits;
pub(crate) mod unimproved;

pub(crate) use best_score::BestScoreTermination;
pub(crate) use composite::OrTermination;
pub(crate) use step_count::StepCountTermination;
pub(crate) use time::TimeTermination;
pub(crate) use traits::Termination;
pub(crate) use unimproved::{UnimprovedStepCountTermination, UnimprovedTimeTermination};

#[cfg(test)]
mod tests;
