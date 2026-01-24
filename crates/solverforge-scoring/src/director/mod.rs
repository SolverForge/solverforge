//! Score director implementations.
//!
//! The score director manages solution state and score calculation.
//!
//! # Score Director
//!
//! - [`ScoreDirector`] - Zero-erasure incremental scoring with shadow variable support
//! - [`RecordingScoreDirector`] - Automatic undo tracking wrapper

pub mod recording;
pub mod score_director;
pub mod shadow;

#[cfg(test)]
mod recording_tests;

pub use recording::RecordingScoreDirector;
pub use score_director::ScoreDirector;
pub use shadow::{ShadowVariableSupport, SolvableSolution};
