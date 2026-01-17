//! Solver phases for different solving strategies
//!
//! Phases are the main building blocks of solving:
//! - ConstructionHeuristicPhase: Builds an initial solution
//! - LocalSearchPhase: Improves an existing solution
//! - ExhaustiveSearchPhase: Explores entire solution space
//! - PartitionedSearchPhase: Parallel solving via partitioning
//! - VndPhase: Variable Neighborhood Descent

pub mod construction;
pub mod exhaustive;
pub mod localsearch;
pub mod partitioned;
pub mod phase_impl;
mod traits;
pub mod vnd;

pub use phase_impl::{ListPhaseImpl, PhaseFactory, PhaseSequence};
pub use traits::Phase;
