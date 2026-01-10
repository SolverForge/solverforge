//! Phase factory for creating phases from configuration.
//!
//! Phase factories create fresh phase instances for each solve, ensuring
//! clean state between solves. This is essential because phases maintain
//! internal state (like step counters, tabu lists, or temperature values)
//! that must be reset for each new solve.
//!
//! # Overview
//!
//! This module provides factories for creating phases with full type preservation
//! (zero type erasure):
//!
//! - [`ConstructionPhaseFactory`]: Creates construction heuristic phases
//! - [`LocalSearchPhaseFactory`]: Creates local search phases
//! - [`BasicConstructionPhaseBuilder`]: Simple round-robin construction
//! - [`BasicLocalSearchPhaseBuilder`]: Simple late-acceptance local search
//! - [`ListConstructionPhaseBuilder`]: List variable construction

mod basic_construction;
mod basic_local_search;
mod construction;
mod list_construction;
mod local_search;

pub use basic_construction::{BasicConstructionPhase, BasicConstructionPhaseBuilder};
pub use basic_local_search::{BasicLocalSearchPhase, BasicLocalSearchPhaseBuilder};
pub use construction::ConstructionPhaseFactory;
pub use list_construction::{ListConstructionPhase, ListConstructionPhaseBuilder};
pub use local_search::{HillClimbingFactory, KOptPhaseBuilder, LocalSearchPhaseFactory};
