/* Typed score director for zero-erasure incremental scoring.

This module provides `ScoreDirector` that uses monomorphized
constraint sets instead of trait-object-based scoring.
*/

mod adapters;
mod typed;

pub use typed::ScoreDirector;
