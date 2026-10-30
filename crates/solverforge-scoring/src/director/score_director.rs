//! Zero-erasure score director for incremental scoring.
//!
//! This module provides `ScoreDirector` that uses monomorphized
//! constraint sets instead of trait-object-based scoring.

mod adapters;
mod analysis;
mod constructors;
mod descriptor;
mod incremental;
mod list_shadows;
mod weight_mode;

pub use descriptor::{DirectorDescriptor, ReplicaDescriptor};
pub use incremental::ScoreDirector;
pub use weight_mode::{requires_weighted_mode, UnweightedMode, WeightMode, WeightedMode};
