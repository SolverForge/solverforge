/* Solver phases for different solving strategies

Phases are the main building blocks of solving:
- ConstructionHeuristicPhase: Builds an initial solution
- LocalSearchPhase: Improves an existing solution
- ExhaustiveSearchPhase: Explores entire solution space
- PartitionedSearchPhase: Parallel solving via partitioning
- VndPhase: Variable Neighborhood Descent
*/

pub mod construction;
pub(crate) mod control;
pub mod dynamic_vnd;
pub mod exhaustive;
mod hard_delta;
#[cfg(test)]
mod hard_delta_phase_tests;
#[cfg(test)]
mod hard_delta_tests;
pub mod localsearch;
pub mod partitioned;
pub mod sequence;
mod traits;
mod tuple_impl;
pub mod vnd;

pub use dynamic_vnd::DynamicVndPhase;
pub use sequence::PhaseSequence;
pub use traits::Phase;
