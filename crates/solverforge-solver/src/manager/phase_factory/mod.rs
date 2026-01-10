//! Phase factory for creating phases from configuration.
//!
//! Phase factories create fresh phase instances for each solve, ensuring
//! clean state between solves. This is essential because phases maintain
//! internal state (like step counters, tabu lists, or temperature values)
//! that must be reset for each new solve.
//!
//! # Overview
//!
//! This module provides two main factories:
//!
//! - [`ConstructionPhaseFactory`]: Creates construction heuristic phases
//! - [`LocalSearchPhaseFactory`]: Creates local search phases
//!
//! # Usage Pattern
//!
//! Phase factories work with the zero-erasure architecture where all types
//! flow through generics. See the individual factory types for usage details.

mod construction;
mod list_construction;
mod local_search;

pub use construction::ConstructionPhaseFactory;
pub use list_construction::{ListConstructionPhase, ListConstructionPhaseBuilder};
pub use local_search::LocalSearchPhaseFactory;
