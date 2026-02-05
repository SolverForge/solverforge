//! Score types for representing solution quality
//!
//! Scores are used to compare solutions and guide the optimization process.
//! All score types are immutable and implement arithmetic operations.

mod bendable;
mod hard_medium_soft;
mod hard_soft;
mod hard_soft_decimal;
mod simple;
mod traits;

#[cfg(test)]
mod tests;

pub use bendable::BendableScore;
pub use hard_medium_soft::HardMediumSoftScore;
pub use hard_soft::HardSoftScore;
pub use hard_soft_decimal::HardSoftDecimalScore;
pub use simple::SimpleScore;
pub use traits::{ParseableScore, Score, ScoreParseError};

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
