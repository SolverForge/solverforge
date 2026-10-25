/* Zero-erasure score director for incremental scoring.

This module provides `ScoreDirector` that uses monomorphized
constraint sets instead of trait-object-based scoring.
*/

mod adapters;
mod descriptor;
mod incremental;
mod list_shadows;

pub use descriptor::{DirectorDescriptor, ReplicaDescriptor};
pub use incremental::ScoreDirector;
