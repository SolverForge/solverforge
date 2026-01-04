//! Score types for representing solution quality
//!
//! Scores are used to compare solutions and guide the optimization process.
//! All score types are immutable and implement arithmetic operations.

mod traits;
mod simple;
mod hard_soft;
mod hard_medium_soft;
mod bendable;
mod hard_soft_decimal;

pub use traits::{ParseableScore, Score, ScoreParseError};
pub use simple::SimpleScore;
pub use hard_soft::HardSoftScore;
pub use hard_medium_soft::HardMediumSoftScore;
pub use bendable::BendableScore;
pub use hard_soft_decimal::HardSoftDecimalScore;

/// Score level representing different constraint priorities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScoreLevel {
    /// Hard constraints - must be satisfied for feasibility
    Hard,
    /// Medium constraints - secondary priority
    Medium,
    /// Soft constraints - optimization objectives
    Soft,
}
