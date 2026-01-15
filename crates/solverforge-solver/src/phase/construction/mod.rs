//! Construction heuristic phase
//!
//! Builds an initial solution by assigning values to uninitialized
//! planning variables one at a time.

mod forager;
mod phase;
mod placer;

pub use forager::{
    BestFitForager, ConstructionForager, FirstFeasibleForager, FirstFitForager,
    StrongestFitForager, WeakestFitForager,
};
pub use phase::ConstructionHeuristicPhase;
pub use placer::{EntityPlacer, Placement, QueuedEntityPlacer, SortedEntityPlacer};

/// Construction heuristic phase configuration.
#[derive(Debug, Clone)]
pub struct ConstructionHeuristicConfig {
    /// The forager type to use.
    pub forager_type: ForagerType,
}

impl Default for ConstructionHeuristicConfig {
    fn default() -> Self {
        Self {
            forager_type: ForagerType::FirstFit,
        }
    }
}

/// Type of forager to use in construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForagerType {
    /// Accept the first feasible move.
    FirstFit,
    /// Evaluate all moves and pick the best.
    BestFit,
}
