/* Zero-erasure score director for incremental scoring.

This module provides `ScoreDirector` that uses monomorphized
constraint sets instead of trait-object-based scoring.
*/

mod adapters;
mod analysis;
mod descriptor;
mod incremental;
mod list_shadows;
mod weights;

pub use descriptor::{DirectorDescriptor, ReplicaDescriptor};
pub use incremental::ScoreDirector;
