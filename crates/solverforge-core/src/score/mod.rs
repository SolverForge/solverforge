/* Score types for representing solution quality

Scores are used to compare solutions and guide the optimization process.
All score types are immutable and implement arithmetic operations.
*/

#[macro_use]
mod macros;
mod bendable;
#[cfg(feature = "serde")]
mod bendable_serde;
mod hard_medium_soft;
mod hard_soft;
mod hard_soft_decimal;
mod level;
mod scale;
mod soft;
mod traits;

#[cfg(test)]
mod tests;

pub use bendable::BendableScore;
pub use hard_medium_soft::HardMediumSoftScore;
pub use hard_soft::HardSoftScore;
pub use hard_soft_decimal::HardSoftDecimalScore;
pub use level::ScoreLevel;
pub(crate) use scale::{scale_level, scale_overflow};
pub use soft::SoftScore;
pub use traits::{ParseableScore, Score, ScoreParseError};
