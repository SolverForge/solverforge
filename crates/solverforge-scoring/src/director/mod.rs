// Score director implementations.
//
// The score director manages solution state and score calculation.
//
// # Score Director Types
//
// - [`ScoreDirector`] - Zero-erasure incremental scoring
// - [`RecordingDirector`] - Automatic undo tracking wrapper

mod traits;

pub mod recording;
pub mod score_director;
pub mod shadow_aware;

#[cfg(test)]
mod tests;

pub use recording::RecordingDirector;
pub use shadow_aware::{ShadowVariableSupport, SolvableSolution};
pub use traits::Director;
