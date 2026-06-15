/* Phase factory for creating phases from configuration.

Phase factories create fresh phase instances for each solve, ensuring
clean state between solves. This is essential because phases maintain
internal state (like step counters, tabu lists, or temperature values)
that must be reset for each new solve.

# Overview

This module provides two main factories:

- [`ConstructionPhaseFactory`]: Creates construction heuristic phases
- [`LocalSearchPhaseFactory`]: Creates local search phases

# Usage Pattern

Phase factories work with the zero-erasure architecture where all types
flow through generics. See the individual factory types for usage details.
*/

mod construction;
mod distance_arithmetic;
mod k_opt;
mod list_clarke_wright;
mod list_construction;
mod list_k_opt;
mod local_search;

pub use construction::ConstructionPhaseFactory;
pub use k_opt::{KOptPhase, KOptPhaseBuilder};
pub use list_clarke_wright::ListClarkeWrightPhase;
pub(crate) use list_construction::solve_specialized_list_construction;
pub use list_construction::{
    ListCheapestInsertionPhase, ListConstructionPhase, ListConstructionPhaseBuilder,
    ListRegretInsertionPhase,
};
pub use list_k_opt::ListKOptPhase;
pub use local_search::LocalSearchPhaseFactory;
