/* Score director implementations.

The score director manages solution state and score calculation.

# Score Director Types

- [`ScoreDirector`] - Zero-erasure incremental scoring
*/

mod traits;

pub mod score_director;
pub mod shadow_aware;

#[cfg(test)]
mod tests;

pub use shadow_aware::SolvableSolution;
pub use traits::{Director, DirectorScoreState};
