/* Zero-erasure score director for incremental scoring.

This module provides `ScoreDirector` that uses monomorphized
constraint sets instead of trait-object-based scoring.
*/

mod adapters;
mod incremental;

pub use incremental::ScoreDirector;
