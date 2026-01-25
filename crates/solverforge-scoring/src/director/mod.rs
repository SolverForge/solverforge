//! Score director implementations.
//!
//! The score director manages solution state and score calculation.
//!
//! # Score Director
//!
//! - [`ScoreDirector`] - Zero-erasure incremental scoring with shadow variable support

pub mod score_director;
pub mod shadow;

pub use score_director::ScoreDirector;
pub use shadow::{ShadowVariableSupport, SolvableSolution};
