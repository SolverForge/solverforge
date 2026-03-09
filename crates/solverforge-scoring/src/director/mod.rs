// Score director implementations.
//
// The score director manages solution state and score calculation.
//
// # Score Director Types
//
// - [`SimpleScoreDirector`] - Full recalculation (baseline)
// - [`TypedScoreDirector`] - Zero-erasure incremental scoring
// - [`RecordingScoreDirector`] - Automatic undo tracking wrapper

mod simple;
mod traits;

pub mod recording;
pub mod shadow_aware;
pub mod typed;

#[cfg(test)]
mod tests;

pub use recording::RecordingScoreDirector;
pub use shadow_aware::{ShadowVariableSupport, SolvableSolution};
pub use simple::SimpleScoreDirector;
pub use traits::ScoreDirector;
