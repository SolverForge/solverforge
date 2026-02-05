//! Score director implementations.
//!
//! The score director manages solution state and score calculation.
//!
//! # Score Director Types
//!
//! - [`SimpleScoreDirector`] - Full recalculation (baseline)
//! - [`TypedScoreDirector`] - Zero-erasure incremental scoring
//! - [`RecordingScoreDirector`] - Automatic undo tracking wrapper
//! - [`ShadowAwareScoreDirector`] - Shadow variable integration wrapper

mod factory;
mod simple;
mod traits;

pub mod recording;
pub mod shadow_aware;
pub mod typed;

#[cfg(test)]
mod tests;

pub use factory::ScoreDirectorFactory;
pub use recording::RecordingScoreDirector;
pub use shadow_aware::{ShadowAwareScoreDirector, ShadowVariableSupport, SolvableSolution};
pub use simple::SimpleScoreDirector;
pub use traits::ScoreDirector;
