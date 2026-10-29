/* Solver phases for different solving strategies

Phases are the main building blocks of solving:
- ConstructionHeuristicPhase: Builds an initial solution
- LocalSearchPhase: Improves an existing solution, including Variable Neighborhood Descent
- ExhaustiveSearchPhase: Explores entire solution space
- PartitionedSearchPhase: Parallel solving via partitioning
*/

pub(crate) mod construction;
pub(crate) mod control;
pub(crate) mod exhaustive;
mod hard_delta;
#[cfg(test)]
mod hard_delta_phase_tests;
#[cfg(test)]
mod hard_delta_tests;
pub(crate) mod localsearch;
pub(crate) mod partitioned;
pub(crate) mod traits;

pub(crate) use traits::Phase;
